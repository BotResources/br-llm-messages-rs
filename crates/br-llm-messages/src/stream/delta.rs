use serde::{Deserialize, Serialize};

use super::block_start::BlockStart;
use super::kind::{DeltaTarget, ResponseDeltaKind};
use super::metadata::ResponseMetadataUpdate;
use super::ops::{BlockMetadataUpdate, Finalization, InterruptionReason};
use crate::block::ContentBlock;
use crate::entry::ResponseMetadata;
use crate::id::{BlockId, EntryId, TurnId};
use crate::value::{ReplayFormat, ReplayMaterial, ResponseEnding};

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum DeltaOp {
    #[non_exhaustive]
    ResponseStarted { metadata: ResponseMetadata },
    #[non_exhaustive]
    BlockStarted {
        block: BlockId,
        position: u32,
        start: BlockStart,
    },
    #[non_exhaustive]
    TextAppended { block: BlockId, text: String },
    #[non_exhaustive]
    ThinkingAppended { block: BlockId, text: String },
    #[non_exhaustive]
    JsonAppended { block: BlockId, fragment: String },
    #[non_exhaustive]
    ReplayFragmentAppended {
        block: BlockId,
        format: ReplayFormat,
        fragment: String,
    },
    #[non_exhaustive]
    ReplayMaterialSet {
        block: BlockId,
        material: ReplayMaterial,
    },
    #[non_exhaustive]
    BlockMetadataUpdated {
        block: BlockId,
        update: BlockMetadataUpdate,
    },
    #[non_exhaustive]
    BlockInserted { position: u32, block: ContentBlock },
    #[non_exhaustive]
    BlockFinalized {
        block: BlockId,
        finalization: Finalization,
    },
    #[non_exhaustive]
    BlockDiscarded { block: BlockId },
    #[non_exhaustive]
    ResponseMetadataUpdated { update: ResponseMetadataUpdate },
    #[non_exhaustive]
    ResponseCompleted { ending: ResponseEnding },
    #[non_exhaustive]
    ResponseInterrupted { reason: InterruptionReason },
}

impl DeltaOp {
    pub fn response_started(metadata: ResponseMetadata) -> Self {
        DeltaOp::ResponseStarted { metadata }
    }

    pub fn block_started(block: BlockId, position: u32, start: BlockStart) -> Self {
        DeltaOp::BlockStarted {
            block,
            position,
            start,
        }
    }

    pub fn text_appended(block: BlockId, text: impl Into<String>) -> Self {
        DeltaOp::TextAppended {
            block,
            text: text.into(),
        }
    }

    pub fn thinking_appended(block: BlockId, text: impl Into<String>) -> Self {
        DeltaOp::ThinkingAppended {
            block,
            text: text.into(),
        }
    }

    pub fn json_appended(block: BlockId, fragment: impl Into<String>) -> Self {
        DeltaOp::JsonAppended {
            block,
            fragment: fragment.into(),
        }
    }

    pub fn replay_fragment_appended(
        block: BlockId,
        format: ReplayFormat,
        fragment: impl Into<String>,
    ) -> Self {
        DeltaOp::ReplayFragmentAppended {
            block,
            format,
            fragment: fragment.into(),
        }
    }

    pub fn replay_material_set(block: BlockId, material: ReplayMaterial) -> Self {
        DeltaOp::ReplayMaterialSet { block, material }
    }

    pub fn block_metadata_updated(block: BlockId, update: BlockMetadataUpdate) -> Self {
        DeltaOp::BlockMetadataUpdated { block, update }
    }

    pub fn block_inserted(position: u32, block: impl Into<ContentBlock>) -> Self {
        DeltaOp::BlockInserted {
            position,
            block: block.into(),
        }
    }

    pub fn block_finalized(block: BlockId, finalization: Finalization) -> Self {
        DeltaOp::BlockFinalized {
            block,
            finalization,
        }
    }

    pub fn block_discarded(block: BlockId) -> Self {
        DeltaOp::BlockDiscarded { block }
    }

    pub fn response_metadata_updated(update: ResponseMetadataUpdate) -> Self {
        DeltaOp::ResponseMetadataUpdated { update }
    }

    pub fn response_completed(ending: ResponseEnding) -> Self {
        DeltaOp::ResponseCompleted { ending }
    }

    pub fn response_interrupted(reason: InterruptionReason) -> Self {
        DeltaOp::ResponseInterrupted { reason }
    }

    pub fn kind(&self) -> ResponseDeltaKind {
        match self {
            DeltaOp::ResponseStarted { .. } => ResponseDeltaKind::ResponseStarted,
            DeltaOp::BlockStarted { .. } => ResponseDeltaKind::BlockStarted,
            DeltaOp::TextAppended { .. } => ResponseDeltaKind::TextAppended,
            DeltaOp::ThinkingAppended { .. } => ResponseDeltaKind::ThinkingAppended,
            DeltaOp::JsonAppended { .. } => ResponseDeltaKind::JsonAppended,
            DeltaOp::ReplayFragmentAppended { .. } => ResponseDeltaKind::ReplayFragmentAppended,
            DeltaOp::ReplayMaterialSet { .. } => ResponseDeltaKind::ReplayMaterialSet,
            DeltaOp::BlockMetadataUpdated { .. } => ResponseDeltaKind::BlockMetadataUpdated,
            DeltaOp::BlockInserted { .. } => ResponseDeltaKind::BlockInserted,
            DeltaOp::BlockFinalized { .. } => ResponseDeltaKind::BlockFinalized,
            DeltaOp::BlockDiscarded { .. } => ResponseDeltaKind::BlockDiscarded,
            DeltaOp::ResponseMetadataUpdated { .. } => ResponseDeltaKind::ResponseMetadataUpdated,
            DeltaOp::ResponseCompleted { .. } => ResponseDeltaKind::ResponseCompleted,
            DeltaOp::ResponseInterrupted { .. } => ResponseDeltaKind::ResponseInterrupted,
        }
    }

    pub fn target(&self) -> DeltaTarget {
        self.kind().target()
    }

    pub fn block_id(&self) -> Option<BlockId> {
        match self {
            DeltaOp::BlockStarted { block, .. }
            | DeltaOp::TextAppended { block, .. }
            | DeltaOp::ThinkingAppended { block, .. }
            | DeltaOp::JsonAppended { block, .. }
            | DeltaOp::ReplayFragmentAppended { block, .. }
            | DeltaOp::ReplayMaterialSet { block, .. }
            | DeltaOp::BlockMetadataUpdated { block, .. }
            | DeltaOp::BlockFinalized { block, .. }
            | DeltaOp::BlockDiscarded { block } => Some(*block),
            DeltaOp::BlockInserted { block, .. } => Some(block.id()),
            _ => None,
        }
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub struct ResponseDelta {
    turn: TurnId,
    entry: EntryId,
    op: DeltaOp,
}

impl ResponseDelta {
    pub fn new(turn: TurnId, entry: EntryId, op: DeltaOp) -> Self {
        ResponseDelta { turn, entry, op }
    }

    pub fn turn(&self) -> TurnId {
        self.turn
    }

    pub fn entry(&self) -> EntryId {
        self.entry
    }

    pub fn op(&self) -> &DeltaOp {
        &self.op
    }

    pub fn kind(&self) -> ResponseDeltaKind {
        self.op.kind()
    }

    pub fn target(&self) -> DeltaTarget {
        self.op.target()
    }
}
