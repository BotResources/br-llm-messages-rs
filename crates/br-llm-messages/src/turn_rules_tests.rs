use super::test_helpers::{result, step_await, step_end, turn};
use super::*;
use crate::value::ToolName;

fn named_result(id: &str, name: &str) -> ToolResult {
    ToolResult::new(
        ToolCallId::new(id).unwrap(),
        ToolName::new(name).unwrap(),
        Vec::new(),
        false,
    )
}

fn assert_load_refused(json: serde_json::Value, rule: MessageError) {
    let error = serde_json::from_value::<Turn>(json).unwrap_err();
    assert!(error.to_string().contains(&rule.to_string()), "{error}");
}

fn call_step_json(ids: &[&str]) -> serde_json::Value {
    let calls: Vec<serde_json::Value> = ids
        .iter()
        .map(|id| serde_json::json!({ "type": "tool_call", "id": id, "name": "search", "arguments": {} }))
        .collect();
    serde_json::json!({
        "type": "step",
        "content": calls,
        "stop_reason": { "type": "awaiting_tool_results" }
    })
}

fn results_json(ids: &[&str]) -> serde_json::Value {
    let results: Vec<serde_json::Value> = ids
        .iter()
        .map(|id| serde_json::json!({ "tool_call_id": id, "tool_name": "search", "is_error": false }))
        .collect();
    serde_json::json!({ "type": "tool_results", "results": results })
}

fn end_step_json() -> serde_json::Value {
    serde_json::json!({
        "type": "step",
        "content": [ { "type": "text", "text": "done" } ],
        "stop_reason": { "type": "end_turn" }
    })
}

#[test]
fn given_call_id_of_the_previous_step_when_push_step_then_duplicate_in_turn() {
    let mut turn = turn(&["a"]);
    turn.push_result(result("a")).unwrap();
    let before = turn.clone();
    assert_eq!(
        turn.push_step(step_await(&["a"])),
        Err(MessageError::DuplicateToolCallIdInTurn {
            id: ToolCallId::new("a").unwrap()
        })
    );
    assert_eq!(turn, before);
    assert_eq!(turn.state(), TurnState::AwaitingStep);
}

#[test]
fn given_call_id_of_an_earlier_step_when_push_step_then_duplicate_in_turn() {
    let mut turn = turn(&["a"]);
    turn.push_result(result("a")).unwrap();
    turn.push_step(step_await(&["b"])).unwrap();
    turn.push_result(result("b")).unwrap();
    assert_eq!(
        turn.push_step(step_await(&["c", "a"])),
        Err(MessageError::DuplicateToolCallIdInTurn {
            id: ToolCallId::new("a").unwrap()
        })
    );
}

#[test]
fn given_new_call_ids_when_push_step_then_accepted() {
    let mut turn = turn(&["a"]);
    turn.push_result(result("a")).unwrap();
    turn.push_step(step_await(&["b", "c"])).unwrap();
    assert!(matches!(
        turn.state(),
        TurnState::AwaitingToolResults { .. }
    ));
}

#[test]
fn given_turn_reusing_a_call_id_when_loaded_then_refused_with_the_rule() {
    let json = serde_json::json!({
        "id": "t",
        "items": [
            call_step_json(&["a"]),
            results_json(&["a"]),
            call_step_json(&["a"]),
        ]
    });
    assert_load_refused(
        json,
        MessageError::DuplicateToolCallIdInTurn {
            id: ToolCallId::new("a").unwrap(),
        },
    );
}

#[test]
fn given_result_naming_another_tool_when_push_result_then_tool_name_mismatch() {
    let mut turn = turn(&["a"]);
    let before = turn.clone();
    assert_eq!(
        turn.push_result(named_result("a", "fetch")),
        Err(MessageError::ToolNameMismatch {
            id: ToolCallId::new("a").unwrap(),
            call: ToolName::new("search").unwrap(),
            result: ToolName::new("fetch").unwrap(),
        })
    );
    assert_eq!(turn, before);
}

#[test]
fn given_result_naming_its_call_tool_when_push_result_then_accepted() {
    let mut turn = turn(&["a"]);
    turn.push_result(named_result("a", "search")).unwrap();
    assert_eq!(turn.state(), TurnState::AwaitingStep);
}

#[test]
fn given_turn_with_result_naming_another_tool_when_loaded_then_refused_with_the_rule() {
    let json = serde_json::json!({
        "id": "t",
        "items": [
            call_step_json(&["a"]),
            {
                "type": "tool_results",
                "results": [
                    { "tool_call_id": "a", "tool_name": "fetch", "is_error": false }
                ]
            }
        ]
    });
    assert_load_refused(
        json,
        MessageError::ToolNameMismatch {
            id: ToolCallId::new("a").unwrap(),
            call: ToolName::new("search").unwrap(),
            result: ToolName::new("fetch").unwrap(),
        },
    );
}

#[test]
fn given_empty_results_item_after_awaiting_step_when_loaded_then_refused() {
    let json = serde_json::json!({
        "id": "t",
        "items": [ call_step_json(&["a"]), results_json(&[]) ]
    });
    assert_load_refused(json, MessageError::EmptyToolResults);
}

#[test]
fn given_empty_results_item_after_finished_step_when_loaded_then_refused() {
    let json = serde_json::json!({
        "id": "t",
        "items": [ end_step_json(), results_json(&[]) ]
    });
    assert_load_refused(json, MessageError::EmptyToolResults);
}

#[test]
fn given_empty_results_item_between_two_results_items_when_loaded_then_refused() {
    let json = serde_json::json!({
        "id": "t",
        "items": [
            call_step_json(&["a", "b"]),
            results_json(&["a"]),
            results_json(&[]),
            results_json(&["b"]),
        ]
    });
    assert_load_refused(json, MessageError::EmptyToolResults);
}

#[test]
fn given_consecutive_results_items_when_loaded_then_merged_into_one() {
    let json = serde_json::json!({
        "id": "t",
        "items": [
            call_step_json(&["a", "b"]),
            results_json(&["b"]),
            results_json(&["a"]),
            end_step_json(),
        ]
    });
    let loaded = serde_json::from_value::<Turn>(json).unwrap();

    let mut built = turn(&["a", "b"]);
    built.push_result(result("b")).unwrap();
    built.push_result(result("a")).unwrap();
    built.push_step(step_end()).unwrap();

    assert_eq!(loaded.items().len(), 3);
    assert_eq!(loaded.items()[1..], built.items()[1..]);
}

#[test]
fn given_results_pushed_one_by_one_when_built_then_one_non_empty_item_per_step() {
    let mut turn = turn(&["a", "b"]);
    turn.push_result(result("a")).unwrap();
    turn.push_result(result("b")).unwrap();
    let results: Vec<usize> = turn
        .items()
        .iter()
        .filter_map(|item| match item {
            TurnItem::ToolResults(results) => Some(results.len()),
            TurnItem::Step(_) => None,
        })
        .collect();
    assert_eq!(results, vec![2]);
}
