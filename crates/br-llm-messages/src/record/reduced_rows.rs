use std::collections::{HashMap, HashSet};

use uuid::Uuid;

use super::block::BlockRecord;
use super::entry::EntryRecord;
use super::kinds::RepresentationState;
use super::recovery::Unfinished;
use super::set::RecordSet;
use super::turn::TurnRecord;
use crate::entry::TurnEntryKind;
use crate::error::{Error, ErrorCode};
use crate::id::{AgentId, EntryId, TurnId};
use crate::projection::Owner;
use crate::turn::TurnKind;

pub(super) struct Rows<'a> {
    pub(super) set: &'a RecordSet,
    pub(super) foreign: HashMap<TurnId, Owner>,
    pub(super) ignored: HashMap<EntryId, Vec<(usize, &'a BlockRecord)>>,
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
    pub(super) fn read(
        set: &'a RecordSet,
        agent: AgentId,
        unfinished: Option<Unfinished>,
    ) -> Result<Self, Error> {
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
        let foreign: HashMap<TurnId, Owner> = set
            .turns()
            .iter()
            .filter(|row| row.kind() == TurnKind::AITurn)
            .filter_map(|row| {
                let agent = row.agent().filter(|owner| *owner != agent)?;
                let author = row.author()?.clone();
                Some((row.id(), Owner { agent, author }))
            })
            .collect();
        let own: HashSet<TurnId> = set
            .turns()
            .iter()
            .filter(|row| row.kind() == TurnKind::AITurn && row.agent() == Some(agent))
            .map(TurnRecord::id)
            .collect();
        let mut ignored: HashMap<EntryId, Vec<(usize, &'a BlockRecord)>> = set
            .entries()
            .iter()
            .filter(|row| row.kind() == TurnEntryKind::ModelResponse)
            .filter(|row| {
                let state = row.state();
                if foreign.contains_key(&row.turn()) {
                    state == RepresentationState::Interrupted
                        || (state == RepresentationState::Producing
                            && unfinished == Some(Unfinished::Drop))
                } else {
                    own.contains(&row.turn())
                        && unfinished.is_some()
                        && state != RepresentationState::Complete
                }
            })
            .map(|row| (row.id(), Vec::new()))
            .collect();
        for (index, row) in set.blocks().iter().enumerate() {
            if let Some(rows) = ignored.get_mut(&row.entry()) {
                rows.push((index, row));
            }
        }
        let rows = Rows {
            set,
            foreign,
            ignored,
            entry_rows,
            block_rows,
        };
        rows.check_parents(&turn_rows)?;
        rows.check_own_unfinished()?;
        rows.check_rows()?;
        rows.check_states()?;
        Ok(rows)
    }

    fn check_own_unfinished(&self) -> Result<(), Error> {
        for (index, row) in self.set.entries().iter().enumerate() {
            if self.is_ignored(row.id()) && !self.foreign.contains_key(&row.turn()) {
                row.check_local()
                    .map_err(|error| error.at(format!("entries[{index}]")))?;
            }
        }
        for (index, row) in self.set.blocks().iter().enumerate() {
            let owner = self
                .entry_rows
                .get(&row.entry().as_uuid())
                .map(|position| &self.set.entries()[*position]);
            if self.is_ignored(row.entry())
                && owner.is_some_and(|entry| !self.foreign.contains_key(&entry.turn()))
            {
                row.check_local()
                    .map_err(|error| error.at(format!("blocks[{index}]")))?;
            }
        }
        Ok(())
    }

    pub(super) fn is_ignored(&self, entry: EntryId) -> bool {
        self.ignored.contains_key(&entry)
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
            let checked = if self.foreign.contains_key(&row.turn()) {
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
            let foreign = self.foreign.contains_key(&row.turn());
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
