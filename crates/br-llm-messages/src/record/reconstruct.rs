use std::collections::{HashMap, HashSet};

use serde_json::Value;
use uuid::Uuid;

use super::block::BlockRecord;
use super::entry::EntryRecord;
use super::kinds::RepresentationState;
use super::payload::entry_value;
use super::set::RecordSet;
use super::tree::BlockTree;
use super::turn::TurnRecord;
use crate::block::ContentBlockKind;
use crate::entry::{IncomingEntry, TurnEntry};
use crate::error::{Error, ErrorCode};
use crate::id::{CitationId, EntryId, TurnId};
use crate::turn::{AITurn, InputTurn, Turn, TurnKind};

fn citation_ids(row: &BlockRecord) -> Vec<Uuid> {
    if row.kind() != ContentBlockKind::Text {
        return Vec::new();
    }
    row.payload()
        .inner()
        .get("citations")
        .and_then(Value::as_array)
        .map(|citations| {
            citations
                .iter()
                .filter_map(|citation| citation.get("id"))
                .filter_map(|id| serde_json::from_value::<CitationId>(id.clone()).ok())
                .map(|id| id.as_uuid())
                .collect()
        })
        .unwrap_or_default()
}

fn ids_of(set: &RecordSet) -> impl Iterator<Item = Uuid> + '_ {
    set.turns()
        .iter()
        .map(|row| row.id().as_uuid())
        .chain(set.entries().iter().map(|row| row.id().as_uuid()))
        .chain(set.blocks().iter().map(|row| row.id().as_uuid()))
        .chain(set.blocks().iter().flat_map(citation_ids))
}

fn ensure_unique(set: &RecordSet) -> Result<(), Error> {
    let mut seen: HashSet<Uuid> = HashSet::new();
    for id in ids_of(set) {
        if !seen.insert(id) {
            return Err(Error::new(ErrorCode::ReusedIdentity).with_id(id));
        }
    }
    Ok(())
}

fn ensure_complete(set: &RecordSet) -> Result<(), Error> {
    let incomplete = |id: Uuid, location: &str| {
        Error::new(ErrorCode::DisallowedKind)
            .with_id(id)
            .at(location)
    };
    for row in set.entries() {
        if row.state() != RepresentationState::Complete {
            return Err(incomplete(row.id().as_uuid(), "state"));
        }
    }
    for row in set.blocks() {
        if row.state() != RepresentationState::Complete {
            return Err(incomplete(row.id().as_uuid(), "state"));
        }
    }
    Ok(())
}

fn ensure_parents(set: &RecordSet) -> Result<(), Error> {
    let turns: HashSet<TurnId> = set.turns().iter().map(TurnRecord::id).collect();
    let entries: HashSet<EntryId> = set.entries().iter().map(EntryRecord::id).collect();
    for row in set.entries() {
        if !turns.contains(&row.turn()) {
            return Err(Error::new(ErrorCode::InvalidRecordStructure)
                .with_id(row.id().as_uuid())
                .at("turn"));
        }
    }
    for row in set.blocks() {
        if !entries.contains(&row.entry()) {
            return Err(Error::new(ErrorCode::InvalidRecordStructure)
                .with_id(row.id().as_uuid())
                .at("entry"));
        }
    }
    Ok(())
}

pub(super) fn build_entry(row: &EntryRecord, tree: &BlockTree<'_>) -> Result<TurnEntry, Error> {
    let mut blocks = Vec::new();
    for block in tree.roots_of(row.id()) {
        blocks.push(tree.value_of(block)?);
    }
    let value = entry_value(row, blocks)?;
    crate::json::decode_value(value)
}

pub(super) fn build_turn(
    row: &TurnRecord,
    entries: Vec<&EntryRecord>,
    tree: &BlockTree<'_>,
) -> Result<Turn, Error> {
    let mut built = Vec::with_capacity(entries.len());
    for (index, entry) in entries.into_iter().enumerate() {
        built
            .push(build_entry(entry, tree).map_err(|error| error.at(format!("entries[{index}]")))?);
    }
    match row.kind() {
        TurnKind::AITurn => {
            let (Some(agent), Some(author)) = (row.agent(), row.author().cloned()) else {
                return Err(Error::new(ErrorCode::InvalidContent).with_id(row.id().as_uuid()));
            };
            let turn = AITurn::from_parts(row.id(), agent, author, built)?;
            Ok(Turn::AITurn(match row.observed_at() {
                Some(observed_at) => turn.with_observed_at(observed_at.clone()),
                None => turn,
            }))
        }
        TurnKind::InputTurn => {
            if built.is_empty() {
                return Err(Error::new(ErrorCode::InvalidRecordStructure)
                    .with_id(row.id().as_uuid())
                    .at("entries"));
            }
            let mut incoming = Vec::with_capacity(built.len());
            for (index, entry) in built.into_iter().enumerate() {
                incoming.push(
                    IncomingEntry::try_from(entry)
                        .map_err(|error| error.at(format!("entries[{index}]")))?,
                );
            }
            let turn = InputTurn::new(row.id(), incoming)?;
            Ok(Turn::InputTurn(match row.observed_at() {
                Some(observed_at) => turn.with_observed_at(observed_at.clone()),
                None => turn,
            }))
        }
    }
}

fn entries_by_turn(set: &RecordSet) -> HashMap<TurnId, Vec<&EntryRecord>> {
    let mut by_turn: HashMap<TurnId, Vec<&EntryRecord>> = HashMap::new();
    for row in set.entries() {
        by_turn.entry(row.turn()).or_default().push(row);
    }
    by_turn
}

pub(super) fn rebuild(set: &RecordSet) -> Result<Vec<Turn>, Error> {
    set.check_local()?;
    ensure_unique(set)?;
    ensure_parents(set)?;
    ensure_complete(set)?;
    let tree = BlockTree::build(set.blocks())?;
    let mut by_turn = entries_by_turn(set);
    let mut rebuilt = Vec::with_capacity(set.turns().len());
    for (index, row) in set.turns().iter().enumerate() {
        let entries = by_turn.remove(&row.id()).unwrap_or_default();
        rebuilt.push(
            build_turn(row, entries, &tree).map_err(|error| error.at(format!("turns[{index}]")))?,
        );
    }
    Ok(rebuilt)
}
