use crate::stop_reason::StopReason;
use crate::usage::Usage;
use crate::value::{ToolCallId, ToolName};

/// The kind of a block a generic block start opens. It may grow in a later
/// version. It serializes for a reader but is not loaded from JSON on its own.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum BlockKind {
    Text,
    Thinking,
    RedactedThinking,
    Structured,
    ToolCall,
}

/// A provider-neutral stream event of one model response, supplied by an
/// adapter. It may grow in a later version. Events are loaded from JSON (a
/// recorded stream, for example) but never serialized.
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[non_exhaustive]
pub enum StreamEvent {
    BlockStart {
        index: usize,
        #[serde(deserialize_with = "block_kind_in_event")]
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
        #[serde(default)]
        usage: Option<Usage>,
    },
}

/// The block kinds a stream event names in JSON. `BlockKind` itself is not
/// loaded from JSON, so a block start reads its kind through this private
/// mirror.
#[derive(serde::Deserialize)]
#[serde(rename_all = "snake_case")]
enum EventBlockKind {
    Text,
    Thinking,
    RedactedThinking,
    Structured,
    ToolCall,
}

fn block_kind_in_event<'de, D>(deserializer: D) -> Result<BlockKind, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let kind = match <EventBlockKind as serde::Deserialize>::deserialize(deserializer)? {
        EventBlockKind::Text => BlockKind::Text,
        EventBlockKind::Thinking => BlockKind::Thinking,
        EventBlockKind::RedactedThinking => BlockKind::RedactedThinking,
        EventBlockKind::Structured => BlockKind::Structured,
        EventBlockKind::ToolCall => BlockKind::ToolCall,
    };
    Ok(kind)
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
    fn given_block_start_json_when_loaded_then_kind_read() {
        let json = serde_json::json!({ "type": "block_start", "index": 0, "kind": "text" });
        assert_eq!(
            serde_json::from_value::<StreamEvent>(json).unwrap(),
            StreamEvent::BlockStart {
                index: 0,
                kind: BlockKind::Text,
            }
        );
    }

    /// Builds the list of every block kind and a wildcard-free match from the
    /// same variant names, so a new variant does not compile until it is
    /// named here, and naming it puts it in the list the test below loads
    /// through the private mirror.
    macro_rules! every_block_kind {
        ($($variant:ident),+ $(,)?) => {{
            fn _exhaustive(kind: &BlockKind) {
                match kind {
                    $(BlockKind::$variant)|+ => {}
                }
            }
            vec![$(BlockKind::$variant),+]
        }};
    }

    fn every_block_kind() -> Vec<BlockKind> {
        every_block_kind!(Text, Thinking, RedactedThinking, Structured, ToolCall)
    }

    #[test]
    fn given_every_serialized_block_kind_when_loaded_in_a_block_start_then_the_same_kind() {
        for kind in every_block_kind() {
            let json = serde_json::json!({
                "type": "block_start",
                "index": 3,
                "kind": serde_json::to_value(&kind).unwrap(),
            });
            assert_eq!(
                serde_json::from_value::<StreamEvent>(json).unwrap(),
                StreamEvent::BlockStart { index: 3, kind }
            );
        }
    }

    #[test]
    fn given_unknown_block_kind_in_json_when_loaded_then_refused() {
        let json = serde_json::json!({ "type": "block_start", "index": 0, "kind": "audio" });
        assert!(serde_json::from_value::<StreamEvent>(json).is_err());
    }

    #[test]
    fn given_tool_call_start_json_when_loaded_then_values_checked() {
        let json = serde_json::json!({
            "type": "tool_call_start", "index": 2, "id": "call_1", "name": "search"
        });
        assert_eq!(
            serde_json::from_value::<StreamEvent>(json).unwrap(),
            StreamEvent::ToolCallStart {
                index: 2,
                id: ToolCallId::new("call_1").unwrap(),
                name: ToolName::new("search").unwrap(),
            }
        );
        let json = serde_json::json!({
            "type": "tool_call_start", "index": 2, "id": "call_1", "name": "web.search"
        });
        assert!(serde_json::from_value::<StreamEvent>(json).is_err());
    }

    #[test]
    fn given_finish_json_without_usage_when_loaded_then_usage_absent() {
        let json = serde_json::json!({ "type": "finish", "stop_reason": { "type": "end_turn" } });
        assert_eq!(
            serde_json::from_value::<StreamEvent>(json).unwrap(),
            StreamEvent::Finish {
                stop_reason: StopReason::EndTurn,
                usage: None,
            }
        );
    }
}
