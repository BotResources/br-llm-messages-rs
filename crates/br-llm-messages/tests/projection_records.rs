mod common;
use common::Raw as _;

use br_llm_messages::{
    ContentBlockKind, ErrorCode, IncomingEntry, Perspective, RecordSet, Turn, TurnEntryKind,
};
use common::histories::{history, reduced_rows};
use common::{aid, cvid, refused};
use serde_json::json;

#[test]
fn given_any_history_when_projected_from_the_full_history_or_from_reduced_rows_then_both_are_equal()
{
    let mut reduced_any = false;
    for seed in 0..80 {
        let conversation = history(seed);
        let full = RecordSet::from_conversation(&conversation).unwrap();
        for agent in 1..=4 {
            let expected = Perspective::of(aid(agent)).project(&conversation);
            let rows = reduced_rows(&full, agent, seed);
            reduced_any |= rows.blocks().len() < full.blocks().len();

            let actual = rows.into_projected_thread(cvid(900), aid(agent)).unwrap();

            assert_eq!(actual, expected, "seed {seed} agent {agent}");
        }
    }
    assert!(reduced_any, "the reduced rows must really omit rows");
}

#[test]
fn given_full_rows_of_every_agent_and_no_reduction_when_door_used_then_other_turns_are_refused() {
    let conversation = history(3);
    let full = RecordSet::from_conversation(&conversation).unwrap();

    let error = refused(full.into_projected_thread(cvid(900), aid(1)));

    assert_eq!(error.code(), ErrorCode::DisallowedKind);
}

fn other_turn_set() -> (RecordSet, br_llm_messages::TurnId) {
    let conversation = history(5);
    let full = RecordSet::from_conversation(&conversation).unwrap();
    let rows = reduced_rows(&full, 1, 5);
    let other = rows
        .turns()
        .iter()
        .find(|row| row.agent().is_some_and(|agent| agent != aid(1)))
        .expect("history 5 has another agent")
        .id();
    (rows, other)
}

#[test]
fn given_an_extra_entry_row_under_another_agents_turn_when_door_used_then_disallowed_kind() {
    let (rows, other) = other_turn_set();
    let source = common::turns::human_message(7001);
    let extra = br_llm_messages::EntryRecord::new(
        source.id(),
        other,
        TurnEntryKind::HumanMessage,
        br_llm_messages::RepresentationState::Complete,
        common::oj(json!({})),
    )
    .with_sender(br_llm_messages::ActorRef::human(common::hid(1)));
    let mut entries = rows.entries().to_vec();
    entries.push(extra);
    let rows = rows.with_entries(entries);

    let error = refused(rows.into_projected_thread(cvid(900), aid(1)));

    assert_eq!(error.code(), ErrorCode::DisallowedKind);
}

#[test]
fn given_a_non_text_block_row_under_another_agents_turn_when_door_used_then_disallowed_kind() {
    let (rows, other) = other_turn_set();
    let entry = rows
        .entries()
        .iter()
        .find(|row| row.turn() == other)
        .unwrap()
        .id();
    let thinking = br_llm_messages::BlockRecord::new(
        common::bid(7002),
        entry,
        ContentBlockKind::Thinking,
        br_llm_messages::RepresentationState::Complete,
        common::oj(json!({"text": "private"})),
    );
    let mut blocks = rows.blocks().to_vec();
    blocks.push(thinking);

    let error = refused(
        rows.with_blocks(blocks)
            .into_projected_thread(cvid(900), aid(1)),
    );

    assert_eq!(error.code(), ErrorCode::DisallowedKind);
}

#[test]
fn given_a_nested_block_row_under_another_agents_turn_when_door_used_then_disallowed_kind() {
    let (rows, _) = other_turn_set();
    let text = rows
        .blocks()
        .iter()
        .find(|row| row.kind() == ContentBlockKind::Text && row.parent().is_none())
        .map(|row| (row.id(), row.entry()))
        .unwrap();
    let nested = br_llm_messages::BlockRecord::new(
        common::bid(7003),
        text.1,
        ContentBlockKind::Text,
        br_llm_messages::RepresentationState::Complete,
        common::oj(json!({"text": "nested"})),
    )
    .with_parent(text.0);
    let mut blocks = rows.blocks().to_vec();
    blocks.push(nested);

    let error = refused(
        rows.with_blocks(blocks)
            .into_projected_thread(cvid(900), aid(1)),
    );

    assert!(matches!(
        error.code(),
        ErrorCode::DisallowedKind | ErrorCode::InvalidRecordStructure
    ));
}

#[test]
fn given_a_block_id_repeated_in_the_rows_when_door_used_then_reused_identity() {
    let (rows, _) = other_turn_set();
    let mut blocks = rows.blocks().to_vec();
    blocks.push(blocks[0].clone());

    let error = refused(
        rows.with_blocks(blocks)
            .into_projected_thread(cvid(900), aid(1)),
    );

    assert_eq!(error.code(), ErrorCode::ReusedIdentity);
}

#[test]
fn given_a_text_row_with_blank_text_when_door_used_then_invalid_content() {
    let (rows, _) = other_turn_set();
    let target = rows
        .blocks()
        .iter()
        .find(|row| row.kind() == ContentBlockKind::Text)
        .map(|row| row.id())
        .unwrap();
    let original = rows.blocks().iter().find(|row| row.id() == target).unwrap();
    let blank = br_llm_messages::BlockRecord::new(
        target,
        original.entry(),
        ContentBlockKind::Text,
        br_llm_messages::RepresentationState::Complete,
        common::oj(json!({"text": "   "})),
    );
    let blocks = rows
        .blocks()
        .iter()
        .map(|row| {
            if row.id() == target {
                blank.clone()
            } else {
                row.clone()
            }
        })
        .collect();
    let broken = rows.with_blocks(blocks);

    let error = refused(broken.into_projected_thread(cvid(900), aid(1)));

    assert_eq!(error.code(), ErrorCode::InvalidContent);
}

#[test]
fn given_an_agent_output_kind_in_a_record_row_when_decoded_then_disallowed_kind() {
    let (rows, _) = other_turn_set();
    let mut document = rows.to_value().unwrap().raw();
    document["entries"][0]["kind"] = json!("agent_output");
    let partial = {
        let mut other = document.clone();
        other["entries"][0]["kind"] = json!("partial_agent_output");
        other
    };

    for document in [document, partial] {
        let error = refused(RecordSet::from_value(common::oj(document)));
        assert_eq!(error.code(), ErrorCode::DisallowedKind);
    }
}

#[test]
fn given_a_projected_input_turn_when_decomposed_into_records_then_disallowed_kind() {
    let conversation = history(5);
    let thread = Perspective::of(aid(1)).project(&conversation);
    let projected: Vec<&Turn> = thread
        .turns()
        .iter()
        .filter(|turn| {
            matches!(turn, Turn::InputTurn(input)
                if input.entries().iter().any(|e| matches!(e, IncomingEntry::AgentOutput(_))))
        })
        .collect();
    assert!(!projected.is_empty());

    for turn in projected {
        let error = refused(RecordSet::from_turn(turn));
        assert_eq!(error.code(), ErrorCode::DisallowedKind);
        assert_eq!(error.location(), Some("entries[0]"));
    }
}
