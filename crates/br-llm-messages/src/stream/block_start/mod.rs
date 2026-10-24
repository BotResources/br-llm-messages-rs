mod builders;
mod wire;

use serde::{Deserialize, Serialize};

use crate::block::ContentBlockKind;
use crate::id::BlockId;
use crate::opaque_json::OpaqueJson;
use crate::value::{ExecutionStatus, ExternalId, ItemRef, ObservedAt, ProviderOperation, ToolName};

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    try_from = "wire::BlockStartWire",
    tag = "type",
    rename_all = "snake_case"
)]
pub enum BlockStart {
    #[non_exhaustive]
    Text {
        #[serde(skip_serializing_if = "Option::is_none")]
        item_ref: Option<ItemRef>,
        #[serde(skip_serializing_if = "Option::is_none")]
        observed_at: Option<ObservedAt>,
    },
    #[non_exhaustive]
    Thinking {
        #[serde(skip_serializing_if = "Option::is_none")]
        observed_at: Option<ObservedAt>,
    },
    #[non_exhaustive]
    RedactedThinking {
        #[serde(skip_serializing_if = "Option::is_none")]
        observed_at: Option<ObservedAt>,
    },
    #[non_exhaustive]
    StructuredContent {
        #[serde(skip_serializing_if = "Option::is_none")]
        observed_at: Option<ObservedAt>,
    },
    #[non_exhaustive]
    RuntimeToolCall {
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<ToolName>,
        #[serde(skip_serializing_if = "Option::is_none")]
        external_id: Option<ExternalId>,
        #[serde(skip_serializing_if = "Option::is_none")]
        item_ref: Option<ItemRef>,
        #[serde(skip_serializing_if = "Option::is_none")]
        arguments: Option<OpaqueJson>,
        #[serde(skip_serializing_if = "Option::is_none")]
        observed_at: Option<ObservedAt>,
    },
    #[non_exhaustive]
    ProviderToolCall {
        #[serde(skip_serializing_if = "Option::is_none")]
        operation: Option<ProviderOperation>,
        #[serde(skip_serializing_if = "Option::is_none")]
        external_id: Option<ExternalId>,
        #[serde(skip_serializing_if = "Option::is_none")]
        item_ref: Option<ItemRef>,
        #[serde(skip_serializing_if = "Option::is_none")]
        arguments: Option<OpaqueJson>,
        #[serde(skip_serializing_if = "Option::is_none")]
        observed_at: Option<ObservedAt>,
    },
    #[non_exhaustive]
    ProviderToolResult {
        call: BlockId,
        #[serde(skip_serializing_if = "Option::is_none")]
        external_id: Option<ExternalId>,
        #[serde(skip_serializing_if = "Option::is_none")]
        observed_at: Option<ObservedAt>,
    },
    #[non_exhaustive]
    ProviderToolExecutionRecord {
        #[serde(skip_serializing_if = "Option::is_none")]
        operation: Option<ProviderOperation>,
        #[serde(skip_serializing_if = "Option::is_none")]
        external_id: Option<ExternalId>,
        #[serde(skip_serializing_if = "Option::is_none")]
        status: Option<ExecutionStatus>,
        #[serde(skip_serializing_if = "Option::is_none")]
        observed_at: Option<ObservedAt>,
    },
}

impl BlockStart {
    pub fn kind(&self) -> ContentBlockKind {
        match self {
            BlockStart::Text { .. } => ContentBlockKind::Text,
            BlockStart::Thinking { .. } => ContentBlockKind::Thinking,
            BlockStart::RedactedThinking { .. } => ContentBlockKind::RedactedThinking,
            BlockStart::StructuredContent { .. } => ContentBlockKind::StructuredContent,
            BlockStart::RuntimeToolCall { .. } => ContentBlockKind::RuntimeToolCall,
            BlockStart::ProviderToolCall { .. } => ContentBlockKind::ProviderToolCall,
            BlockStart::ProviderToolResult { .. } => ContentBlockKind::ProviderToolResult,
            BlockStart::ProviderToolExecutionRecord { .. } => {
                ContentBlockKind::ProviderToolExecutionRecord
            }
        }
    }

    pub fn observed_at(&self) -> Option<&ObservedAt> {
        match self {
            BlockStart::Text { observed_at, .. }
            | BlockStart::Thinking { observed_at }
            | BlockStart::RedactedThinking { observed_at }
            | BlockStart::StructuredContent { observed_at }
            | BlockStart::RuntimeToolCall { observed_at, .. }
            | BlockStart::ProviderToolCall { observed_at, .. }
            | BlockStart::ProviderToolResult { observed_at, .. }
            | BlockStart::ProviderToolExecutionRecord { observed_at, .. } => observed_at.as_ref(),
        }
    }
}
