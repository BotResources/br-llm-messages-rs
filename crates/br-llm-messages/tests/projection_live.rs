mod common;

use br_llm_messages::{
    AITurn, BlockStart, ContentBlockKind, Conversation, DeltaOp, ErrorCode, Finalization,
    IncomingEntry, InterruptionReason, Perspective, RecordSet, ResponseDelta, ResponseDraft,
    ResponseEnding, ResponseMetadata, TextBlock, Turn,
};
use common::turns::{human, response, tid};
use common::{aid, author, bid, cvid, eid, refused};

fn started(turn: u64, entry: u64) -> ResponseDraft {
    let mut draft = ResponseDraft::new(tid(turn), eid(entry));
    feed(
        &mut draft,
        turn,
        entry,
        vec![DeltaOp::response_started(ResponseMetadata::new())],
    );
    draft
}

fn feed(draft: &mut ResponseDraft, turn: u64, entry: u64, ops: Vec<DeltaOp>) {
    for op in ops {
        draft
            .apply(&ResponseDelta::new(tid(turn), eid(entry), op))
            .unwrap();
    }
}

fn producing(turn: u64, entry: u64, block: u64, text: &str) -> ResponseDraft {
    let mut draft = started(turn, entry);
    feed(
        &mut draft,
        turn,
        entry,
        vec![
            DeltaOp::block_started(bid(block), BlockStart::thinking()),
            DeltaOp::thinking_appended(bid(block), "private plan"),
            DeltaOp::block_finalized(bid(block), Finalization::accumulated()),
            DeltaOp::block_started(bid(block + 1), BlockStart::text()),
            DeltaOp::text_appended(bid(block + 1), text),
        ],
    );
    draft
}

fn interrupted(turn: u64, entry: u64, block: u64) -> ResponseDraft {
    let mut draft = producing(turn, entry, block, "cut short");
    feed(
        &mut draft,
        turn,
        entry,
        vec![DeltaOp::response_interrupted(
            InterruptionReason::cancelled(),
        )],
    );
    draft
}

fn ai(id: u64, agent: u64, entries: Vec<br_llm_messages::TurnEntry>) -> Turn {
    AITurn::from_parts(tid(id), aid(agent), author("Claude"), entries)
        .unwrap()
        .into()
}

fn said(entry: u64, block: u64, text: &str) -> br_llm_messages::TurnEntry {
    response(
        entry,
        vec![TextBlock::new(bid(block), text).unwrap().into()],
        ResponseEnding::finished(),
    )
}

fn kinds(turn: &Turn) -> Vec<&'static str> {
    let Turn::InputTurn(input) = turn else {
        panic!("input turn expected");
    };
    input
        .entries()
        .iter()
        .map(|entry| match entry {
            IncomingEntry::AgentOutput(_) => "output",
            IncomingEntry::PartialAgentOutput(_) => "partial",
            _ => "recorded",
        })
        .collect()
}

fn history() -> Vec<Turn> {
    vec![
        ai(10, 2, vec![said(11, 12, "complete")]),
        ai(30, 2, vec![human(31)]),
        ai(50, 2, vec![human(51)]),
    ]
}

#[test]
fn given_complete_producing_and_interrupted_responses_when_projected_then_output_partial_and_nothing()
 {
    let drafts = [producing(30, 32, 33, "so far"), interrupted(50, 52, 53)];

    let thread = Perspective::of(aid(1))
        .project_live(cvid(900), &history(), &drafts)
        .unwrap();

    assert_eq!(thread.len(), 2);
    assert_eq!(kinds(&thread.turns()[0]), ["output"]);
    assert_eq!(thread.turns()[1].id(), tid(30));
    assert_eq!(kinds(&thread.turns()[1]), ["partial"]);
}

#[test]
fn given_a_producing_response_after_a_completed_one_when_projected_then_the_partial_is_last() {
    let turns = vec![ai(10, 2, vec![said(11, 12, "complete")])];
    let draft = producing(10, 20, 21, "so far");

    let thread = Perspective::of(aid(1))
        .project_live(cvid(900), &turns, &[draft])
        .unwrap();

    assert_eq!(kinds(&thread.turns()[0]), ["output", "partial"]);
    let Turn::InputTurn(input) = &thread.turns()[0] else {
        panic!("input turn expected");
    };
    let IncomingEntry::PartialAgentOutput(partial) = &input.entries()[1] else {
        panic!("partial expected");
    };
    assert_eq!(partial.id(), eid(20));
    assert_eq!(partial.agent(), aid(2));
    assert_eq!(partial.content().len(), 1);
    assert_eq!(partial.content()[0].id(), bid(22));
    assert!(!format!("{thread:?}").contains("private plan"));
}

#[test]
fn given_a_producing_response_when_text_arrives_afterwards_then_the_built_thread_is_unchanged() {
    let turns = vec![ai(30, 2, vec![human(31)])];
    let mut draft = producing(30, 32, 33, "so far");
    let thread = Perspective::of(aid(1))
        .project_live(cvid(900), &turns, std::slice::from_ref(&draft))
        .unwrap();
    let before = thread.clone();

    feed(
        &mut draft,
        30,
        32,
        vec![DeltaOp::text_appended(bid(34), " and more")],
    );

    assert_eq!(thread, before);
    let again = Perspective::of(aid(1))
        .project_live(cvid(900), &turns, &[draft])
        .unwrap();
    assert_ne!(again, thread);
}

