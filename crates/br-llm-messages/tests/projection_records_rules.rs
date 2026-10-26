mod common;
mod projection_support;

use br_llm_messages::{
    BlockRecord, Citation, CitationSource, Conversation, EntryRecord, ErrorCode, ImageBlock,
    Perspective, PlainBlock, RecordSet, RepresentationState, ResponseEnding, StructuredBlock,
    TextBlock, TextRange, Turn, TurnEntryKind,
};
use common::turns::{input_turn, response};
use common::{aid, at, bid, cid, cvid, envelope, oj, png, refused};
use projection_support::*;
use serde_json::json;

fn rows_of(turns: &[Turn]) -> RecordSet {
    let conversation = Conversation::from_parts(cvid(900), turns.to_vec()).unwrap();
    RecordSet::from_conversation(&conversation).unwrap()
}

fn foreign_only(full: &RecordSet) -> RecordSet {
    let entries = full
        .entries()
        .iter()
        .map(|row| match row.kind() {
            TurnEntryKind::ModelResponse => {
                EntryRecord::new(row.id(), row.turn(), row.kind(), row.state(), oj(json!({})))
            }
            _ => row.clone(),
        })
        .collect();
    RecordSet::new()
        .with_turns(full.turns().to_vec())
        .with_entries(entries)
        .with_blocks(full.blocks().to_vec())
}

fn decorated_history() -> Vec<Turn> {
    let citation = Citation::new(cid(50), CitationSource::new("doc").unwrap())
        .with_range(TextRange::new(0, 1).unwrap());
    let text = TextBlock::new(bid(12), "cited")
        .unwrap()
        .with_citations(vec![citation])
        .unwrap()
        .with_item_ref(br_llm_messages::ItemRef::new("msg_1").unwrap())
        .with_replay(envelope(json!({"k": 1})))
        .with_observed_at(at(1));
    let image: ImageBlock = png(13).with_observed_at(at(2));
    let structured = StructuredBlock::new(bid(14), oj(json!({"a": 1})))
        .unwrap()
        .with_observed_at(at(3));
    vec![
        input_turn(1, 2).into(),
        ai(
            10,
            2,
            vec![response(
                11,
                vec![text.into(), image.into(), structured.into()],
                ResponseEnding::finished(),
            )],
        ),
    ]
}

#[test]
fn given_kept_block_rows_with_citations_replay_item_ref_and_observed_at_when_projected_from_records_then_they_are_dropped_silently()
 {
    let turns = decorated_history();
    let expected = Perspective::of(aid(1))
        .project_turns(cvid(900), &turns)
        .unwrap();

    let actual = rows_with(&[], &turns, 1)
        .into_projected_thread(cvid(900), aid(1))
        .unwrap();

    assert_eq!(actual, expected);
    let Turn::InputTurn(input) = &actual.turns()[1] else {
        panic!("input turn expected");
    };
    let br_llm_messages::IncomingEntry::AgentOutput(output) = &input.entries()[0] else {
        panic!("output expected");
    };
    assert_eq!(output.content().len(), 3);
    assert!(matches!(&output.content()[0], PlainBlock::Text(text)
        if text.citations().is_empty() && text.replay().is_none() && text.item_ref().is_none()
            && text.observed_at().is_none()));
    assert!(output.content().iter().all(|b| b.observed_at().is_none()));
}

#[test]
fn given_kept_response_rows_without_any_payload_when_built_in_memory_then_they_project_but_never_load_from_json()
 {
    let turns = decorated_history();
    let rows = foreign_only(&rows_with(&[], &turns, 1));
    let expected = Perspective::of(aid(1))
        .project_turns(cvid(900), &turns)
        .unwrap();

    let loaded = refused(RecordSet::from_json(&rows.to_json().unwrap()));
    let actual = rows.into_projected_thread(cvid(900), aid(1)).unwrap();

    assert_eq!(loaded.code(), ErrorCode::InvalidContent);
    assert_eq!(actual, expected);
}

#[test]
fn given_a_complete_response_row_without_ending_when_rebuilt_as_a_conversation_then_it_is_still_refused()
 {
    let turns = decorated_history();
    let rows = foreign_only(&rows_of(&turns));

    let error = refused(rows.into_conversation(cvid(900)));

    assert_eq!(error.code(), ErrorCode::InvalidContent);
}

