mod common;

use br_llm_messages::{
    ActorRef, BlockRecord, ContentBlockKind, EntryRecord, ErrorCode, RepresentationState,
    TurnEntryKind,
};
use common::turns::tid;
use common::{bid, eid, hid};
use serde_json::{Value, json};

fn entry(kind: TurnEntryKind, state: RepresentationState, payload: Value) -> EntryRecord {
    EntryRecord::new(eid(2), tid(1), kind, state, common::oj(payload))
}

fn block(kind: ContentBlockKind, state: RepresentationState, payload: Value) -> BlockRecord {
    BlockRecord::new(bid(3), eid(2), kind, state, common::oj(payload))
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
    for kind in [ContentBlockKind::Image, ContentBlockKind::RuntimeToolResult] {
        let row = block(kind, RepresentationState::Producing, json!({})).with_call(bid(1));

        assert_eq!(code(row.check_local()), ErrorCode::DisallowedKind);
    }
}

#[test]
fn given_partial_rows_of_the_replay_only_kinds_when_checked_then_they_are_accepted() {
    let redacted = block(
        ContentBlockKind::RedactedThinking,
        RepresentationState::Producing,
        json!({"replay": {"format": "anthropic.redacted", "payload": {"type": "text", "value": "x"}}}),
    );
    let provider_result = block(
        ContentBlockKind::ProviderToolResult,
        RepresentationState::Producing,
        json!({}),
    )
    .with_call(bid(9));

    assert!(redacted.check_local().is_ok());
    assert!(provider_result.check_local().is_ok());
}

#[test]
fn given_a_provisional_argument_that_is_not_an_object_when_a_call_row_is_checked_then_it_is_refused()
 {
    let row = |value: Value| {
        block(
            ContentBlockKind::RuntimeToolCall,
            RepresentationState::Producing,
            json!({"raw_arguments": "", "provisional_arguments": value}),
        )
    };

    assert!(row(json!({"q": 1})).check_local().is_ok());
    assert_eq!(
        code(row(json!([1])).check_local()),
        ErrorCode::InvalidContent
    );
}

#[test]
fn given_the_discarded_state_when_parsed_then_it_is_not_a_representation_state() {
    assert_eq!(
        "discarded"
            .parse::<RepresentationState>()
            .unwrap_err()
            .code(),
        ErrorCode::VariantMismatch
    );
    assert_eq!(RepresentationState::ALL.len(), 3);
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
        json!({"interruption": {"kind": "cancelled"}}),
    );
    let interrupted_without_reason = entry(
        TurnEntryKind::ModelResponse,
        RepresentationState::Interrupted,
        json!({}),
    );
    let interrupted = entry(
        TurnEntryKind::ModelResponse,
        RepresentationState::Interrupted,
        json!({"interruption": {"kind": "truncated", "detail": "cut"}}),
    );

    assert!(producing.check_local().is_ok());
    assert!(interrupted.check_local().is_ok());
    assert_eq!(
        code(producing_with_reason.check_local()),
        ErrorCode::InvalidContent
    );
    assert_eq!(
        code(interrupted_without_reason.check_local()),
        ErrorCode::InvalidContent
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
    assert_eq!(
        code(producing_human.check_local()),
        ErrorCode::DisallowedKind
    );
}