#[test]
fn given_a_producing_response_without_text_yet_when_projected_then_no_entry_is_made() {
    let mut draft = started(30, 32);
    feed(
        &mut draft,
        30,
        32,
        vec![DeltaOp::block_started(bid(33), BlockStart::text())],
    );

    let thread = Perspective::of(aid(1))
        .project_live(cvid(900), &[ai(30, 2, vec![human(31)])], &[draft])
        .unwrap();

    assert!(thread.is_empty());
}

#[test]
fn given_a_draft_of_an_unknown_turn_or_of_the_target_agent_when_projected_then_it_is_refused() {
    let turns = history();

    let unknown = refused(Perspective::of(aid(1)).project_live(
        cvid(900),
        &turns,
        &[producing(99, 90, 91, "x")],
    ));
    let own = refused(Perspective::of(aid(2)).project_live(
        cvid(900),
        &turns,
        &[producing(30, 32, 33, "x")],
    ));

    assert_eq!(unknown.code(), ErrorCode::UnknownReference);
    assert_eq!(own.code(), ErrorCode::DisallowedKind);
}

#[test]
fn given_two_producing_responses_in_one_turn_when_projected_then_invalid_record_structure() {
    let drafts = [producing(30, 32, 33, "a"), producing(30, 42, 43, "b")];

    let error = refused(Perspective::of(aid(1)).project_live(cvid(900), &history(), &drafts));

    assert_eq!(error.code(), ErrorCode::InvalidRecordStructure);
}

fn rows_with(drafts: &[ResponseDraft], turns: &[Turn], agent: u64) -> RecordSet {
    let conversation = Conversation::from_parts(cvid(900), turns.to_vec()).unwrap();
    let full = RecordSet::from_conversation(&conversation).unwrap();
    let mut entries = Vec::new();
    let mut blocks = Vec::new();
    for row in full.entries() {
        let owner = full.turns().iter().find(|t| t.id() == row.turn()).unwrap();
        let kept = owner.agent() == Some(aid(agent))
            || owner.agent().is_none()
            || row.kind() == br_llm_messages::TurnEntryKind::ModelResponse;
        if kept {
            entries.push(row.clone());
        }
    }
    for row in full.blocks() {
        if entries.iter().any(|e| e.id() == row.entry()) {
            blocks.push(row.clone());
        }
    }
    let blocks_of_owner = |row: &br_llm_messages::BlockRecord| {
        let entry = entries.iter().find(|e| e.id() == row.entry()).unwrap();
        let owner = full
            .turns()
            .iter()
            .find(|t| t.id() == entry.turn())
            .unwrap();
        owner.agent() == Some(aid(agent))
            || owner.agent().is_none()
            || (row.kind() == ContentBlockKind::Text && row.parent().is_none())
    };
    blocks.retain(blocks_of_owner);
    for draft in drafts {
        let snapshot = draft.to_snapshot().unwrap();
        entries.push(snapshot.entry().clone());
        blocks.extend(
            snapshot
                .blocks()
                .iter()
                .filter(|row| row.kind() == ContentBlockKind::Text && row.parent().is_none())
                .cloned(),
        );
    }
    RecordSet::new()
        .with_turns(full.turns().to_vec())
        .with_entries(entries)
        .with_blocks(blocks)
}

#[test]
fn given_producing_and_interrupted_rows_when_projected_from_records_then_it_equals_the_live_path() {
    let turns = history();
    let drafts = [producing(30, 32, 33, "so far"), interrupted(50, 52, 53)];
    let expected = Perspective::of(aid(1))
        .project_live(cvid(900), &turns, &drafts)
        .unwrap();

    let rows = rows_with(&drafts, &turns, 1);
    let actual = rows.into_projected_thread(cvid(900), aid(1)).unwrap();

    assert_eq!(actual, expected);
    assert_eq!(kinds(&actual.turns()[1]), ["partial"]);
}

#[test]
fn given_a_producing_text_row_with_blank_text_when_projected_from_records_then_invalid_content() {
    let turns = history();
    let drafts = [producing(30, 32, 33, " ")];

    let rows = rows_with(&drafts, &turns, 1);
    let error = refused(rows.into_projected_thread(cvid(900), aid(1)));

    assert_eq!(error.code(), ErrorCode::InvalidContent);
}

#[test]
fn given_a_producing_row_under_the_target_agents_turn_when_projected_from_records_then_disallowed_kind()
 {
    let turns = history();
    let drafts = [producing(30, 32, 33, "so far")];

    let rows = rows_with(&drafts, &turns, 2);
    let error = refused(rows.into_projected_thread(cvid(900), aid(2)));

    assert_eq!(error.code(), ErrorCode::DisallowedKind);
}
