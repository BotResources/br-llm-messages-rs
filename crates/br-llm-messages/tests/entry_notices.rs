mod common;

use br_llm_messages::{
    ErrorCode, IncomingEntry, QueuePosition, TaskCompleted, TaskFailed, TaskStarted, TurnEntry,
};
use common::entries::all_entries;
use common::{cvid, eid, kid, refused, rid};
use serde_json::json;

#[test]
fn given_queue_position_zero_when_built_then_invalid_content_and_one_is_accepted() {
    let error = refused(QueuePosition::new(eid(1), rid(1), 0));

    assert_eq!(error.code(), ErrorCode::InvalidContent);
    assert_eq!(error.location(), Some("position"));
    assert_eq!(QueuePosition::new(eid(1), rid(1), 1).unwrap().position(), 1);
}

#[test]
fn given_queue_position_json_with_zero_or_negative_when_loaded_then_refused() {
    for position in [json!(0), json!(-1)] {
        let value = json!({"type": "queue_position", "id": eid(1).to_string(),
            "request": rid(1).to_string(), "position": position});

        let error = refused(TurnEntry::from_value(common::oj(value)));

        assert_eq!(error.code(), ErrorCode::InvalidContent, "{position}");
    }
}

#[test]
fn given_an_empty_or_blank_task_summary_when_built_then_refused() {
    for summary in ["", "  \n"] {
        let error = refused(TaskCompleted::new(eid(1), kid(1)).with_summary(summary));

        assert_eq!(error.code(), ErrorCode::InvalidContent);
        assert_eq!(error.location(), Some("summary"));
    }
    let kept = TaskCompleted::new(eid(1), kid(1))
        .with_summary(" done ")
        .unwrap();
    assert_eq!(kept.summary(), Some(" done "));
}

#[test]
fn given_task_started_when_built_then_request_task_and_launched_conversation_are_kept() {
    let started = TaskStarted::new(eid(1), rid(2), kid(3)).with_conversation(cvid(4));

    assert_eq!(started.request(), rid(2));
    assert_eq!(started.task(), kid(3));
    assert_eq!(started.conversation(), Some(cvid(4)));
    assert_eq!(
        TaskStarted::new(eid(1), rid(2), kid(3)).conversation(),
        None
    );
}

#[test]
fn given_each_notice_json_with_an_author_key_when_loaded_then_author_forbidden() {
    let notices = [
        json!({"type": "request_accepted", "id": eid(1).to_string(), "request": rid(1).to_string()}),
        json!({"type": "queue_position", "id": eid(1).to_string(), "request": rid(1).to_string(), "position": 2}),
        json!({"type": "task_started", "id": eid(1).to_string(), "request": rid(1).to_string(), "task": kid(1).to_string()}),
        json!({"type": "task_completed", "id": eid(1).to_string(), "task": kid(1).to_string()}),
        json!({"type": "task_failed", "id": eid(1).to_string(), "task": kid(1).to_string(), "reason": "x"}),
    ];
    for notice in notices {
        assert!(
            TurnEntry::from_value(common::oj(notice.clone())).is_ok(),
            "{notice}"
        );
        let mut with_author = notice.clone();
        with_author["author"] = json!("Codex");

        let error = refused(TurnEntry::from_value(common::oj(with_author)));

        assert_eq!(error.code(), ErrorCode::AuthorForbidden, "{notice}");
    }
}

#[test]
fn given_a_task_failed_json_without_reason_when_loaded_then_invalid_content() {
    let value =
        json!({"type": "task_failed", "id": eid(1).to_string(), "task": kid(1).to_string()});

    let error = refused(TurnEntry::from_value(common::oj(value)));

    assert_eq!(error.code(), ErrorCode::InvalidContent);
}

#[test]
fn given_a_task_failed_when_built_then_the_reason_is_kept_and_a_blank_one_is_refused() {
    let failed = TaskFailed::new(eid(1), kid(1), "  out of disk \n").unwrap();

    assert_eq!(failed.reason(), "  out of disk \n");
    for blank in ["", "   ", "\n\t"] {
        let error = refused(TaskFailed::new(eid(1), kid(1), blank));
        assert_eq!(error.code(), ErrorCode::InvalidContent);
        assert_eq!(error.object_id(), Some(eid(1).as_uuid()));

        let value = json!({"type": "task_failed", "id": eid(1).to_string(),
            "task": kid(1).to_string(), "reason": blank});
        assert_eq!(
            refused(TurnEntry::from_value(common::oj(value))).code(),
            ErrorCode::InvalidContent
        );
    }
}

#[test]
fn given_a_task_failed_json_with_the_retired_failure_object_when_loaded_then_invalid_content() {
    let value = json!({"type": "task_failed", "id": eid(1).to_string(),
        "task": kid(1).to_string(), "failure": {"code": "x"}});

    let error = refused(TurnEntry::from_value(common::oj(value)));

    assert_eq!(error.code(), ErrorCode::InvalidContent);
}

#[test]
fn given_incoming_entries_when_converted_to_turn_entries_and_back_then_they_are_equal() {
    for entry in all_entries().into_iter().filter(|e| e.is_incoming()) {
        let incoming = IncomingEntry::try_from(entry.clone()).unwrap();

        assert_eq!(incoming.kind(), Some(entry.kind()));
        assert_eq!(incoming.id(), entry.id());
        assert_eq!(TurnEntry::try_from(incoming).unwrap(), entry);
    }
}

#[test]
fn given_non_incoming_entries_when_converted_then_disallowed_kind_names_the_entry() {
    let non_incoming: Vec<TurnEntry> = all_entries()
        .into_iter()
        .filter(|e| !e.is_incoming())
        .collect();
    assert_eq!(non_incoming.len(), 3);

    for entry in non_incoming {
        let error = refused(IncomingEntry::try_from(entry.clone()));

        assert_eq!(error.code(), ErrorCode::DisallowedKind, "{}", entry.kind());
        assert_eq!(error.object_id(), Some(entry.id().as_uuid()));
        assert_eq!(
            refused(IncomingEntry::try_from(&entry)).code(),
            ErrorCode::DisallowedKind
        );
    }
}

#[test]
fn given_json_of_a_non_incoming_kind_when_loaded_as_incoming_then_disallowed_kind() {
    for entry in all_entries().into_iter().filter(|e| !e.is_incoming()) {
        let text = entry.to_json().unwrap();

        let error = refused(IncomingEntry::from_json(&text));

        assert_eq!(error.code(), ErrorCode::DisallowedKind, "{}", entry.kind());
        assert_eq!(error.object_id(), Some(entry.id().as_uuid()));
    }
}

#[test]
fn given_json_of_an_unknown_kind_when_loaded_as_incoming_then_variant_mismatch() {
    let value = json!({"type": "carrier_pigeon", "id": eid(1).to_string()});

    let error = refused(IncomingEntry::from_value(common::oj(value)));

    assert_eq!(error.code(), ErrorCode::VariantMismatch);
}
