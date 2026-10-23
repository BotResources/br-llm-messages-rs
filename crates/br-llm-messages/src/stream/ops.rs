use serde::{Deserialize, Serialize};

use crate::block::{ContentBlock, ContentBlockKind};
use crate::value::{
    Citation, ExecutionStatus, ExternalId, FailureDetail, ProviderOperation, ToolName,
};

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(from = "BlockStartWire", tag = "type", rename_all = "snake_case")]
pub enum BlockStart {
    #[non_exhaustive]
    Text {},
    #[non_exhaustive]
    Thinking {},
    #[non_exhaustive]
    StructuredContent {},
    #[non_exhaustive]
    RuntimeToolCall {
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<ToolName>,
        #[serde(skip_serializing_if = "Option::is_none")]
        external_id: Option<ExternalId>,
    },
    #[non_exhaustive]
    ProviderToolCall {
        #[serde(skip_serializing_if = "Option::is_none")]
        operation: Option<ProviderOperation>,
        #[serde(skip_serializing_if = "Option::is_none")]
        external_id: Option<ExternalId>,
    },
    #[non_exhaustive]
    ProviderToolExecutionRecord {
        #[serde(skip_serializing_if = "Option::is_none")]
        operation: Option<ProviderOperation>,
        #[serde(skip_serializing_if = "Option::is_none")]
        external_id: Option<ExternalId>,
        #[serde(skip_serializing_if = "Option::is_none")]
        status: Option<ExecutionStatus>,
    },
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
enum BlockStartWire {
    Text {},
    Thinking {},
    StructuredContent {},
    RuntimeToolCall {
        name: Option<ToolName>,
        external_id: Option<ExternalId>,
    },
    ProviderToolCall {
        operation: Option<ProviderOperation>,
        external_id: Option<ExternalId>,
    },
    ProviderToolExecutionRecord {
        operation: Option<ProviderOperation>,
        external_id: Option<ExternalId>,
        status: Option<ExecutionStatus>,
    },
}

impl From<BlockStartWire> for BlockStart {
    fn from(wire: BlockStartWire) -> Self {
        match wire {
            BlockStartWire::Text {} => BlockStart::Text {},
            BlockStartWire::Thinking {} => BlockStart::Thinking {},
            BlockStartWire::StructuredContent {} => BlockStart::StructuredContent {},
            BlockStartWire::RuntimeToolCall { name, external_id } => {
                BlockStart::RuntimeToolCall { name, external_id }
            }
            BlockStartWire::ProviderToolCall {
                operation,
                external_id,
            } => BlockStart::ProviderToolCall {
                operation,
                external_id,
            },
            BlockStartWire::ProviderToolExecutionRecord {
                operation,
                external_id,
                status,
            } => BlockStart::ProviderToolExecutionRecord {
                operation,
                external_id,
                status,
            },
        }
    }
}

impl BlockStart {
    pub fn text() -> Self {
        BlockStart::Text {}
    }

    pub fn thinking() -> Self {
        BlockStart::Thinking {}
    }

    pub fn structured_content() -> Self {
        BlockStart::StructuredContent {}
    }

    pub fn runtime_tool_call(name: Option<ToolName>, external_id: Option<ExternalId>) -> Self {
        BlockStart::RuntimeToolCall { name, external_id }
    }

    pub fn provider_tool_call(
        operation: Option<ProviderOperation>,
        external_id: Option<ExternalId>,
    ) -> Self {
        BlockStart::ProviderToolCall {
            operation,
            external_id,
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

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum BlockMetadataUpdate {
    #[non_exhaustive]
    CitationAdded { citation: Citation },
    #[non_exhaustive]
    RuntimeToolCallIdentified {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<ToolName>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        external_id: Option<ExternalId>,
    },
    #[non_exhaustive]
    ProviderReferencesUpdated {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        operation: Option<ProviderOperation>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        external_id: Option<ExternalId>,
    },
    #[non_exhaustive]
    ExecutionStatusObserved { status: ExecutionStatus },
}

impl BlockMetadataUpdate {
    pub fn citation_added(citation: Citation) -> Self {
        BlockMetadataUpdate::CitationAdded { citation }
    }

    pub fn runtime_tool_call_identified(
        name: Option<ToolName>,
        external_id: Option<ExternalId>,
    ) -> Self {
        BlockMetadataUpdate::RuntimeToolCallIdentified { name, external_id }
    }

    pub fn provider_references_updated(
        operation: Option<ProviderOperation>,
        external_id: Option<ExternalId>,
    ) -> Self {
        BlockMetadataUpdate::ProviderReferencesUpdated {
            operation,
            external_id,
        }
    }

    pub fn execution_status_observed(status: ExecutionStatus) -> Self {
        BlockMetadataUpdate::ExecutionStatusObserved { status }
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(from = "FinalizationWire", tag = "type", rename_all = "snake_case")]
pub enum Finalization {
    #[non_exhaustive]
    Accumulated {},
    #[non_exhaustive]
    Authoritative { block: Box<ContentBlock> },
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
enum FinalizationWire {
    Accumulated {},
    Authoritative { block: Box<ContentBlock> },
}

impl From<FinalizationWire> for Finalization {
    fn from(wire: FinalizationWire) -> Self {
        match wire {
            FinalizationWire::Accumulated {} => Finalization::Accumulated {},
            FinalizationWire::Authoritative { block } => Finalization::Authoritative { block },
        }
    }
}

impl Finalization {
    pub fn accumulated() -> Self {
        Finalization::Accumulated {}
    }

    pub fn authoritative(block: ContentBlock) -> Self {
        Finalization::Authoritative {
            block: Box::new(block),
        }
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    from = "InterruptionReasonWire",
    tag = "type",
    rename_all = "snake_case"
)]
pub enum InterruptionReason {
    #[non_exhaustive]
    Cancelled {},
    #[non_exhaustive]
    Truncated {},
    #[non_exhaustive]
    Interrupted {},
    #[non_exhaustive]
    Failed { failure: FailureDetail },
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
enum InterruptionReasonWire {
    Cancelled {},
    Truncated {},
    Interrupted {},
    Failed { failure: FailureDetail },
}

impl From<InterruptionReasonWire> for InterruptionReason {
    fn from(wire: InterruptionReasonWire) -> Self {
        match wire {
            InterruptionReasonWire::Cancelled {} => InterruptionReason::cancelled(),
            InterruptionReasonWire::Truncated {} => InterruptionReason::truncated(),
            InterruptionReasonWire::Interrupted {} => InterruptionReason::interrupted(),
            InterruptionReasonWire::Failed { failure } => InterruptionReason::Failed { failure },
        }
    }
}

impl InterruptionReason {
    pub fn cancelled() -> Self {
        InterruptionReason::Cancelled {}
    }

    pub fn truncated() -> Self {
        InterruptionReason::Truncated {}
    }

    pub fn interrupted() -> Self {
        InterruptionReason::Interrupted {}
    }

    pub fn failed(failure: FailureDetail) -> Self {
        InterruptionReason::Failed { failure }
    }
}
