mod common;
use common::Raw as _;

use br_llm_messages::{
    BudgetKind, ClosureReason, Error, ErrorCode, FailureDetail, RuntimeToolResults, ToolDefinition,
    ToolUnmounted, TurnEntry, TurnEntryKind,
};
use common::entries::all_entries;
use common::{bid, eid, refused, result, tool};
use serde_json::json;

#[test]
fn given_runtime_tool_results_without_results_when_built_then_missing_content() {
    let error = refused(RuntimeToolResults::new(eid(1), vec![]));

    assert_eq!(error.code(), ErrorCode::MissingContent);
}

#[test]
fn given_two_results_for_the_same_call_in_one_entry_when_built_then_duplicate_tool_result() {
    let error = refused(RuntimeToolResults::new(
        eid(1),
        vec![result(10, 5, "t"), result(11, 5, "t")],
    ));

    assert_eq!(error.code(), ErrorCode::DuplicateToolResult);
    assert_eq!(error.object_id(), Some(bid(5).as_uuid()));
}

#[test]
fn given_results_for_parallel_calls_when_built_then_recorded_order_is_kept() {
    let delivery = RuntimeToolResults::new(
        eid(1),
        vec![result(10, 9, "t"), result(11, 3, "t"), result(12, 6, "u")],
    )
    .unwrap();

    let calls: Vec<_> = delivery.results().iter().map(|r| r.call()).collect();

    assert_eq!(calls, vec![bid(9), bid(3), bid(6)]);
}

#[test]
fn given_runtime_tool_results_json_with_a_non_result_block_when_loaded_then_disallowed_block_kind()
{
    let value = json!({"type": "runtime_tool_results", "id": eid(1).to_string(),
        "results": [{"type": "text", "id": bid(2).to_string(), "text": "hi"}]});

    let error = refused(TurnEntry::from_value(common::oj(value)));

    assert_eq!(error.code(), ErrorCode::DisallowedBlockKind);
}

#[test]
fn given_runtime_tool_results_json_with_a_repeated_call_when_loaded_then_duplicate_tool_result() {
    let item = |n: u64| {
        json!({"type": "runtime_tool_result", "id": bid(n).to_string(), "call": bid(5).to_string(),
            "tool_name": "t", "status": "success", "content": []})
    };
    let value = json!({"type": "runtime_tool_results", "id": eid(1).to_string(), "results": [item(10), item(11)]});

    let error = refused(TurnEntry::from_value(common::oj(value)));

    assert_eq!(error.code(), ErrorCode::DuplicateToolResult);
}

#[test]
fn given_a_result_reusing_the_entry_id_when_built_then_reused_identity() {
    let error = refused(RuntimeToolResults::new(eid(1), vec![result(1, 5, "t")]));

    assert_eq!(error.code(), ErrorCode::ReusedIdentity);
}

#[test]
fn given_an_inline_tool_definition_when_schema_is_not_an_object_then_refused() {
    for schema in [json!(null), json!([]), json!("{}"), json!(1)] {
        let error = refused(ToolDefinition::inline(
            tool("t"),
            "d",
            common::oj(schema.clone()),
        ));

        assert_eq!(error.code(), ErrorCode::InvalidContent, "{schema}");
        assert_eq!(error.location(), Some("input_schema"));
    }
    let kept =
        ToolDefinition::inline(tool("t"), "d", common::oj(json!({"type": "object"}))).unwrap();
    assert_eq!(kept.name(), &tool("t"));
}

#[test]
fn given_a_versioned_definition_with_an_empty_version_when_built_then_refused() {
    let error = refused(ToolDefinition::versioned(tool("t"), ""));

    assert_eq!(error.code(), ErrorCode::InvalidContent);
    assert_eq!(error.location(), Some("version"));
}

#[test]
fn given_tool_definition_json_when_malformed_then_each_defect_has_its_own_code() {
    let cases = [
        (
            json!({"type": "inline", "name": "t", "description": "d", "input_schema": []}),
            ErrorCode::InvalidContent,
        ),
        (
            json!({"type": "inline", "name": "t", "input_schema": {}}),
            ErrorCode::MissingContent,
        ),
        (
            json!({"type": "inline", "name": "bad name", "description": "d", "input_schema": {}}),
            ErrorCode::InvalidContent,
        ),
        (
            json!({"type": "versioned", "name": "t", "version": "1", "extra": 1}),
            ErrorCode::UnsupportedField,
        ),
        (
            json!({"type": "remote", "name": "t"}),
            ErrorCode::UnsupportedVariant,
        ),
    ];
    for (value, code) in cases {
        let error = refused(
            serde_json::from_value::<ToolDefinition>(value.clone())
                .map_err(|e| br_llm_messages::Error::from_json_error(&e)),
        );

        assert_eq!(error.code(), code, "{value}");
    }
}

