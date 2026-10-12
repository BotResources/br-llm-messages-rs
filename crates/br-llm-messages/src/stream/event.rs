use crate::stop_reason::StopReason;
use crate::usage::Usage;
use crate::value::{ToolCallId, ToolName};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlockKind {
    Text,
    Thinking,
    RedactedThinking,
    Structured,
    ToolCall,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamEvent {
    BlockStart {
        index: usize,
        kind: BlockKind,
    },
    TextDelta {
        index: usize,
        text: String,
    },
    ThinkingDelta {
        index: usize,
        text: String,
    },
    SignatureDelta {
        index: usize,
        signature: String,
    },
    RedactedThinkingData {
        index: usize,
        data: String,
    },
    StructuredDelta {
        index: usize,
        json_fragment: String,
    },
    ToolCallStart {
        index: usize,
        id: ToolCallId,
        name: ToolName,
    },
    ToolCallArgumentsDelta {
        index: usize,
        json_fragment: String,
    },
    BlockEnd {
        index: usize,
    },
    Finish {
        stop_reason: StopReason,
        #[serde(skip_serializing_if = "Option::is_none", default)]
        usage: Option<Usage>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn given_block_kind_when_serialized_then_plain_string() {
        assert_eq!(
            serde_json::to_value(BlockKind::RedactedThinking).unwrap(),
            serde_json::json!("redacted_thinking")
        );
    }

    #[test]
    fn given_block_start_when_serialized_then_type_tagged() {
        let event = StreamEvent::BlockStart {
            index: 0,
            kind: BlockKind::Text,
        };
        let json = serde_json::to_value(&event).unwrap();
        assert_eq!(
            json,
            serde_json::json!({ "type": "block_start", "index": 0, "kind": "text" })
        );
        assert_eq!(serde_json::from_value::<StreamEvent>(json).unwrap(), event);
    }

    #[test]
    fn given_tool_call_start_when_round_tripped_then_identical() {
        let event = StreamEvent::ToolCallStart {
            index: 2,
            id: ToolCallId::new("call_1").unwrap(),
            name: ToolName::new("search").unwrap(),
        };
        let json = serde_json::to_value(&event).unwrap();
        assert_eq!(serde_json::from_value::<StreamEvent>(json).unwrap(), event);
    }

    #[test]
    fn given_finish_when_serialized_then_usage_optional() {
        let event = StreamEvent::Finish {
            stop_reason: StopReason::EndTurn,
            usage: None,
        };
        let json = serde_json::to_value(&event).unwrap();
        assert_eq!(
            json,
            serde_json::json!({ "type": "finish", "stop_reason": { "type": "end_turn" } })
        );
    }
}
