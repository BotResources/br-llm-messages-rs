use serde_json::Value;

use crate::error::MessageError;
use crate::value::{Signature, Text, ToolCallId, ToolName};

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AssistantBlock {
    Text { text: Text },
    Thinking(Thinking),
    RedactedThinking(RedactedThinking),
    Structured { value: Value },
    ToolCall(ToolCall),
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Thinking {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub signature: Option<Signature>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(try_from = "RawRedactedThinking")]
pub struct RedactedThinking {
    data: String,
}

#[derive(serde::Deserialize)]
struct RawRedactedThinking {
    data: String,
}

impl TryFrom<RawRedactedThinking> for RedactedThinking {
    type Error = MessageError;

    fn try_from(raw: RawRedactedThinking) -> Result<Self, Self::Error> {
        RedactedThinking::new(raw.data)
    }
}

impl RedactedThinking {
    pub fn new(data: impl Into<String>) -> Result<Self, MessageError> {
        let data = data.into();
        if data.is_empty() {
            return Err(MessageError::Blank {
                field: "redacted_thinking_data",
            });
        }
        Ok(Self { data })
    }

    pub fn data(&self) -> &str {
        &self.data
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ToolCall {
    pub id: ToolCallId,
    pub name: ToolName,
    pub arguments: Value,
}

impl AssistantBlock {
    pub fn as_text(&self) -> Option<&Text> {
        match self {
            AssistantBlock::Text { text } => Some(text),
            AssistantBlock::Thinking(_)
            | AssistantBlock::RedactedThinking(_)
            | AssistantBlock::Structured { .. }
            | AssistantBlock::ToolCall(_) => None,
        }
    }

    pub fn as_thinking(&self) -> Option<&Thinking> {
        match self {
            AssistantBlock::Thinking(thinking) => Some(thinking),
            AssistantBlock::Text { .. }
            | AssistantBlock::RedactedThinking(_)
            | AssistantBlock::Structured { .. }
            | AssistantBlock::ToolCall(_) => None,
        }
    }

    pub fn as_structured(&self) -> Option<&Value> {
        match self {
            AssistantBlock::Structured { value } => Some(value),
            AssistantBlock::Text { .. }
            | AssistantBlock::Thinking(_)
            | AssistantBlock::RedactedThinking(_)
            | AssistantBlock::ToolCall(_) => None,
        }
    }

    pub fn as_tool_call(&self) -> Option<&ToolCall> {
        match self {
            AssistantBlock::ToolCall(call) => Some(call),
            AssistantBlock::Text { .. }
            | AssistantBlock::Thinking(_)
            | AssistantBlock::RedactedThinking(_)
            | AssistantBlock::Structured { .. } => None,
        }
    }

    pub fn is_tool_call(&self) -> bool {
        matches!(self, AssistantBlock::ToolCall(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn given_thinking_with_signature_when_serialized_then_flattened() {
        let block = AssistantBlock::Thinking(Thinking {
            text: "let me think".to_owned(),
            signature: Some(Signature::new("sig==").unwrap()),
        });
        let json = serde_json::to_value(&block).unwrap();
        assert_eq!(
            json,
            serde_json::json!({ "type": "thinking", "text": "let me think", "signature": "sig==" })
        );
        assert_eq!(
            serde_json::from_value::<AssistantBlock>(json).unwrap(),
            block
        );
    }

    #[test]
    fn given_empty_thinking_text_when_built_then_accepted() {
        let block = AssistantBlock::Thinking(Thinking {
            text: String::new(),
            signature: None,
        });
        let json = serde_json::to_value(&block).unwrap();
        assert_eq!(json, serde_json::json!({ "type": "thinking", "text": "" }));
        assert_eq!(
            serde_json::from_value::<AssistantBlock>(json).unwrap(),
            block
        );
    }

    #[test]
    fn given_redacted_thinking_when_empty_data_then_refused_both_ways() {
        assert!(matches!(
            RedactedThinking::new(""),
            Err(MessageError::Blank {
                field: "redacted_thinking_data"
            })
        ));
        let json = serde_json::json!({ "type": "redacted_thinking", "data": "" });
        assert!(serde_json::from_value::<AssistantBlock>(json).is_err());
    }

    #[test]
    fn given_structured_block_when_serialized_then_value_nested() {
        let block = AssistantBlock::Structured {
            value: serde_json::json!({ "answer": 42 }),
        };
        let json = serde_json::to_value(&block).unwrap();
        assert_eq!(
            json,
            serde_json::json!({ "type": "structured", "value": { "answer": 42 } })
        );
        assert_eq!(
            serde_json::from_value::<AssistantBlock>(json).unwrap(),
            block
        );
    }

    #[test]
    fn given_structured_array_when_round_tripped_then_identical() {
        let block = AssistantBlock::Structured {
            value: serde_json::json!([1, 2, 3]),
        };
        let json = serde_json::to_value(&block).unwrap();
        assert_eq!(
            serde_json::from_value::<AssistantBlock>(json).unwrap(),
            block
        );
    }

    #[test]
    fn given_tool_call_when_serialized_then_flattened() {
        let block = AssistantBlock::ToolCall(ToolCall {
            id: ToolCallId::new("call_1").unwrap(),
            name: ToolName::new("search").unwrap(),
            arguments: serde_json::json!({ "q": "rust" }),
        });
        let json = serde_json::to_value(&block).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "type": "tool_call",
                "id": "call_1",
                "name": "search",
                "arguments": { "q": "rust" }
            })
        );
        assert_eq!(
            serde_json::from_value::<AssistantBlock>(json).unwrap(),
            block
        );
        assert!(block.is_tool_call());
    }

    #[test]
    fn given_unknown_type_tag_when_deserialized_then_refused() {
        let json = serde_json::json!({ "type": "nonsense", "text": "x" });
        assert!(serde_json::from_value::<AssistantBlock>(json).is_err());
    }
}
