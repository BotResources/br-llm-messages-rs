use std::collections::{HashMap, HashSet};

use uuid::Uuid;

use super::entry::EntryRecord;
use super::kinds::RepresentationState;
use super::payload::entry_value;
use super::set::RecordSet;
use super::tree::{BlockTree, order};
use super::turn::TurnRecord;
use crate::entry::{IncomingEntry, TurnEntry};
use crate::error::{Error, ErrorCode};
use crate::id::{EntryId, TurnId};
use crate::turn::{AITurn, InputTurn, Turn, TurnKind};

fn ensure_unique(set: &RecordSet) -> Result<(), Error> {
    let mut seen: HashSet<Uuid> = HashSet::new();
    let ids = set
        .turns()
        .iter()
        .map(|row| row.id().as_uuid())
        .chain(set.entries().iter().map(|row| row.id().as_uuid()))
        .chain(set.blocks().iter().map(|row| row.id().as_uuid()));
    for id in ids {
        if !seen.insert(id) {
            return Err(Error::new(ErrorCode::ReusedIdentity).with_id(id));
        }
    }
    Ok(())
}

fn ensure_complete(set: &RecordSet) -> Result<(), Error> {
    let incomplete = |id: Uuid, location: &str| {
        Error::new(ErrorCode::InvalidRecord)
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
            return Err(Error::new(ErrorCode::UnknownReference)
                .with_id(row.id().as_uuid())
                .at("turn"));
        }
    }
    for row in set.blocks() {
        if !entries.contains(&row.entry()) {
            return Err(Error::new(ErrorCode::UnknownReference)
                .with_id(row.id().as_uuid())
                .at("entry"));
        }
    }
    Ok(())
}

fn build_entry(row: &EntryRecord, tree: &BlockTree<'_>) -> Result<TurnEntry, Error> {
    let mut blocks = Vec::new();
    for block in tree.roots_of(row.id()) {
        blocks.push(tree.value_of(block)?);
    }
    let value = entry_value(row, blocks)?;
    TurnEntry::from_value(value)
}

fn build_turn(
    row: &TurnRecord,
    entries: Vec<&EntryRecord>,
    tree: &BlockTree<'_>,
) -> Result<Turn, Error> {
    let entries = order(
        entries,
        |entry| entry.position(),
        |entry| entry.id().as_uuid(),
        true,
    )?;
    let mut built = Vec::with_capacity(entries.len());
    for (index, entry) in entries.into_iter().enumerate() {
        built
            .push(build_entry(entry, tree).map_err(|error| error.at(format!("entries[{index}]")))?);
    }
    match row.kind() {
        TurnKind::AITurn => {
            let (Some(agent), Some(author)) = (row.agent(), row.author().cloned()) else {
                return Err(Error::new(ErrorCode::MissingContent).with_id(row.id().as_uuid()));
            };
            AITurn::from_parts(row.id(), agent, author, built).map(Turn::AITurn)
        }
        _ => {
            let mut built = built.into_iter();
            let (Some(entry), None) = (built.next(), built.next()) else {
                return Err(Error::new(ErrorCode::InvalidRecord)
                    .with_id(row.id().as_uuid())
                    .at("entries"));
            };
            let entry = IncomingEntry::try_from(entry).map_err(|error| error.at("entries[0]"))?;
            InputTurn::new(row.id(), entry).map(Turn::InputTurn)
        }
    }
}

pub(super) fn rebuild(set: &RecordSet, contiguous: bool) -> Result<Vec<Turn>, Error> {
    set.check_local()?;
    ensure_unique(set)?;
    ensure_parents(set)?;
    ensure_complete(set)?;
    let tree = BlockTree::build(set.blocks())?;
    let mut by_turn: HashMap<TurnId, Vec<&EntryRecord>> = HashMap::new();
    for row in set.entries() {
        by_turn.entry(row.turn()).or_default().push(row);
    }
    let turns = order(
        set.turns().iter().collect(),
        |turn| turn.position(),
        |turn| turn.id().as_uuid(),
        contiguous,
    )?;
    let mut rebuilt = Vec::with_capacity(turns.len());
    for (index, row) in turns.into_iter().enumerate() {
        let entries = by_turn.remove(&row.id()).unwrap_or_default();
        rebuilt.push(
            build_turn(row, entries, &tree).map_err(|error| error.at(format!("turns[{index}]")))?,
        );
    }
    Ok(rebuilt)
}
