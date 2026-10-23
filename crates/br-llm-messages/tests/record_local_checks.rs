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
        common::oj(payload),
    )
}

fn completed(payload: Value) -> EntryRecord {
    EntryRecord::new(
        eid(2),
        tid(1),
        TurnEntryKind::TaskCompleted,
        0,
        RepresentationState::Complete,
        common::oj(payload),
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

fn deep(levels: usize) -> Value {
    (0..levels).fold(json!(1), |inner, _| json!([inner]))
}

fn opaque_row(kind: ContentBlockKind, payload: Value) -> BlockRecord {
    BlockRecord::new(
        bid(3),
        eid(2),
        kind,
        0,
        RepresentationState::Complete,
        common::oj(payload),
    )
}

#[test]
fn given_an_opaque_value_beyond_the_depth_limit_when_a_row_is_checked_locally_then_it_is_refused() {
    let at_limit = [
        opaque_row(
            ContentBlockKind::StructuredContent,
            json!({"value": deep(64)}),
        ),
        opaque_row(
            ContentBlockKind::RuntimeToolCall,
            json!({"name": "t", "arguments": deep(64)}),
        ),
        opaque_row(
            ContentBlockKind::ProviderToolCall,
            json!({"arguments": deep(64)}),
        ),
    ];
    let beyond = [
        (
            opaque_row(
                ContentBlockKind::StructuredContent,
                json!({"value": deep(65)}),
            ),
            "payload.value",
        ),
        (
            opaque_row(
                ContentBlockKind::RuntimeToolCall,
                json!({"name": "t", "arguments": deep(65)}),
            ),
            "payload.arguments",
        ),
        (
            opaque_row(
                ContentBlockKind::ProviderToolCall,
                json!({"arguments": deep(65)}),
            ),
            "payload.arguments",
        ),
    ];

    assert!(at_limit.iter().all(|row| row.check_local().is_ok()));
    for (row, location) in beyond {
        let error = row.check_local().unwrap_err();
        assert_eq!(error.code(), ErrorCode::InvalidContent);
        assert_eq!(error.location(), Some(location));
    }
}
