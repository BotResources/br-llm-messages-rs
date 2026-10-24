use std::collections::{HashMap, HashSet};

use uuid::Uuid;

use super::DraftSnapshot;
use super::block::open_from_row;
use super::header;
use crate::block::{ContentBlock, ContentBlockKind};
use crate::entry::TurnEntryKind;
use crate::error::{Error, ErrorCode};
use crate::id::BlockId;
use crate::record::{BlockRecord, BlockTree, RepresentationState};
use crate::stream::draft::slot::Slot;
use crate::stream::draft::{DraftPhase, ResponseDraft};

fn invalid(row: &BlockRecord, location: &str) -> Error {
    Error::new(ErrorCode::DisallowedKind)
        .with_id(row.id().as_uuid())
        .at(location)
}

fn slot_of(
    row: &BlockRecord,
    tree: &BlockTree<'_>,
    entry_state: RepresentationState,
) -> Result<Slot, Error> {
    match row.state() {
        state @ (RepresentationState::Producing | RepresentationState::Interrupted) => {
            if state != entry_state {
                return Err(invalid(row, "state"));
            }
            open_from_row(row).map(Slot::Open)
        }
        RepresentationState::Complete => {
            if row.kind() == ContentBlockKind::RuntimeToolResult {
                return Err(Error::new(ErrorCode::DisallowedKind)
                    .with_id(row.id().as_uuid())
                    .at("kind"));
            }
            let block = crate::json::decode_value::<ContentBlock>(tree.value_of(row)?).map_err(
                |error| match error.object_id() {
                    Some(_) => error,
                    None => error.with_id(row.id().as_uuid()),
                },
            )?;
            Ok(Slot::Final(block))
        }
    }
}

pub(super) fn restore(snapshot: &DraftSnapshot) -> Result<ResponseDraft, Error> {
    let entry = snapshot.entry();
    entry.check_local()?;
    if entry.kind() != TurnEntryKind::ModelResponse {
        return Err(Error::new(ErrorCode::DisallowedKind)
            .with_id(entry.id().as_uuid())
            .at("kind"));
    }
    let entry_state = entry.state();
    if entry_state == RepresentationState::Complete {
        return Err(Error::new(ErrorCode::StreamPhaseError).with_id(entry.id().as_uuid()));
    }
    let (mut metadata, interruption) = header::read(entry.payload())?;
    if let Some(observed_at) = entry.observed_at() {
        metadata = metadata.with_observed_at(observed_at.clone());
    }
    let phase = match (entry_state, interruption) {
        (RepresentationState::Producing, None) => DraftPhase::Producing {},
        (RepresentationState::Interrupted, Some(reason)) => DraftPhase::Interrupted { reason },
        _ => {
            return Err(Error::new(ErrorCode::InvalidContent)
                .with_id(entry.id().as_uuid())
                .at("payload"));
        }
    };
    for row in snapshot.blocks() {
        row.check_local()?;
        if row.entry() != entry.id() {
            return Err(Error::new(ErrorCode::InvalidRecordStructure)
                .with_id(row.id().as_uuid())
                .at("entry"));
        }
    }
    let tree = BlockTree::build(snapshot.blocks())?;
    let mut slots = Vec::new();
    let mut index: HashMap<BlockId, usize> = HashMap::new();
    let mut ids: HashSet<Uuid> = HashSet::from([entry.id().as_uuid()]);
    for row in tree.roots_of(entry.id()) {
        let slot = slot_of(row, &tree, entry_state)?;
        for id in slot.ids() {
            if !ids.insert(id) {
                return Err(Error::new(ErrorCode::ReusedIdentity).with_id(id));
            }
        }
        index.insert(row.id(), slots.len());
        slots.push(slot);
    }
    Ok(ResponseDraft {
        turn: entry.turn(),
        entry: entry.id(),
        phase,
        metadata,
        slots,
        index,
        ids,
        response: None,
    })
}
