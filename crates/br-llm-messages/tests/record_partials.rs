mod common;

use br_llm_messages::{
    ActorRef, BlockRecord, ContentBlockKind, EntryRecord, ErrorCode, RepresentationState,
    TurnEntryKind,
};
use common::turns::tid;
use common::{bid, eid, hid};
use serde_json::{Value, json};

fn entry(kind: TurnEntryKind, state: RepresentationState, payload: Value) -> EntryRecord {
    EntryRecord::new(eid(2), tid(1), kind, 0, state, payload)
}

fn block(kind: ContentBlockKind, state: RepresentationState, payload: Value) -> BlockRecord {
    BlockRecord::new(bid(3), eid(2), kind, 0, state, payload)
}

fn code<T>(result: Result<T, br_llm_messages::Error>) -> ErrorCode {
    result.err().expect("must be refused").code()
}

#[test]
fn given_a_standalone_partial_text_row_when_checked_locally_then_draft_rules_apply() {
    let blank_producing = block(
        ContentBlockKind::Text,
        RepresentationState::Producing,
        json!({"text": "  "}),
    );
    let raw_call = block(
        ContentBlockKind::RuntimeToolCall,
        RepresentationState::Interrupted,
        json!({"raw_arguments": "{\"q\": \"ru"}),
    );

    assert!(blank_producing.check_local().is_ok());
    assert!(raw_call.check_local().is_ok());
}

#[test]
fn given_a_partial_row_of_a_kind_that_cannot_be_partial_when_checked_then_it_is_refused() {
    for kind in [
        ContentBlockKind::Image,
        ContentBlockKind::RedactedThinking,
        ContentBlockKind::ProviderToolResult,
        ContentBlockKind::RuntimeToolResult,
    ] {
        let row = block(kind, RepresentationState::Producing, json!({})).with_call(bid(1));

        assert_eq!(code(row.check_local()), ErrorCode::InvalidRecord);
    }
}

#[test]
fn given_discarded_rows_when_checked_then_only_empty_text_rows_are_valid() {
    let good = block(
        ContentBlockKind::Text,
        RepresentationState::Discarded,
        json!({}),
    );
    let wrong_kind = block(
        ContentBlockKind::Thinking,
        RepresentationState::Discarded,
        json!({}),
    );
    let with_payload = block(
        ContentBlockKind::Text,
        RepresentationState::Discarded,
        json!({"text": "x"}),
    );

    assert!(good.check_local().is_ok());
    assert_eq!(code(wrong_kind.check_local()), ErrorCode::InvalidRecord);
    assert_eq!(code(with_payload.check_local()), ErrorCode::InvalidRecord);
}

#[test]
fn given_a_response_header_row_when_checked_locally_then_interruption_depends_on_state() {
    let producing = entry(
        TurnEntryKind::ModelResponse,
        RepresentationState::Producing,
        json!({"model": "m"}),
    );
    let producing_with_reason = entry(
        TurnEntryKind::ModelResponse,
        RepresentationState::Producing,
        json!({"interruption": {"type": "cancelled"}}),
    );
    let interrupted_without_reason = entry(
        TurnEntryKind::ModelResponse,
        RepresentationState::Interrupted,
        json!({}),
    );
    let interrupted = entry(
        TurnEntryKind::ModelResponse,
        RepresentationState::Interrupted,
        json!({"interruption": {"type": "truncated"}}),
    );

    assert!(producing.check_local().is_ok());
    assert!(interrupted.check_local().is_ok());
    assert_eq!(
        code(producing_with_reason.check_local()),
        ErrorCode::UnsupportedField
    );
    assert_eq!(
        code(interrupted_without_reason.check_local()),
        ErrorCode::MissingContent
    );
}

#[test]
fn given_a_non_response_entry_in_a_partial_state_when_checked_then_it_is_refused() {
    let producing_human = entry(
        TurnEntryKind::HumanMessage,
        RepresentationState::Producing,
        json!({}),
    )
    .with_sender(ActorRef::human(hid(1)));
    let discarded = entry(
        TurnEntryKind::ModelResponse,
        RepresentationState::Discarded,
        json!({}),
    );

    assert_eq!(
        code(producing_human.check_local()),
        ErrorCode::InvalidRecord
    );
    assert_eq!(code(discarded.check_local()), ErrorCode::InvalidRecord);
}
