use super::*;
use crate::block::ToolResult;
use crate::value::{Text, ToolName};

pub(super) fn user_text(body: &str) -> WireUserBlock {
    WireUserBlock::Text {
        text: Text::new(body).unwrap(),
    }
}

pub(super) fn answer(id: &str) -> WireUserBlock {
    WireUserBlock::ToolResult(ToolResult::new(
        ToolCallId::new(id).unwrap(),
        ToolName::new("search").unwrap(),
        Vec::new(),
        false,
    ))
}

pub(super) fn call(id: &str) -> AssistantBlock {
    AssistantBlock::ToolCall(ToolCall {
        id: ToolCallId::new(id).unwrap(),
        name: ToolName::new("search").unwrap(),
        arguments: serde_json::json!({}),
    })
}

pub(super) fn said(body: &str) -> AssistantBlock {
    AssistantBlock::Text {
        text: Text::new(body).unwrap(),
    }
}

pub(super) fn user(content: Vec<WireUserBlock>) -> WireMessage {
    WireMessage::User { content }
}

pub(super) fn assistant(content: Vec<AssistantBlock>) -> WireMessage {
    WireMessage::Assistant { content }
}

pub(super) fn relay(body: &str) -> WireMessage {
    WireMessage::Relay {
        content: vec![Text::new(body).unwrap()],
    }
}

pub(super) fn id(value: &str) -> ToolCallId {
    ToolCallId::new(value).unwrap()
}
