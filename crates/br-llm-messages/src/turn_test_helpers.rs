use super::*;
use crate::block::{AssistantBlock, ToolCall, ToolResult};
use crate::value::{Text, ToolName};

pub(super) fn step_await(ids: &[&str]) -> Step {
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

pub(super) fn step_end() -> Step {
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

pub(super) fn result(id: &str) -> ToolResult {
    ToolResult::new(
        ToolCallId::new(id).unwrap(),
        ToolName::new("search").unwrap(),
        Vec::new(),
        false,
    )
}

pub(super) fn turn(ids: &[&str]) -> Turn {
    Turn::new(
        TurnId::new("turn-1").unwrap(),
        Some(Author::new("id-agent-a", "agent-a").unwrap()),
        step_await(ids),
    )
}
