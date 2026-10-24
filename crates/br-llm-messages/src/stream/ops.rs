use serde::{Deserialize, Serialize};

use crate::block::ContentBlock;
use crate::error::Error;
use crate::value::{
    Citation, ExecutionStatus, ExternalId, ItemRef, PlainText, ProviderOperation, ToolName,
};

tagged_enum! {
    #[non_exhaustive]
    #[derive(Debug, Clone, PartialEq)]
    pub enum BlockMetadataUpdate, BlockMetadataUpdateWire, remote = "BlockMetadataUpdate", [tag = "type", rename_all = "snake_case", deny_unknown_fields] {
        #[non_exhaustive]
        ToolNameSet { name: ToolName },
        #[non_exhaustive]
        ExternalIdSet { external_id: ExternalId },
        #[non_exhaustive]
        ItemRefSet { item_ref: ItemRef },
        #[non_exhaustive]
        CitationAdded { citation: Citation },
        #[non_exhaustive]
        OperationSet { operation: ProviderOperation },
        #[non_exhaustive]
        ExecutionStatusSet { status: ExecutionStatus },
    }
}

impl BlockMetadataUpdate {
    pub fn tool_name_set(name: ToolName) -> Self {
        BlockMetadataUpdate::ToolNameSet { name }
    }

    pub fn external_id_set(external_id: ExternalId) -> Self {
        BlockMetadataUpdate::ExternalIdSet { external_id }
    }

    pub fn item_ref_set(item_ref: ItemRef) -> Self {
        BlockMetadataUpdate::ItemRefSet { item_ref }
    }

    pub fn citation_added(citation: Citation) -> Self {
        BlockMetadataUpdate::CitationAdded { citation }
    }

    pub fn operation_set(operation: ProviderOperation) -> Self {
        BlockMetadataUpdate::OperationSet { operation }
    }

    pub fn execution_status_set(status: ExecutionStatus) -> Self {
        BlockMetadataUpdate::ExecutionStatusSet { status }
    }
}

tagged_enum! {
    #[non_exhaustive]
    #[derive(Debug, Clone, PartialEq)]
    pub enum Finalization, FinalizationWire, remote = "Finalization", [tag = "type", rename_all = "snake_case", deny_unknown_fields] {
        #[non_exhaustive]
        Accumulated {},
        #[non_exhaustive]
        Authoritative { block: Box<ContentBlock> },
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InterruptionKind {
    Interrupted,
    Cancelled,
    Truncated,
    Failed,
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "InterruptionReasonWire")]
pub struct InterruptionReason {
    kind: InterruptionKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    detail: Option<PlainText>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct InterruptionReasonWire {
    kind: InterruptionKind,
    detail: Option<String>,
}

impl TryFrom<InterruptionReasonWire> for InterruptionReason {
    type Error = Error;

    fn try_from(wire: InterruptionReasonWire) -> Result<Self, Error> {
        let reason = InterruptionReason::new(wire.kind);
        match wire.detail {
            Some(detail) => reason.with_detail(detail),
            None => Ok(reason),
        }
    }
}

impl InterruptionReason {
    pub fn new(kind: InterruptionKind) -> Self {
        InterruptionReason { kind, detail: None }
    }

    pub fn interrupted() -> Self {
        InterruptionReason::new(InterruptionKind::Interrupted)
    }

    pub fn cancelled() -> Self {
        InterruptionReason::new(InterruptionKind::Cancelled)
    }

    pub fn truncated() -> Self {
        InterruptionReason::new(InterruptionKind::Truncated)
    }

    pub fn failed() -> Self {
        InterruptionReason::new(InterruptionKind::Failed)
    }

    pub fn with_detail(mut self, detail: impl Into<String>) -> Result<Self, Error> {
        self.detail = Some(PlainText::new(detail).map_err(|error| error.at("detail"))?);
        Ok(self)
    }

    pub fn kind(&self) -> InterruptionKind {
        self.kind
    }

    pub fn detail(&self) -> Option<&str> {
        self.detail.as_ref().map(PlainText::as_str)
    }
}
