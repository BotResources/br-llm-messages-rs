mod common;

use br_llm_messages::{
    BlockRecord, ContentBlockKind, EntryRecord, ErrorCode, RepresentationState, TurnEntryKind,
};
use common::turns::tid;
use common::{bid, eid, kid};
use serde_json::{Value, json};

fn text_row(payload: Value) -> BlockRecord {
    BlockRecord::new(
        bid(3),
        eid(2),
        ContentBlockKind::Text,
        0,
        RepresentationState::Complete,
        payload,
    )
}

fn completed(payload: Value) -> EntryRecord {
    EntryRecord::new(
        eid(2),
        tid(1),
        TurnEntryKind::TaskCompleted,
        0,
        RepresentationState::Complete,
        payload,
    )
}

#[test]
fn given_a_null_for_a_non_optional_list_when_checked_locally_then_it_is_refused() {
    let null = text_row(json!({"text": "hi", "citations": null}));
    let absent = text_row(json!({"text": "hi"}));
    let empty = text_row(json!({"text": "hi", "citations": []}));

    assert_eq!(
        null.check_local().unwrap_err().code(),
        ErrorCode::InvalidContent
    );
    assert!(absent.check_local().is_ok());
    assert!(empty.check_local().is_ok());
}

#[test]
fn given_a_null_for_an_optional_scalar_when_checked_locally_then_it_is_refused() {
    let null = text_row(json!({"text": "hi", "replay": null}));
    let absent = text_row(json!({"text": "hi"}));

    assert_eq!(
        null.check_local().unwrap_err().code(),
        ErrorCode::InvalidContent
    );
    assert!(absent.check_local().is_ok());
}

#[test]
fn given_a_blank_task_summary_when_checked_locally_then_invalid_content_like_the_entry() {
    let blank = completed(json!({"task": kid(1).to_string(), "summary": "   "}));
    let named = completed(json!({"task": kid(1).to_string(), "summary": "done"}));
    let none = completed(json!({"task": kid(1).to_string()}));

    assert_eq!(
        blank.check_local().unwrap_err().code(),
        ErrorCode::InvalidContent
    );
    assert!(named.check_local().is_ok());
    assert!(none.check_local().is_ok());
}
