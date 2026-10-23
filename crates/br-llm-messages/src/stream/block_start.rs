use serde::{Deserialize, Serialize};

use crate::block::ContentBlockKind;
use crate::value::{ExecutionStatus, ExternalId, ObservedAt, ProviderOperation, ToolName};

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(from = "BlockStartWire", tag = "type", rename_all = "snake_case")]
pub enum BlockStart {
    #[non_exhaustive]
    Text {
        #[serde(skip_serializing_if = "Option::is_none")]
        observed_at: Option<ObservedAt>,
    },
    #[non_exhaustive]
    Thinking {
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
        observed_at: Option<ObservedAt>,
    },
    #[non_exhaustive]
    ProviderToolCall {
        #[serde(skip_serializing_if = "Option::is_none")]
        operation: Option<ProviderOperation>,
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

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
enum BlockStartWire {
    Text {
        observed_at: Option<ObservedAt>,
    },
    Thinking {
        observed_at: Option<ObservedAt>,
    },
    StructuredContent {
        observed_at: Option<ObservedAt>,
    },
    RuntimeToolCall {
        name: Option<ToolName>,
        external_id: Option<ExternalId>,
        observed_at: Option<ObservedAt>,
    },
    ProviderToolCall {
        operation: Option<ProviderOperation>,
        external_id: Option<ExternalId>,
        observed_at: Option<ObservedAt>,
    },
    ProviderToolExecutionRecord {
        operation: Option<ProviderOperation>,
        external_id: Option<ExternalId>,
        status: Option<ExecutionStatus>,
        observed_at: Option<ObservedAt>,
    },
}

impl From<BlockStartWire> for BlockStart {
    fn from(wire: BlockStartWire) -> Self {
        match wire {
            BlockStartWire::Text { observed_at } => BlockStart::Text { observed_at },
            BlockStartWire::Thinking { observed_at } => BlockStart::Thinking { observed_at },
            BlockStartWire::StructuredContent { observed_at } => {
                BlockStart::StructuredContent { observed_at }
            }
            BlockStartWire::RuntimeToolCall {
                name,
                external_id,
                observed_at,
            } => BlockStart::RuntimeToolCall {
                name,
                external_id,
                observed_at,
            },
            BlockStartWire::ProviderToolCall {
                operation,
                external_id,
                observed_at,
            } => BlockStart::ProviderToolCall {
                operation,
                external_id,
                observed_at,
            },
            BlockStartWire::ProviderToolExecutionRecord {
                operation,
                external_id,
                status,
                observed_at,
            } => BlockStart::ProviderToolExecutionRecord {
                operation,
                external_id,
                status,
                observed_at,
            },
        }
    }
}

impl BlockStart {
    pub fn text() -> Self {
        BlockStart::Text { observed_at: None }
    }

    pub fn thinking() -> Self {
        BlockStart::Thinking { observed_at: None }
    }

    pub fn structured_content() -> Self {
        BlockStart::StructuredContent { observed_at: None }
    }

    pub fn runtime_tool_call(name: Option<ToolName>, external_id: Option<ExternalId>) -> Self {
        BlockStart::RuntimeToolCall {
            name,
            external_id,
            observed_at: None,
        }
    }

    pub fn provider_tool_call(
        operation: Option<ProviderOperation>,
        external_id: Option<ExternalId>,
    ) -> Self {
        BlockStart::ProviderToolCall {
            operation,
            external_id,
            observed_at: None,
        }
    }

    pub fn provider_tool_execution_record(
        operation: Option<ProviderOperation>,
        external_id: Option<ExternalId>,
        status: Option<ExecutionStatus>,
    ) -> Self {
        BlockStart::ProviderToolExecutionRecord {
            operation,
            external_id,
            status,
            observed_at: None,
        }
    }

    pub fn with_observed_at(mut self, value: ObservedAt) -> Self {
        match &mut self {
            BlockStart::Text { observed_at }
            | BlockStart::Thinking { observed_at }
            | BlockStart::StructuredContent { observed_at }
            | BlockStart::RuntimeToolCall { observed_at, .. }
            | BlockStart::ProviderToolCall { observed_at, .. }
            | BlockStart::ProviderToolExecutionRecord { observed_at, .. } => {
                *observed_at = Some(value);
            }
        }
        self
    }

    pub fn observed_at(&self) -> Option<&ObservedAt> {
        match self {
            BlockStart::Text { observed_at }
            | BlockStart::Thinking { observed_at }
            | BlockStart::StructuredContent { observed_at }
            | BlockStart::RuntimeToolCall { observed_at, .. }
            | BlockStart::ProviderToolCall { observed_at, .. }
            | BlockStart::ProviderToolExecutionRecord { observed_at, .. } => observed_at.as_ref(),
        }
    }

    pub fn kind(&self) -> ContentBlockKind {
        match self {
            BlockStart::Text { .. } => ContentBlockKind::Text,
            BlockStart::Thinking { .. } => ContentBlockKind::Thinking,
            BlockStart::StructuredContent { .. } => ContentBlockKind::StructuredContent,
            BlockStart::RuntimeToolCall { .. } => ContentBlockKind::RuntimeToolCall,
            BlockStart::ProviderToolCall { .. } => ContentBlockKind::ProviderToolCall,
            BlockStart::ProviderToolExecutionRecord { .. } => {
                ContentBlockKind::ProviderToolExecutionRecord
            }
        }
    }
}
