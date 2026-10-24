mod common;

use br_llm_messages::{
    ActorRef, BlockRecord, ContentBlockKind, EntryRecord, ErrorCode, RepresentationState,
    TurnEntryKind,
};
use common::turns::tid;
use common::{bid, eid, kid};
use serde_json::{Value, json};

fn text_row(payload: Value) -> BlockRecord {
    BlockRecord::new(
        bid(3),
        eid(2),
        ContentBlockKind::Text,
        RepresentationState::Complete,
        common::oj(payload),
    )
}

fn completed(payload: Value) -> EntryRecord {
    EntryRecord::new(
        eid(2),
        tid(1),
        TurnEntryKind::TaskCompleted,
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
            json!({"name": "t", "arguments": {"k": deep(63)}}),
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
                json!({"name": "t", "arguments": {"k": deep(64)}}),
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

#[test]
fn given_a_runtime_call_row_with_non_object_arguments_when_checked_locally_then_invalid_content() {
    for arguments in [json!(null), json!([1]), json!("x")] {
        let row = opaque_row(
            ContentBlockKind::RuntimeToolCall,
            json!({"name": "t", "arguments": arguments}),
        );

        let error = row.check_local().unwrap_err();

        assert_eq!(error.code(), ErrorCode::InvalidContent);
        assert_eq!(error.location(), Some("payload.arguments"));
    }
}

#[test]
fn given_a_thinking_row_with_empty_text_when_checked_locally_then_replay_decides_but_only_if_complete()
 {
    let sealed = json!({"format": "a.b", "payload": {"type": "text", "value": "s"}});
    let row = |state: RepresentationState, payload: Value| {
        BlockRecord::new(
            bid(3),
            eid(2),
            ContentBlockKind::Thinking,
            state,
            common::oj(payload),
        )
    };

    let bare = row(RepresentationState::Complete, json!({"text": ""}));
    let replayed = row(
        RepresentationState::Complete,
        json!({"text": "", "replay": sealed}),
    );
    let producing = row(RepresentationState::Producing, json!({"text": ""}));

    let error = bare.check_local().unwrap_err();
    assert_eq!(error.code(), ErrorCode::InvalidContent);
    assert_eq!(error.location(), Some("payload.text"));
    assert!(replayed.check_local().is_ok());
    assert!(producing.check_local().is_ok());
}

fn row_of(kind: TurnEntryKind, payload: Value) -> EntryRecord {
    EntryRecord::new(
        eid(2),
        tid(1),
        kind,
        RepresentationState::Complete,
        common::oj(payload),
    )
}

#[test]
fn given_a_task_failed_payload_when_checked_locally_then_a_non_blank_reason_is_required() {
    let task = kid(1).to_string();
    let reason = |payload: Value| row_of(TurnEntryKind::TaskFailed, payload).check_local();

    assert!(reason(json!({"task": task, "reason": "boom"})).is_ok());
    for payload in [
        json!({"task": task, "reason": "  "}),
        json!({"task": task}),
        json!({"task": task, "failure": {"code": "boom"}}),
    ] {
        assert_eq!(
            reason(payload).unwrap_err().code(),
            ErrorCode::InvalidContent
        );
    }
}

#[test]
fn given_a_closure_payload_when_checked_locally_then_failed_requires_a_detail() {
    let closure =
        |reason: Value| row_of(TurnEntryKind::TurnClosure, json!({"reason": reason})).check_local();

    assert!(closure(json!({"type": "failed", "detail": "boom"})).is_ok());
    assert!(closure(json!({"type": "completed"})).is_ok());
    assert!(closure(json!({"type": "completed", "detail": "ok"})).is_ok());
    for reason in [
        json!({"type": "failed"}),
        json!({"type": "failed", "detail": " "}),
        json!({"type": "cancelled", "detail": ""}),
        json!({"type": "budget_exhausted", "budget": "tokens"}),
    ] {
        assert_eq!(
            closure(reason).unwrap_err().code(),
            ErrorCode::InvalidContent
        );
    }
}

#[test]
fn given_a_mail_payload_when_checked_locally_then_in_reply_to_is_a_plain_entry_id() {
    let conversation = common::cvid(1).to_string();
    let mail = |reply: Value| {
        row_of(
            TurnEntryKind::AgentMail,
            json!({"source_conversation": conversation, "in_reply_to": reply}),
        )
        .with_sender(ActorRef::agent(common::aid(1)))
        .with_author(common::author("A"))
        .check_local()
    };

    assert!(mail(json!(eid(9).to_string())).is_ok());
    assert_eq!(
        mail(json!({"type": "message", "entry": eid(9).to_string()}))
            .unwrap_err()
            .code(),
        ErrorCode::InvalidContent
    );
}
