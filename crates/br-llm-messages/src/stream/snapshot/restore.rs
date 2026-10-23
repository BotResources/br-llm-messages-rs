use std::collections::{BTreeMap, HashMap, HashSet};

use uuid::Uuid;

use super::DraftSnapshot;
use super::block::open_from_row;
use super::header;
use crate::block::{ContentBlock, ContentBlockKind};
use crate::entry::TurnEntryKind;
use crate::error::{Error, ErrorCode};
use crate::id::BlockId;
use crate::record::{BlockRecord, BlockTree, RepresentationState};
use crate::stream::draft::slot::{Slot, block_ids};
use crate::stream::draft::{DraftPhase, ResponseDraft};

fn invalid(row: &BlockRecord, location: &str) -> Error {
    Error::new(ErrorCode::InvalidRecord)
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
                return Err(Error::new(ErrorCode::DisallowedBlockKind)
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
        _ => Ok(Slot::Discarded {
            id: row.id(),
            kind: row.kind(),
        }),
    }
}

fn slot_ids(slot: &Slot) -> Vec<Uuid> {
    match slot {
        Slot::Open(open) => open.ids(),
        Slot::Final(block) => block_ids(block),
        Slot::Discarded { id, .. } => vec![id.as_uuid()],
    }
}

pub(super) fn restore(snapshot: &DraftSnapshot) -> Result<ResponseDraft, Error> {
    let entry = snapshot.entry();
    entry.check_local()?;
    if entry.kind() != TurnEntryKind::ModelResponse {
        return Err(Error::new(ErrorCode::InvalidRecord)
            .with_id(entry.id().as_uuid())
            .at("kind"));
    }
    let entry_state = entry.state();
    if entry_state == RepresentationState::Complete {
        return Err(Error::new(ErrorCode::StreamPhase).with_id(entry.id().as_uuid()));
    }
    let (metadata, interruption) = header::read(entry.payload())?;
    let phase = match (entry_state, interruption) {
        (RepresentationState::Producing, None) => DraftPhase::Producing {},
        (RepresentationState::Interrupted, Some(reason)) => DraftPhase::Interrupted(reason),
        _ => {
            return Err(Error::new(ErrorCode::InvalidRecord)
                .with_id(entry.id().as_uuid())
                .at("payload"));
        }
    };
    for row in snapshot.blocks() {
        row.check_local()?;
        if row.entry() != entry.id() {
            return Err(Error::new(ErrorCode::UnknownReference)
                .with_id(row.id().as_uuid())
                .at("entry"));
        }
    }
    let tree = BlockTree::build(snapshot.blocks())?;
    let mut slots = BTreeMap::new();
    let mut positions: HashMap<BlockId, u32> = HashMap::new();
    let mut ids: HashSet<Uuid> = HashSet::from([entry.id().as_uuid()]);
    for row in tree.roots_of(entry.id()) {
        let slot = slot_of(row, &tree, entry_state)?;
        for id in slot_ids(&slot) {
            if !ids.insert(id) {
                return Err(Error::new(ErrorCode::ReusedIdentity).with_id(id));
            }
        }
        positions.insert(row.id(), row.position());
        slots.insert(row.position(), slot);
    }
    Ok(ResponseDraft {
        turn: entry.turn(),
        entry: entry.id(),
        phase,
        metadata,
        slots,
        positions,
        ids,
        response: None,
    })
}
