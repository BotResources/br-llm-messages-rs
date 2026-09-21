use crate::block::{AssistantBlock, Image, ToolResult};
use crate::value::{Author, Text};

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "role", rename_all = "snake_case")]
pub enum WireMessage {
    User { content: Vec<WireUserBlock> },
    Assistant { content: Vec<AssistantBlock> },
    Relay { content: Vec<Text> },
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WireUserBlock {
    Text { text: Text },
    Image(Image),
    ToolResult(ToolResult),
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Perspective {
    pub agent: Author,
}

impl Perspective {
    pub fn new(agent: Author) -> Self {
        Self { agent }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::ToolCallId;

    #[test]
    fn given_user_message_when_serialized_then_role_tagged() {
        let message = WireMessage::User {
            content: vec![WireUserBlock::Text {
                text: Text::new("hi").unwrap(),
            }],
        };
        let json = serde_json::to_value(&message).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "role": "user",
                "content": [ { "type": "text", "text": "hi" } ]
            })
        );
        assert_eq!(
            serde_json::from_value::<WireMessage>(json).unwrap(),
            message
        );
    }

    #[test]
    fn given_tool_result_block_when_round_tripped_then_identical() {
        let block = WireUserBlock::ToolResult(ToolResult::new(
            ToolCallId::new("c1").unwrap(),
            crate::value::ToolName::new("search").unwrap(),
            Vec::new(),
            false,
        ));
        let json = serde_json::to_value(&block).unwrap();
        assert_eq!(
            serde_json::from_value::<WireUserBlock>(json).unwrap(),
            block
        );
    }

    #[test]
    fn given_perspective_when_round_tripped_then_identical() {
        let perspective = Perspective::new(Author::new("agent-a").unwrap());
        let json = serde_json::to_value(&perspective).unwrap();
        assert_eq!(
            serde_json::from_value::<Perspective>(json).unwrap(),
            perspective
        );
    }
}
