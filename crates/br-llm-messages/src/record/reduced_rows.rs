use std::collections::{HashMap, HashSet};

use uuid::Uuid;

use super::block::BlockRecord;
use super::entry::EntryRecord;
use super::kinds::RepresentationState;
use super::set::RecordSet;
use crate::entry::TurnEntryKind;
use crate::error::{Error, ErrorCode};
use crate::id::{AgentId, EntryId, TurnId};
use crate::turn::TurnKind;

pub(super) struct Rows<'a> {
    pub(super) set: &'a RecordSet,
    pub(super) foreign: HashSet<TurnId>,
    ignored: HashSet<EntryId>,
    pub(super) entry_rows: HashMap<Uuid, usize>,
    pub(super) block_rows: HashMap<Uuid, usize>,
}

fn unique(ids: impl Iterator<Item = Uuid>, rows: &str) -> Result<HashMap<Uuid, usize>, Error> {
    let mut seen = HashMap::new();
    for (index, id) in ids.enumerate() {
        if seen.insert(id, index).is_some() {
            return Err(Error::new(ErrorCode::ReusedIdentity)
                .with_id(id)
                .at(format!("{rows}[{index}]")));
        }
    }
    Ok(seen)
}

fn refused(id: Uuid, location: String) -> Error {
    Error::new(ErrorCode::DisallowedKind)
        .with_id(id)
        .at(location)
}

fn unattached(id: Uuid, location: String) -> Error {
    Error::new(ErrorCode::InvalidRecordStructure)
        .with_id(id)
        .at(location)
}

impl<'a> Rows<'a> {
    pub(super) fn read(set: &'a RecordSet, agent: AgentId) -> Result<Self, Error> {
        for (index, row) in set.turns().iter().enumerate() {
            row.check_local()
                .map_err(|error| error.at(format!("turns[{index}]")))?;
        }
        let turn_rows = unique(set.turns().iter().map(|row| row.id().as_uuid()), "turns")?;
        let entry_rows = unique(
            set.entries().iter().map(|row| row.id().as_uuid()),
            "entries",
        )?;
        let block_rows = unique(set.blocks().iter().map(|row| row.id().as_uuid()), "blocks")?;
        let foreign: HashSet<TurnId> = set
            .turns()
            .iter()
            .filter(|row| {
                row.kind() == TurnKind::AITurn && row.agent().is_some_and(|owner| owner != agent)
            })
            .map(|row| row.id())
            .collect();
        let ignored = set
            .entries()
            .iter()
            .filter(|row| {
                row.state() == RepresentationState::Interrupted
                    && row.kind() == TurnEntryKind::ModelResponse
                    && foreign.contains(&row.turn())
            })
            .map(EntryRecord::id)
            .collect();
        let rows = Rows {
            set,
            foreign,
            ignored,
            entry_rows,
            block_rows,
        };
        rows.check_parents(&turn_rows)?;
        rows.check_rows()?;
        rows.check_states()?;
        Ok(rows)
    }

    fn is_ignored(&self, entry: EntryId) -> bool {
        self.ignored.contains(&entry)
    }

    pub(super) fn entries(&self) -> impl Iterator<Item = (usize, &'a EntryRecord)> + '_ {
        let set = self.set;
        set.entries()
            .iter()
            .enumerate()
            .filter(|(_, row)| !self.is_ignored(row.id()))
    }

    pub(super) fn blocks(&self) -> impl Iterator<Item = (usize, &'a BlockRecord)> + '_ {
        let set = self.set;
        set.blocks()
            .iter()
            .enumerate()
            .filter(|(_, row)| !self.is_ignored(row.entry()))
    }

    pub(super) fn ignored_blocks(
        &self,
        entry: EntryId,
    ) -> impl Iterator<Item = (usize, &'a BlockRecord)> + '_ {
        let set = self.set;
        set.blocks()
            .iter()
            .enumerate()
            .filter(move |(_, row)| row.entry() == entry)
    }

    fn check_parents(&self, turns: &HashMap<Uuid, usize>) -> Result<(), Error> {
        for (index, row) in self.set.entries().iter().enumerate() {
            if !turns.contains_key(&row.turn().as_uuid()) {
                return Err(unattached(
                    row.id().as_uuid(),
                    format!("entries[{index}].turn"),
                ));
            }
        }
        for (index, row) in self.set.blocks().iter().enumerate() {
            if !self.entry_rows.contains_key(&row.entry().as_uuid()) {
                return Err(unattached(
                    row.id().as_uuid(),
                    format!("blocks[{index}].entry"),
                ));
            }
        }
        Ok(())
    }

    fn check_rows(&self) -> Result<(), Error> {
        for (index, row) in self.entries() {
            let checked = if self.foreign.contains(&row.turn()) {
                row.check_reduced()
            } else {
                row.check_local()
            };
            checked.map_err(|error| error.at(format!("entries[{index}]")))?;
        }
        for (index, row) in self.blocks() {
            row.check_local()
                .map_err(|error| error.at(format!("blocks[{index}]")))?;
        }
        Ok(())
    }

    fn check_states(&self) -> Result<(), Error> {
        let mut reduced: HashMap<EntryId, RepresentationState> = HashMap::new();
        for (index, row) in self.entries() {
            let foreign = self.foreign.contains(&row.turn());
            if foreign {
                reduced.insert(row.id(), row.state());
            }
            let partial_allowed = foreign && row.kind() == TurnEntryKind::ModelResponse;
            if !partial_allowed && row.state() != RepresentationState::Complete {
                return Err(refused(
                    row.id().as_uuid(),
                    format!("entries[{index}].state"),
                ));
            }
        }
        for (index, row) in self.blocks() {
            let owner = reduced.get(&row.entry());
            let allowed = row.state() == RepresentationState::Complete
                || owner.is_some_and(|state| row.state() == *state);
            if !allowed {
                return Err(refused(
                    row.id().as_uuid(),
                    format!("blocks[{index}].state"),
                ));
            }
            let usable_root = row.kind().is_plain() && row.parent().is_none();
            if owner.is_some() && !usable_root {
                return Err(refused(row.id().as_uuid(), format!("blocks[{index}]")));
            }
        }
        Ok(())
    }
}