#[test]
fn given_a_tool_unmounted_when_built_then_it_references_an_entry_id_not_a_name() {
    let unmount = ToolUnmounted::new(eid(2), eid(1));

    assert_eq!(unmount.mounting(), eid(1));
    assert_eq!(
        serde_json::to_value(&unmount).unwrap(),
        json!({"id": eid(2).to_string(), "mounting": eid(1).to_string()})
    );
}

#[test]
fn given_a_cancelled_closure_with_an_empty_detail_when_built_then_refused() {
    let error = refused(ClosureReason::cancelled(Some(String::new())));

    assert_eq!(error.code(), ErrorCode::InvalidContent);
    assert!(ClosureReason::cancelled(None).is_ok());
}

#[test]
fn given_the_five_closure_reasons_when_serialized_then_typed_details_are_golden() {
    let failure = || {
        FailureDetail::new("oom")
            .unwrap()
            .with_message("out of memory")
    };
    let cases = [
        (ClosureReason::completed(), json!({"type": "completed"})),
        (
            ClosureReason::cancelled(Some("user".into())).unwrap(),
            json!({"type": "cancelled", "detail": "user"}),
        ),
        (
            ClosureReason::interrupted(failure()),
            json!({"type": "interrupted", "detail": {"code": "oom", "message": "out of memory"}}),
        ),
        (
            ClosureReason::budget_exhausted(BudgetKind::WallClock),
            json!({"type": "budget_exhausted", "budget": "wall_clock"}),
        ),
        (
            ClosureReason::failed(failure()),
            json!({"type": "failed", "detail": {"code": "oom", "message": "out of memory"}}),
        ),
    ];
    for (reason, golden) in cases {
        assert_eq!(serde_json::to_value(&reason).unwrap(), golden);
        assert_eq!(
            serde_json::from_value::<ClosureReason>(golden).unwrap(),
            reason
        );
    }
}

#[test]
fn given_a_closure_with_an_unknown_reason_when_loaded_then_unsupported_variant() {
    let value =
        json!({"type": "turn_closure", "id": eid(1).to_string(), "reason": {"type": "abducted"}});

    let error = refused(TurnEntry::from_value(common::oj(value)));

    assert_eq!(error.code(), ErrorCode::UnsupportedVariant);
}

#[test]
fn given_a_closure_reason_missing_or_with_extra_details_when_loaded_then_refused() {
    let missing =
        json!({"type": "turn_closure", "id": eid(1).to_string(), "reason": {"type": "failed"}});
    let extra = json!({"type": "turn_closure", "id": eid(1).to_string(), "reason": {"type": "completed", "detail": "x"}});
    let bad_budget = json!({"type": "turn_closure", "id": eid(1).to_string(), "reason": {"type": "budget_exhausted", "budget": "patience"}});

    assert_eq!(
        refused(TurnEntry::from_value(common::oj(missing))).code(),
        ErrorCode::MissingContent
    );
    assert_eq!(
        refused(TurnEntry::from_value(common::oj(extra))).code(),
        ErrorCode::UnsupportedField
    );
    assert_eq!(
        refused(TurnEntry::from_value(common::oj(bad_budget))).code(),
        ErrorCode::UnsupportedVariant
    );
}

#[test]
fn given_every_forbidden_author_kind_when_json_carries_an_author_then_author_forbidden() {
    for entry in all_entries() {
        let kind = entry.kind();
        let required = matches!(kind, TurnEntryKind::AgentRequest | TurnEntryKind::AgentMail);
        let mut value = entry.to_value().unwrap().raw();
        let had_author = value.get("author").is_some();

        assert_eq!(had_author, required, "{kind}");
        value["author"] = json!("Codex GPT-6.1-Sol high");
        let outcome = TurnEntry::from_value(common::oj(value));

        if required {
            assert!(outcome.is_ok(), "{kind}");
        } else {
            assert_eq!(
                refused(outcome).code(),
                ErrorCode::AuthorForbidden,
                "{kind}"
            );
        }
    }
}

#[test]
fn given_a_forbidden_author_error_when_displayed_then_it_names_the_entry_without_the_author_value()
{
    let value = json!({"type": "turn_closure", "id": eid(1).to_string(),
        "reason": {"type": "completed"}, "author": "SENTINEL-AUTHOR-LABEL"});

    let error = refused(TurnEntry::from_value(common::oj(value)));

    assert_eq!(error.object_id(), Some(eid(1).as_uuid()));
    assert!(!error.to_string().contains("SENTINEL"));
    assert!(!format!("{error:?}").contains("SENTINEL"));
}

#[test]
fn given_a_failed_closure_reason_written_with_the_retired_failure_key_when_loaded_then_unsupported_field()
 {
    let retired = json!({"type": "failed", "failure": {"code": "oom"}});

    let error = serde_json::from_value::<ClosureReason>(retired).unwrap_err();

    assert_eq!(
        Error::from_json_error(&error).code(),
        ErrorCode::UnsupportedField
    );
}
