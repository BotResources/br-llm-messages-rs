mod append;
mod build;
mod complete;
mod finalize;
pub(in crate::stream) mod raw_json;
mod replay;
pub(in crate::stream) mod slot;
mod start;
mod view;

use std::collections::{HashMap, HashSet};

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
    Unstarted {},
    #[non_exhaustive]
    Producing {},
    #[non_exhaustive]
    Interrupted { reason: InterruptionReason },
    #[non_exhaustive]
    Completed {},
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub enum DeltaOutcome {
    #[non_exhaustive]
    Applied {},
    #[non_exhaustive]
    Finalized { replaced: bool },
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
    pub(in crate::stream) slots: Vec<Slot>,
    pub(in crate::stream) index: HashMap<BlockId, usize>,
    pub(in crate::stream) ids: HashSet<Uuid>,
    pub(in crate::stream) response: Option<ModelResponse>,
}

fn target_error() -> Error {
    Error::new(ErrorCode::StreamTargetError)
}

fn phase_error() -> Error {
    Error::new(ErrorCode::StreamPhaseError)
}

impl ResponseDraft {
    pub fn new(turn: TurnId, entry: EntryId) -> ResponseDraft {
        ResponseDraft {
            turn,
            entry,
            phase: DraftPhase::Unstarted {},
            metadata: ResponseMetadata::new(),
            slots: Vec::new(),
            index: HashMap::new(),
            ids: HashSet::from([entry.as_uuid()]),
            response: None,
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
        match (&self.phase, delta.op()) {
            (DraftPhase::Unstarted { .. }, DeltaOp::ResponseStarted { metadata }) => {
                self.metadata = metadata.clone();
                self.phase = DraftPhase::Producing {};
                return Ok(DeltaOutcome::Applied {});
            }
            (DraftPhase::Producing { .. }, DeltaOp::ResponseStarted { .. }) => {
                return Err(phase_error());
            }
            (DraftPhase::Producing { .. }, _) => {}
            _ => return Err(phase_error()),
        }
        match delta.op() {
            DeltaOp::ResponseStarted { .. } => Err(phase_error()),
            DeltaOp::BlockStarted { block, start } => self
                .start_block(*block, start)
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
            DeltaOp::BlockInserted { block } => {
                self.insert_block(block).map(|()| DeltaOutcome::Applied {})
            }
            DeltaOp::BlockFinalized {
                block,
                finalization,
            } => self
                .finalize_block(*block, finalization)
                .map(|replaced| DeltaOutcome::Finalized { replaced }),
            DeltaOp::ResponseMetadataUpdated { update } => {
                self.metadata = update.replacing(&self.metadata);
                Ok(DeltaOutcome::Applied {})
            }
            DeltaOp::ResponseCompleted { ending } => {
                self.complete(ending).map(|()| DeltaOutcome::Completed {})
            }
            DeltaOp::ResponseInterrupted { reason } => {
                self.phase = DraftPhase::Interrupted {
                    reason: reason.clone(),
                };
                Ok(DeltaOutcome::Interrupted {})
            }
        }
    }

    fn open_mut(&mut self, block: BlockId) -> Result<&mut OpenBlock, Error> {
        let at = *self.index.get(&block).ok_or_else(target_error)?;
        match self.slots.get_mut(at) {
            Some(Slot::Open(open)) => Ok(open),
            Some(Slot::Final(_)) => Err(phase_error()),
            None => Err(target_error()),
        }
    }
}
