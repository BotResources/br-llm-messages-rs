mod append;
mod complete;
mod finalize;
mod replay;
pub(in crate::stream) mod slot;
mod start;
mod view;

use std::collections::{BTreeMap, HashMap, HashSet};

use uuid::Uuid;

pub use view::{DraftView, SlotLifecycle, SlotView};

use self::slot::{OpenBlock, Slot};
use super::delta::{DeltaOp, ResponseDelta};
use super::ops::InterruptionReason;
use crate::entry::{ModelResponse, ResponseMetadata};
use crate::error::{Error, ErrorCode};
use crate::id::{BlockId, EntryId, TurnId};

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub enum DraftPhase {
    #[non_exhaustive]
    Producing {},
    Interrupted(InterruptionReason),
    #[non_exhaustive]
    Completed {},
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub enum DeltaOutcome {
    #[non_exhaustive]
    Applied {},
    #[non_exhaustive]
    BlockDiscarded { block: BlockId, position: u32 },
    #[non_exhaustive]
    Completed {},
    #[non_exhaustive]
    Interrupted {},
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResponseDraft {
    pub(in crate::stream) turn: TurnId,
    pub(in crate::stream) entry: EntryId,
    pub(in crate::stream) phase: DraftPhase,
    pub(in crate::stream) metadata: ResponseMetadata,
    pub(in crate::stream) slots: BTreeMap<u32, Slot>,
    pub(in crate::stream) positions: HashMap<BlockId, u32>,
    pub(in crate::stream) ids: HashSet<Uuid>,
    pub(in crate::stream) response: Option<ModelResponse>,
}

fn target_error() -> Error {
    Error::new(ErrorCode::StreamTarget)
}

fn phase_error() -> Error {
    Error::new(ErrorCode::StreamPhase)
}

impl ResponseDraft {
    pub fn begin(first: &ResponseDelta) -> Result<ResponseDraft, Error> {
        match first.op() {
            DeltaOp::ResponseStarted { metadata } => Ok(ResponseDraft {
                turn: first.turn(),
                entry: first.entry(),
                phase: DraftPhase::Producing {},
                metadata: metadata.clone(),
                slots: BTreeMap::new(),
                positions: HashMap::new(),
                ids: HashSet::from([first.entry().as_uuid()]),
                response: None,
            }),
            _ => Err(phase_error().with_id(first.entry().as_uuid())),
        }
    }

    pub fn apply(&mut self, delta: &ResponseDelta) -> Result<DeltaOutcome, Error> {
        let target = delta
            .op()
            .block_id()
            .map_or(delta.entry().as_uuid(), |block| block.as_uuid());
        self.dispatch(delta).map_err(|error| error.with_id(target))
    }

    pub fn turn(&self) -> TurnId {
        self.turn
    }

    pub fn entry(&self) -> EntryId {
        self.entry
    }

    pub fn phase(&self) -> &DraftPhase {
        &self.phase
    }

    pub fn metadata(&self) -> &ResponseMetadata {
        &self.metadata
    }

    pub fn response(&self) -> Option<&ModelResponse> {
        self.response.as_ref()
    }

    pub fn into_response(self) -> Option<ModelResponse> {
        self.response
    }

    pub fn view(&self) -> DraftView<'_> {
        DraftView::new(self)
    }

    fn dispatch(&mut self, delta: &ResponseDelta) -> Result<DeltaOutcome, Error> {
        if delta.turn() != self.turn || delta.entry() != self.entry {
            return Err(target_error());
        }
        if !matches!(self.phase, DraftPhase::Producing { .. }) {
            return Err(phase_error());
        }
        match delta.op() {
            DeltaOp::ResponseStarted { .. } => Err(phase_error()),
            DeltaOp::BlockStarted {
                block,
                position,
                start,
            } => self
                .start_block(*block, *position, start)
                .map(|()| DeltaOutcome::Applied {}),
            DeltaOp::TextAppended { block, text } => self
                .append_text(*block, text)
                .map(|()| DeltaOutcome::Applied {}),
            DeltaOp::ThinkingAppended { block, text } => self
                .append_thinking(*block, text)
                .map(|()| DeltaOutcome::Applied {}),
            DeltaOp::JsonAppended { block, fragment } => self
                .append_json(*block, fragment)
                .map(|()| DeltaOutcome::Applied {}),
            DeltaOp::ReplayFragmentAppended {
                block,
                format,
                fragment,
            } => self
                .append_replay(*block, format, fragment)
                .map(|()| DeltaOutcome::Applied {}),
            DeltaOp::ReplayMaterialSet { block, material } => self
                .set_replay(*block, material)
                .map(|()| DeltaOutcome::Applied {}),
            DeltaOp::BlockMetadataUpdated { block, update } => self
                .update_block(*block, update)
                .map(|()| DeltaOutcome::Applied {}),
            DeltaOp::BlockInserted { position, block } => self
                .insert_block(*position, block)
                .map(|()| DeltaOutcome::Applied {}),
            DeltaOp::BlockFinalized {
                block,
                finalization,
            } => self
                .finalize_block(*block, finalization)
                .map(|()| DeltaOutcome::Applied {}),
            DeltaOp::BlockDiscarded { block } => {
                self.discard_block(*block)
                    .map(|position| DeltaOutcome::BlockDiscarded {
                        block: *block,
                        position,
                    })
            }
            DeltaOp::ResponseMetadataUpdated { update } => {
                self.metadata = update.replacing(&self.metadata);
                Ok(DeltaOutcome::Applied {})
            }
            DeltaOp::ResponseCompleted { ending } => {
                self.complete(ending).map(|()| DeltaOutcome::Completed {})
            }
            DeltaOp::ResponseInterrupted { reason } => {
                self.phase = DraftPhase::Interrupted(reason.clone());
                Ok(DeltaOutcome::Interrupted {})
            }
        }
    }

    fn open_mut(&mut self, block: BlockId) -> Result<&mut OpenBlock, Error> {
        let position = *self.positions.get(&block).ok_or_else(target_error)?;
        match self.slots.get_mut(&position) {
            Some(Slot::Open(open)) => Ok(open),
            Some(_) => Err(phase_error()),
            None => Err(target_error()),
        }
    }
}
