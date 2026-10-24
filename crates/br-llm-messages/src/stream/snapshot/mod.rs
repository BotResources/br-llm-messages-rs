mod block;
mod checkpoint;
mod header;
mod restore;

use crate::opaque_json::OpaqueJson;

use super::draft::slot::Slot;
use super::draft::{DraftPhase, ResponseDraft};
use crate::entry::TurnEntryKind;
use crate::error::{Error, ErrorCode};
use crate::id::TurnId;
use crate::record::{BlockRecord, EntryRecord, RepresentationState, block_rows};

pub const DRAFT_SCHEMA: &str = "br-llm-messages-draft/1";

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub struct DraftSnapshot {
    entry: EntryRecord,
    blocks: Vec<BlockRecord>,
}

impl DraftSnapshot {
    pub fn new(entry: EntryRecord, blocks: Vec<BlockRecord>) -> Result<DraftSnapshot, Error> {
        let snapshot = DraftSnapshot { entry, blocks };
        restore::restore(&snapshot)?;
        Ok(snapshot)
    }

    pub fn schema() -> &'static str {
        DRAFT_SCHEMA
    }

    pub fn turn(&self) -> TurnId {
        self.entry.turn()
    }

    pub fn entry(&self) -> &EntryRecord {
        &self.entry
    }

    pub fn blocks(&self) -> &[BlockRecord] {
        &self.blocks
    }

    pub fn to_json(&self) -> Result<String, Error> {
        crate::json::encode(self)
    }

    pub fn to_value(&self) -> Result<OpaqueJson, Error> {
        crate::json::encode_value(self).map(OpaqueJson::from_inner)
    }

    pub fn from_json(text: &str) -> Result<DraftSnapshot, Error> {
        DraftSnapshot::from_value(OpaqueJson::from_inner(crate::json::parse(text)?))
    }

    pub fn from_value(value: OpaqueJson) -> Result<DraftSnapshot, Error> {
        let value = value.into_inner();
        checkpoint::from_value(value)
    }
}

impl ResponseDraft {
    pub fn to_snapshot(&self) -> Result<DraftSnapshot, Error> {
        let state = match &self.phase {
            DraftPhase::Producing { .. } => RepresentationState::Producing,
            DraftPhase::Interrupted { .. } => RepresentationState::Interrupted,
            DraftPhase::Unstarted { .. } | DraftPhase::Completed { .. } => {
                return Err(Error::new(ErrorCode::StreamPhaseError).with_id(self.entry.as_uuid()));
            }
        };
        let mut entry = EntryRecord::new(
            self.entry,
            self.turn,
            TurnEntryKind::ModelResponse,
            state,
            header::write(&self.metadata, &self.phase)?,
        );
        if let Some(observed_at) = self.metadata.observed_at() {
            entry = entry.with_observed_at(observed_at.clone());
        }
        let blocks = self.block_rows(state)?;
        Ok(DraftSnapshot { entry, blocks })
    }

    pub fn from_snapshot(snapshot: &DraftSnapshot) -> Result<ResponseDraft, Error> {
        restore::restore(snapshot)
    }

    fn block_rows(&self, state: RepresentationState) -> Result<Vec<BlockRecord>, Error> {
        let mut rows = Vec::with_capacity(self.slots.len());
        for slot in &self.slots {
            match slot {
                Slot::Open(open) => rows.push(block::open_row(self.entry, state, open)?),
                Slot::Final(done) => block_rows(self.entry, done, &mut rows)?,
            }
        }
        Ok(rows)
    }
}
