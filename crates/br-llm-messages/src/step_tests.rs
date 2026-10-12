use super::*;
use crate::block::ToolCall;
use crate::value::{ToolCallId, ToolName};

fn text_block(body: &str) -> AssistantBlock {
    AssistantBlock::Text {
        text: Text::new(body).unwrap(),
    }
}

fn call_block(id: &str) -> AssistantBlock {
    AssistantBlock::ToolCall(ToolCall {
        id: ToolCallId::new(id).unwrap(),
        name: ToolName::new("search").unwrap(),
        arguments: serde_json::json!({}),
    })
}

#[test]
fn given_empty_content_when_new_then_refused_both_ways() {
    assert!(matches!(
        Step::new(Vec::new(), StopReason::EndTurn, None, None),
        Err(MessageError::EmptyStep)
    ));
    let json = serde_json::json!({ "content": [], "stop_reason": { "type": "end_turn" } });
    assert!(serde_json::from_value::<Step>(json).is_err());
}

#[test]
fn given_text_after_tool_call_when_new_then_not_at_tail_both_ways() {
    let content = vec![call_block("call_1"), text_block("after")];
    assert!(matches!(
        Step::new(content, StopReason::AwaitingToolResults, None, None),
        Err(MessageError::ToolCallNotAtTail)
    ));
    let json = serde_json::json!({
        "content": [
            { "type": "tool_call", "id": "call_1", "name": "search", "arguments": {} },
            { "type": "text", "text": "after" }
        ],
        "stop_reason": { "type": "awaiting_tool_results" }
    });
    assert!(serde_json::from_value::<Step>(json).is_err());
}

#[test]
fn given_tool_calls_as_tail_when_new_then_accepted() {
    let content = vec![
        text_block("before"),
        call_block("call_1"),
        call_block("call_2"),
    ];
    assert!(Step::new(content, StopReason::AwaitingToolResults, None, None).is_ok());
}

#[test]
fn given_awaiting_without_tool_calls_when_new_then_refused_both_ways() {
    let content = vec![text_block("hi")];
    assert!(matches!(
        Step::new(content, StopReason::AwaitingToolResults, None, None),
        Err(MessageError::AwaitingToolResultsWithoutCalls)
    ));
    let json = serde_json::json!({
        "content": [ { "type": "text", "text": "hi" } ],
        "stop_reason": { "type": "awaiting_tool_results" }
    });
    assert!(serde_json::from_value::<Step>(json).is_err());
}

#[test]
fn given_tool_calls_without_awaiting_when_new_then_refused_both_ways() {
    let content = vec![call_block("call_1")];
    assert!(matches!(
        Step::new(content, StopReason::EndTurn, None, None),
        Err(MessageError::ToolCallsWithoutAwaiting)
    ));
    let json = serde_json::json!({
        "content": [ { "type": "tool_call", "id": "call_1", "name": "search", "arguments": {} } ],
        "stop_reason": { "type": "end_turn" }
    });
    assert!(serde_json::from_value::<Step>(json).is_err());
}

#[test]
fn given_duplicate_tool_call_ids_when_new_then_refused_both_ways() {
    let content = vec![call_block("dup"), call_block("dup")];
    assert!(matches!(
        Step::new(content, StopReason::AwaitingToolResults, None, None),
        Err(MessageError::DuplicateToolCallId { .. })
    ));
    let json = serde_json::json!({
        "content": [
            { "type": "tool_call", "id": "dup", "name": "search", "arguments": {} },
            { "type": "tool_call", "id": "dup", "name": "search", "arguments": {} }
        ],
        "stop_reason": { "type": "awaiting_tool_results" }
    });
    assert!(serde_json::from_value::<Step>(json).is_err());
}

#[test]
fn given_mixed_step_when_accessors_read_then_order_preserved() {
    let content = vec![
        AssistantBlock::Thinking(crate::block::Thinking {
            text: "hmm".to_owned(),
            signature: None,
        }),
        text_block("one"),
        text_block("two"),
        call_block("call_1"),
    ];
    let step = Step::new(content, StopReason::AwaitingToolResults, None, None).unwrap();
    let texts: Vec<&str> = step.text().map(Text::as_str).collect();
    assert_eq!(texts, vec!["one", "two"]);
    assert_eq!(step.thinking().count(), 1);
    assert_eq!(step.tool_calls().count(), 1);
}

#[test]
fn given_step_with_usage_and_model_when_round_tripped_then_identical() {
    let step = Step::new(
        vec![text_block("done")],
        StopReason::EndTurn,
        Some(Usage {
            input_tokens: 5,
            output_tokens: 7,
            ..Usage::default()
        }),
        Some(ModelId::new("claude-fable-5-1").unwrap()),
    )
    .unwrap();
    let json = serde_json::to_value(&step).unwrap();
    assert_eq!(serde_json::from_value::<Step>(json).unwrap(), step);
}
