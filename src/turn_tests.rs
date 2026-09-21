use super::*;
use crate::block::{AssistantBlock, ToolCall, ToolResult};
use crate::value::{Text, ToolName};

fn step_await(ids: &[&str]) -> Step {
    let mut content = vec![AssistantBlock::Text {
        text: Text::new("calling").unwrap(),
    }];
    for id in ids {
        content.push(AssistantBlock::ToolCall(ToolCall {
            id: ToolCallId::new(*id).unwrap(),
            name: ToolName::new("search").unwrap(),
            arguments: serde_json::json!({}),
        }));
    }
    Step::new(content, StopReason::AwaitingToolResults, None, None).unwrap()
}

fn step_end() -> Step {
    Step::new(
        vec![AssistantBlock::Text {
            text: Text::new("done").unwrap(),
        }],
        StopReason::EndTurn,
        None,
        None,
    )
    .unwrap()
}

fn result(id: &str) -> ToolResult {
    ToolResult::new(
        ToolCallId::new(id).unwrap(),
        ToolName::new("search").unwrap(),
        Vec::new(),
        false,
    )
}

fn turn(ids: &[&str]) -> Turn {
    Turn::new(
        TurnId::new("turn-1").unwrap(),
        Some(Author::new("agent-a").unwrap()),
        step_await(ids),
    )
}

#[test]
fn given_finished_first_step_when_state_then_finished() {
    let turn = Turn::new(TurnId::new("t").unwrap(), None, step_end());
    assert!(matches!(turn.state(), TurnState::Finished { .. }));
}

#[test]
fn given_awaiting_step_when_push_result_all_then_awaiting_step() {
    let mut turn = turn(&["a", "b"]);
    assert!(matches!(
        turn.state(),
        TurnState::AwaitingToolResults { .. }
    ));
    turn.push_result(result("b")).unwrap();
    assert!(matches!(
        turn.state(),
        TurnState::AwaitingToolResults { .. }
    ));
    turn.push_result(result("a")).unwrap();
    assert_eq!(turn.state(), TurnState::AwaitingStep);
}

#[test]
fn given_awaiting_step_when_push_step_then_finished() {
    let mut turn = turn(&["a"]);
    turn.push_result(result("a")).unwrap();
    turn.push_step(step_end()).unwrap();
    assert!(matches!(turn.state(), TurnState::Finished { .. }));
}

#[test]
fn given_foreign_id_when_push_result_then_not_pending() {
    let mut turn = turn(&["a"]);
    assert!(matches!(
        turn.push_result(result("zzz")),
        Err(MessageError::ToolResultNotPending { .. })
    ));
}

#[test]
fn given_already_collected_id_when_push_result_then_duplicate() {
    let mut turn = turn(&["a", "b"]);
    turn.push_result(result("a")).unwrap();
    assert!(matches!(
        turn.push_result(result("a")),
        Err(MessageError::DuplicateToolResultId { .. })
    ));
}

#[test]
fn given_finished_turn_when_push_result_then_not_awaiting_results() {
    let mut turn = Turn::new(TurnId::new("t").unwrap(), None, step_end());
    assert!(matches!(
        turn.push_result(result("a")),
        Err(MessageError::TurnNotAwaitingResults { .. })
    ));
}

#[test]
fn given_awaiting_results_when_push_step_then_not_awaiting_step() {
    let mut turn = turn(&["a"]);
    assert!(matches!(
        turn.push_step(step_end()),
        Err(MessageError::TurnNotAwaitingStep { .. })
    ));
}

#[test]
fn given_all_answered_when_push_step_second_time_then_refused() {
    let mut turn = turn(&["a"]);
    turn.push_result(result("a")).unwrap();
    turn.push_step(step_end()).unwrap();
    assert!(matches!(
        turn.push_step(step_end()),
        Err(MessageError::TurnNotAwaitingStep { .. })
    ));
}

#[test]
fn given_partial_results_item_when_push_results_batch_stops_at_first_refusal() {
    let mut turn = turn(&["a", "b"]);
    let outcome = turn.push_results([result("a"), result("zzz"), result("b")]);
    assert!(matches!(
        outcome,
        Err(MessageError::ToolResultNotPending { .. })
    ));
    let ids: Vec<&str> = match turn.items().last().unwrap() {
        TurnItem::ToolResults(results) => results.ids().map(ToolCallId::as_str).collect(),
        TurnItem::Step(_) => Vec::new(),
    };
    assert_eq!(ids, vec!["a"]);
}

#[test]
fn given_tool_call_id_reused_across_steps_when_pushed_then_accepted() {
    let mut turn = turn(&["a"]);
    turn.push_result(result("a")).unwrap();
    turn.push_step(step_await(&["a"])).unwrap();
    assert!(matches!(
        turn.state(),
        TurnState::AwaitingToolResults { .. }
    ));
    turn.push_result(result("a")).unwrap();
    assert_eq!(turn.state(), TurnState::AwaitingStep);
}

#[test]
fn given_empty_results_item_when_loaded_then_dropped_deterministically() {
    let json = serde_json::json!({
        "id": "t",
        "items": [
            {
                "type": "step",
                "content": [
                    { "type": "tool_call", "id": "a", "name": "search", "arguments": {} }
                ],
                "stop_reason": { "type": "awaiting_tool_results" }
            },
            { "type": "tool_results", "results": [] }
        ]
    });
    let turn = serde_json::from_value::<Turn>(json).unwrap();
    assert_eq!(turn.items().len(), 1);
    assert!(matches!(
        turn.state(),
        TurnState::AwaitingToolResults { .. }
    ));
}

#[test]
fn given_complete_turn_when_round_tripped_then_identical() {
    let mut turn = turn(&["a", "b"]);
    turn.push_result(result("a")).unwrap();
    turn.push_result(result("b")).unwrap();
    turn.push_step(step_end()).unwrap();
    let json = serde_json::to_value(&turn).unwrap();
    assert_eq!(serde_json::from_value::<Turn>(json).unwrap(), turn);
}

#[test]
fn given_items_starting_with_results_when_loaded_then_must_start_with_step() {
    let json = serde_json::json!({
        "id": "t",
        "items": [
            { "type": "tool_results", "results": [] }
        ]
    });
    assert!(serde_json::from_value::<Turn>(json).is_err());
}

#[test]
fn given_foreign_result_in_items_when_loaded_then_refused() {
    let json = serde_json::json!({
        "id": "t",
        "items": [
            {
                "type": "step",
                "content": [
                    { "type": "tool_call", "id": "a", "name": "search", "arguments": {} }
                ],
                "stop_reason": { "type": "awaiting_tool_results" }
            },
            {
                "type": "tool_results",
                "results": [
                    { "tool_call_id": "zzz", "tool_name": "search", "content": [], "is_error": false }
                ]
            }
        ]
    });
    assert!(serde_json::from_value::<Turn>(json).is_err());
}

#[test]
fn given_consecutive_steps_without_results_when_loaded_then_refused() {
    let json = serde_json::json!({
        "id": "t",
        "items": [
            {
                "type": "step",
                "content": [
                    { "type": "tool_call", "id": "a", "name": "search", "arguments": {} }
                ],
                "stop_reason": { "type": "awaiting_tool_results" }
            },
            {
                "type": "step",
                "content": [ { "type": "text", "text": "done" } ],
                "stop_reason": { "type": "end_turn" }
            }
        ]
    });
    assert!(serde_json::from_value::<Turn>(json).is_err());
}
