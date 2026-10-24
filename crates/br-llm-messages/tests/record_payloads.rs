mod common;

use br_llm_messages::{
    ActorRef, BlockRecord, ContentBlockKind, EntryRecord, ErrorCode, RepresentationState,
    TurnEntryKind, TurnKind, TurnRecord,
};
use common::records::{rich_set, row, try_edit};
use common::turns::tid;
use common::{aid, author, bid, eid, hid};
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
fn given_an_agent_mail_row_without_author_when_checked_then_it_is_invalid_content() {
    let row = entry(
        TurnEntryKind::AgentMail,
        RepresentationState::Complete,
        json!({"source_conversation": common::cvid(1).to_string()}),
    )
    .with_sender(ActorRef::agent(aid(1)));

    assert_eq!(code(row.check_local()), ErrorCode::InvalidContent);
    assert!(row.with_author(author("A")).check_local().is_ok());
}

#[test]
fn given_an_author_on_kinds_that_forbid_it_when_checked_then_it_is_author_forbidden() {
    let human = entry(
        TurnEntryKind::HumanMessage,
        RepresentationState::Complete,
        json!({}),
    )
    .with_sender(ActorRef::human(hid(1)))
    .with_author(author("A"));
    let notice = entry(
        TurnEntryKind::RequestAccepted,
        RepresentationState::Complete,
        json!({"request": common::rid(1).to_string()}),
    )
    .with_author(author("A"));
    let input = TurnRecord::new(tid(1), TurnKind::InputTurn).with_author(author("A"));

    assert_eq!(code(human.check_local()), ErrorCode::AuthorForbidden);
    assert_eq!(code(notice.check_local()), ErrorCode::AuthorForbidden);
    assert_eq!(code(input.check_local()), ErrorCode::AuthorForbidden);
}

#[test]
fn given_an_ai_turn_row_without_agent_or_author_when_checked_then_it_is_invalid_content() {
    let no_author = TurnRecord::new(tid(1), TurnKind::AITurn).with_agent(aid(1));
    let no_agent = TurnRecord::new(tid(1), TurnKind::AITurn).with_author(author("A"));

    assert_eq!(code(no_author.check_local()), ErrorCode::InvalidContent);
    assert_eq!(code(no_agent.check_local()), ErrorCode::InvalidContent);
}

#[test]
fn given_a_wrong_sender_kind_when_checked_then_the_row_is_refused() {
    let human_by_agent = entry(
        TurnEntryKind::HumanMessage,
        RepresentationState::Complete,
        json!({}),
    )
    .with_sender(ActorRef::agent(aid(1)));
    let notice_with_sender = entry(
        TurnEntryKind::TaskCompleted,
        RepresentationState::Complete,
        json!({"task": common::kid(1).to_string()}),
    )
    .with_sender(ActorRef::human(hid(1)));
    let human_without = entry(
        TurnEntryKind::HumanMessage,
        RepresentationState::Complete,
        json!({}),
    );

    assert_eq!(
        code(human_by_agent.check_local()),
        ErrorCode::DisallowedKind
    );
    assert_eq!(
        code(notice_with_sender.check_local()),
        ErrorCode::DisallowedKind
    );
    assert_eq!(code(human_without.check_local()), ErrorCode::InvalidContent);
}

#[test]
fn given_a_payload_of_the_wrong_shape_when_checked_then_it_is_invalid_content() {
    let string_payload = entry(
        TurnEntryKind::QueuePosition,
        RepresentationState::Complete,
        json!("position 3"),
    );
    let array_block = block(
        ContentBlockKind::Text,
        RepresentationState::Complete,
        json!([1, 2]),
    );

    assert_eq!(
        code(string_payload.check_local()),
        ErrorCode::InvalidContent
    );
    assert_eq!(code(array_block.check_local()), ErrorCode::InvalidContent);
}

#[test]
fn given_an_unknown_payload_field_when_checked_then_it_is_invalid_content() {
    let extra = entry(
        TurnEntryKind::RequestAccepted,
        RepresentationState::Complete,
        json!({"request": common::rid(1).to_string(), "colour": "red"}),
    );
    let column_collision = block(
        ContentBlockKind::Text,
        RepresentationState::Complete,
        json!({"text": "x", "id": bid(9).to_string()}),
    );
    let author_collision = entry(
        TurnEntryKind::HumanMessage,
        RepresentationState::Complete,
        json!({"author": "A"}),
    )
    .with_sender(ActorRef::human(hid(1)));

    assert_eq!(code(extra.check_local()), ErrorCode::InvalidContent);
    assert_eq!(
        code(column_collision.check_local()),
        ErrorCode::InvalidContent
    );
    assert_eq!(
        code(author_collision.check_local()),
        ErrorCode::InvalidContent
    );
}

#[test]
fn given_a_payload_missing_a_required_field_when_checked_then_it_is_invalid_content() {
    let queue = entry(
        TurnEntryKind::QueuePosition,
        RepresentationState::Complete,
        json!({"request": common::rid(1).to_string()}),
    );
    let text = block(
        ContentBlockKind::Text,
        RepresentationState::Complete,
        json!({}),
    );

    assert_eq!(code(queue.check_local()), ErrorCode::InvalidContent);
    assert_eq!(code(text.check_local()), ErrorCode::InvalidContent);
}

#[test]
fn given_a_payload_that_breaks_a_field_rule_when_checked_then_the_field_rule_code_is_reported() {
    let zero = entry(
        TurnEntryKind::QueuePosition,
        RepresentationState::Complete,
        json!({"request": common::rid(1).to_string(), "position": 0}),
    );
    let blank = block(
        ContentBlockKind::Text,
        RepresentationState::Complete,
        json!({"text": "   "}),
    );

    assert_eq!(code(zero.check_local()), ErrorCode::InvalidContent);
    assert_eq!(code(blank.check_local()), ErrorCode::InvalidContent);
}

#[test]
fn given_the_call_column_when_checked_then_it_is_required_on_results_only() {
    let result_payload = json!({"tool_name": "lookup", "is_error": false});
    let missing = block(
        ContentBlockKind::RuntimeToolResult,
        RepresentationState::Complete,
        result_payload.clone(),
    );
    let present = missing.clone().with_call(bid(7));
    let on_text = block(
        ContentBlockKind::Text,
        RepresentationState::Complete,
        json!({"text": "x"}),
    )
    .with_call(bid(7));

    assert_eq!(code(missing.check_local()), ErrorCode::InvalidContent);
    assert!(present.check_local().is_ok());
    assert_eq!(code(on_text.check_local()), ErrorCode::DisallowedKind);
}

#[test]
fn given_a_json_document_with_a_bad_row_when_loaded_then_the_row_code_is_reported() {
    let set = rich_set();
    let id = set.blocks()[0].id().as_uuid();

    let unknown_field = try_edit(&set, |document| {
        row(document, "blocks", id)["payload"]["colour"] = json!("red");
    });
    let wrong_column = try_edit(&set, |document| {
        row(document, "blocks", id)["call"] = json!(bid(5).to_string());
    });

    assert_eq!(code(unknown_field), ErrorCode::InvalidContent);
    assert_eq!(code(wrong_column), ErrorCode::DisallowedKind);
}
