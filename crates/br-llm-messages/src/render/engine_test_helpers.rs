use super::*;

use crate::block::{AssistantBlock, Thinking, ToolCall, ToolResult};
use crate::step::Step;
use crate::stop_reason::StopReason;
use crate::user_input::{UserInput, UserSource};
use crate::value::{ToolCallId, ToolName, TurnId};

pub(super) fn agent() -> Author {
    Author::new("agent-a").unwrap()
}

pub(super) fn perspective() -> Perspective {
    Perspective::new(agent())
}

pub(super) fn human(body: &str) -> UserInput {
    UserInput::new(
        UserSource::Human,
        None,
        vec![crate::block::UserBlock::text(Text::new(body).unwrap())],
    )
    .unwrap()
}

pub(super) fn call_step(id: &str) -> Step {
    Step::new(
        vec![
            AssistantBlock::Thinking(Thinking {
                text: "reasoning".to_owned(),
                signature: None,
            }),
            AssistantBlock::Text {
                text: Text::new("calling a tool").unwrap(),
            },
            AssistantBlock::ToolCall(ToolCall {
                id: ToolCallId::new(id).unwrap(),
                name: ToolName::new("search").unwrap(),
                arguments: serde_json::json!({ "q": "rust" }),
            }),
        ],
        StopReason::AwaitingToolResults,
        None,
        None,
    )
    .unwrap()
}

pub(super) fn end_step() -> Step {
    Step::new(
        vec![AssistantBlock::Text {
            text: Text::new("the answer").unwrap(),
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
        vec![crate::block::ToolResultBlock::text(
            Text::new("42").unwrap(),
        )],
        false,
    )
}

pub(super) fn own_turn_finished() -> Turn {
    let mut turn = Turn::new(TurnId::new("t1").unwrap(), Some(agent()), call_step("c1"));
    turn.push_result(result("c1")).unwrap();
    turn.push_step(end_step()).unwrap();
    turn
}

pub(super) fn frame_text(message: &WireMessage) -> String {
    match message {
        WireMessage::User { content } => content
            .iter()
            .filter_map(|block| match block {
                WireUserBlock::Text { text } => Some(text.as_str().to_owned()),
                WireUserBlock::Image(_) | WireUserBlock::ToolResult(_) => None,
            })
            .collect::<Vec<_>>()
            .join(" | "),
        WireMessage::Relay { content } => content
            .iter()
            .map(|text| text.as_str().to_owned())
            .collect::<Vec<_>>()
            .join(" | "),
        WireMessage::Assistant { .. } => String::new(),
    }
}
