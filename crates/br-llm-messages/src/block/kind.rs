use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::error::{Error, ErrorCode};

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContentBlockKind {
    Text,
    Image,
    StructuredContent,
    Thinking,
    RedactedThinking,
    RuntimeToolCall,
    RuntimeToolResult,
    ProviderToolCall,
    ProviderToolResult,
    ProviderToolExecutionRecord,
}

impl ContentBlockKind {
    pub const ALL: &'static [ContentBlockKind] = &[
        ContentBlockKind::Text,
        ContentBlockKind::Image,
        ContentBlockKind::StructuredContent,
        ContentBlockKind::Thinking,
        ContentBlockKind::RedactedThinking,
        ContentBlockKind::RuntimeToolCall,
        ContentBlockKind::RuntimeToolResult,
        ContentBlockKind::ProviderToolCall,
        ContentBlockKind::ProviderToolResult,
        ContentBlockKind::ProviderToolExecutionRecord,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            ContentBlockKind::Text => "text",
            ContentBlockKind::Image => "image",
            ContentBlockKind::StructuredContent => "structured_content",
            ContentBlockKind::Thinking => "thinking",
            ContentBlockKind::RedactedThinking => "redacted_thinking",
            ContentBlockKind::RuntimeToolCall => "runtime_tool_call",
            ContentBlockKind::RuntimeToolResult => "runtime_tool_result",
            ContentBlockKind::ProviderToolCall => "provider_tool_call",
            ContentBlockKind::ProviderToolResult => "provider_tool_result",
            ContentBlockKind::ProviderToolExecutionRecord => "provider_tool_execution_record",
        }
    }

    pub fn is_plain(self) -> bool {
        matches!(
            self,
            ContentBlockKind::Text | ContentBlockKind::Image | ContentBlockKind::StructuredContent
        )
    }
}

impl FromStr for ContentBlockKind {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self, Error> {
        ContentBlockKind::ALL
            .iter()
            .copied()
            .find(|kind| kind.as_str() == value)
            .ok_or_else(|| Error::new(ErrorCode::VariantMismatch))
    }
}

impl fmt::Display for ContentBlockKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