#[test]
fn given_every_row_of_an_interrupted_foreign_response_whatever_kind_or_nesting_when_projected_then_they_are_ignored()
 {
    let turns = history();
    let draft = interrupted(30, 32, 33);
    let snapshot = draft.to_snapshot().unwrap();
    let nested = BlockRecord::new(
        bid(90),
        snapshot.entry().id(),
        br_llm_messages::ContentBlockKind::Text,
        RepresentationState::Interrupted,
        oj(json!({"text": "child"})),
    )
    .with_parent(bid(33));
    let mut rows = rows_with(&[], &turns, 1);
    let mut entries = rows.entries().to_vec();
    entries.push(snapshot.entry().clone());
    let mut blocks = rows.blocks().to_vec();
    blocks.extend(snapshot.blocks().iter().cloned());
    blocks.push(nested);
    assert!(
        snapshot.blocks().iter().any(|row| !row.kind().is_plain()),
        "the snapshot must carry a non-text block"
    );
    rows = rows.with_entries(entries).with_blocks(blocks);

    let actual = rows.into_projected_thread(cvid(900), aid(1)).unwrap();

    assert_eq!(
        actual,
        Perspective::of(aid(1))
            .project_turns(cvid(900), &turns)
            .unwrap()
    );
}

#[test]
fn given_an_interrupted_response_row_in_an_input_turn_when_projected_then_disallowed_kind() {
    let turns = history();
    let draft = interrupted(30, 32, 33);
    let snapshot = draft.to_snapshot().unwrap();
    let stray = EntryRecord::new(
        snapshot.entry().id(),
        tid_of_input(&turns),
        TurnEntryKind::ModelResponse,
        RepresentationState::Interrupted,
        snapshot.entry().payload().clone(),
    );
    let rows = rows_with(&[], &[input_turn(1, 2).into()], 1);
    let rows = rows
        .clone()
        .with_entries(vec![rows.entries()[0].clone(), stray]);

    let error = refused(rows.into_projected_thread(cvid(900), aid(1)));

    assert_eq!(error.code(), ErrorCode::DisallowedKind);
}

fn tid_of_input(_: &[Turn]) -> br_llm_messages::TurnId {
    common::turns::tid(1)
}

#[test]
fn given_an_interrupted_incoming_row_under_a_foreign_turn_when_projected_then_disallowed_kind() {
    let turns = vec![ai(10, 2, vec![said(11, 12, "x"), common::turns::human(13)])];
    let full = rows_of(&turns);
    let broken: Vec<EntryRecord> = full
        .entries()
        .iter()
        .map(|row| {
            if row.kind() == TurnEntryKind::HumanMessage {
                let mut copy = EntryRecord::new(
                    row.id(),
                    row.turn(),
                    row.kind(),
                    RepresentationState::Interrupted,
                    row.payload().clone(),
                );
                if let Some(sender) = row.sender() {
                    copy = copy.with_sender(sender.clone());
                }
                copy
            } else {
                row.clone()
            }
        })
        .collect();
    let rows = rows_with(&[], &turns, 1).with_entries(broken);

    let error = refused(rows.into_projected_thread(cvid(900), aid(1)));

    assert_eq!(error.code(), ErrorCode::DisallowedKind);
}

#[test]
fn given_a_response_row_without_ending_in_the_targets_own_turn_when_projected_then_invalid_content()
{
    let turns = decorated_history();
    let rows = foreign_only(&rows_with(&[], &turns, 1));

    let error = refused(rows.into_projected_thread(cvid(900), aid(2)));

    assert_eq!(error.code(), ErrorCode::InvalidContent);
}

fn with_interrupted_block(
    rows: &RecordSet,
    target: br_llm_messages::BlockId,
) -> (RecordSet, usize) {
    let index = rows
        .blocks()
        .iter()
        .position(|row| row.id() == target)
        .expect("the block row exists");
    let blocks = rows
        .blocks()
        .iter()
        .map(|row| {
            if row.id() == target {
                BlockRecord::new(
                    row.id(),
                    row.entry(),
                    row.kind(),
                    RepresentationState::Interrupted,
                    row.payload().clone(),
                )
            } else {
                row.clone()
            }
        })
        .collect();
    (rows.clone().with_blocks(blocks), index)
}

#[test]
fn given_an_interrupted_block_row_under_a_complete_or_producing_foreign_response_when_projected_then_disallowed_kind()
 {
    let turns = history();
    let complete = rows_with(&[], &turns, 1);
    let producing_rows = rows_with(&[producing(30, 32, 33, "partial")], &turns, 1);

    for (rows, block) in [(complete, bid(12)), (producing_rows, bid(34))] {
        let (rows, index) = with_interrupted_block(&rows, block);

        let error = refused(rows.into_projected_thread(cvid(900), aid(1)));

        assert_eq!(error.code(), ErrorCode::DisallowedKind);
        assert_eq!(
            error.location(),
            Some(format!("blocks[{index}].state").as_str())
        );
    }
}
