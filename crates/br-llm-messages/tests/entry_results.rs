mod common;
use common::Raw as _;

use br_llm_messages::{ErrorCode, RuntimeToolResults, TurnEntry, TurnEntryKind};
use common::entries::all_entries;
use common::{bid, eid, refused, result};
use serde_json::json;

#[test]
fn given_runtime_tool_results_without_results_when_built_then_invalid_content() {
    let error = refused(RuntimeToolResults::new(eid(1), vec![]));

    assert_eq!(error.code(), ErrorCode::InvalidContent);
}

#[test]
fn given_two_results_for_the_same_call_in_one_entry_when_built_then_duplicate_result() {
    let error = refused(RuntimeToolResults::new(
        eid(1),
        vec![result(10, 5, "t"), result(11, 5, "t")],
    ));

    assert_eq!(error.code(), ErrorCode::DuplicateResult);
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
fn given_runtime_tool_results_json_with_a_non_result_block_when_loaded_then_disallowed_kind() {
    let value = json!({"type": "runtime_tool_results", "id": eid(1).to_string(),
        "results": [{"type": "text", "id": bid(2).to_string(), "text": "hi"}]});

    let error = refused(TurnEntry::from_value(common::oj(value)));

    assert_eq!(error.code(), ErrorCode::DisallowedKind);
}

#[test]
fn given_runtime_tool_results_json_with_a_repeated_call_when_loaded_then_duplicate_result() {
    let item = |n: u64| {
        json!({"type": "runtime_tool_result", "id": bid(n).to_string(), "call": bid(5).to_string(),
            "tool_name": "t", "is_error": false, "content": []})
    };
    let value = json!({"type": "runtime_tool_results", "id": eid(1).to_string(), "results": [item(10), item(11)]});

    let error = refused(TurnEntry::from_value(common::oj(value)));

    assert_eq!(error.code(), ErrorCode::DuplicateResult);
}

#[test]
fn given_a_result_reusing_the_entry_id_when_built_then_reused_identity() {
    let error = refused(RuntimeToolResults::new(eid(1), vec![result(1, 5, "t")]));

    assert_eq!(error.code(), ErrorCode::ReusedIdentity);
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
