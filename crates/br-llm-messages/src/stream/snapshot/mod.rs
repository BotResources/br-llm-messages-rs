mod block;
mod checkpoint;
mod header;
mod restore;

use std::collections::BTreeMap;

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

    pub fn with_entry_position(mut self, position: u32) -> Self {
        self.entry = self.entry.with_position(position);
        self
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
            DraftPhase::Interrupted(_) => RepresentationState::Interrupted,
            DraftPhase::Completed { .. } => {
                return Err(Error::new(ErrorCode::StreamPhase).with_id(self.entry.as_uuid()));
            }
        };
        let entry = EntryRecord::new(
            self.entry,
            self.turn,
            TurnEntryKind::ModelResponse,
            0,
            state,
            header::write(&self.metadata, &self.phase)?,
        );
        let blocks = self.block_rows(state)?;
        Ok(DraftSnapshot { entry, blocks })
    }

    pub fn from_snapshot(snapshot: &DraftSnapshot) -> Result<ResponseDraft, Error> {
        restore::restore(snapshot)
    }

    fn block_rows(&self, state: RepresentationState) -> Result<Vec<BlockRecord>, Error> {
        let ordered: &BTreeMap<u32, Slot> = &self.slots;
        let mut rows = Vec::with_capacity(ordered.len());
        for (position, slot) in ordered {
            match slot {
                Slot::Open(open) => rows.push(block::open_row(self.entry, *position, state, open)?),
                Slot::Final(done) => block_rows(self.entry, done, *position, &mut rows)?,
                Slot::Discarded { id, kind } => rows.push(BlockRecord::new(
                    *id,
                    self.entry,
                    *kind,
                    *position,
                    RepresentationState::Discarded,
                    OpaqueJson::empty_object(),
                )),
            }
        }
        Ok(rows)
    }
}
