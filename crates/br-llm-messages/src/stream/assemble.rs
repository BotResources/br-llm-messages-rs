use serde_json::{Map, Value};

use crate::block::{AssistantBlock, RedactedThinking, Thinking, ToolCall};
use crate::error::MessageError;
use crate::value::{Signature, Text, is_blank};

use super::draft::DraftBlock;

/// Assembles a finished draft block into an assistant block. A blank text
/// block assembles to nothing: providers send them as normal output, so the
/// step is built from the other blocks.
pub(super) fn assemble(block: DraftBlock) -> Result<Option<AssistantBlock>, MessageError> {
    let assembled = match block {
        DraftBlock::Text { text } if is_blank(&text) => return Ok(None),
        DraftBlock::Text { text } => AssistantBlock::Text {
            text: Text::new(text)?,
        },
        DraftBlock::Thinking { text, signature } => AssistantBlock::Thinking(Thinking {
            text,
            signature: signature.map(Signature::new).transpose()?,
        }),
        DraftBlock::RedactedThinking { data } => {
            AssistantBlock::RedactedThinking(RedactedThinking::new(data)?)
        }
        DraftBlock::Structured { json } => AssistantBlock::Structured {
            value: parse_json(&json, "structured")?,
        },
        DraftBlock::ToolCall {
            id,
            name,
            arguments,
        } => AssistantBlock::ToolCall(ToolCall {
            id,
            name,
            arguments: parse_arguments(&arguments)?,
        }),
    };
    Ok(Some(assembled))
}

fn parse_json(fragment: &str, field: &'static str) -> Result<Value, MessageError> {
    serde_json::from_str(fragment).map_err(|error| MessageError::InvalidJson {
        field,
        message: error.to_string(),
    })
}

fn parse_arguments(fragment: &str) -> Result<Value, MessageError> {
    if fragment.trim().is_empty() {
        return Ok(Value::Object(Map::new()));
    }
    parse_json(fragment, "tool_arguments")
}
