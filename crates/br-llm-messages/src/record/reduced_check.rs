use std::collections::{HashMap, HashSet};

use super::kept::Kept;
use super::kinds::RepresentationState;
use super::set::RecordSet;
use super::turn::TurnRecord;
use crate::entry::TurnEntryKind;
use crate::error::{Error, ErrorCode};
use crate::id::{AgentId, EntryId, TurnId};
use crate::turn::TurnKind;

pub(super) type Owners = HashMap<EntryId, (TurnId, RepresentationState)>;

pub(super) fn other_turns(set: &RecordSet, agent: AgentId) -> HashSet<TurnId> {
    set.turns()
        .iter()
        .filter(|row| {
            row.kind() == TurnKind::AITurn && row.agent().is_some_and(|owner| owner != agent)
        })
        .map(TurnRecord::id)
        .collect()
}

pub(super) fn check_rows(kept: &Kept<'_>, others: &HashSet<TurnId>) -> Result<(), Error> {
    for (index, row) in kept.turns().iter().enumerate() {
        row.check_local()
            .map_err(|error| error.at(format!("turns[{index}]")))?;
    }
    for (index, row) in kept.entries() {
        let checked = if others.contains(&row.turn()) {
            row.check_reduced()
        } else {
            row.check_local()
        };
        checked.map_err(|error| error.at(format!("entries[{index}]")))?;
    }
    for (index, row) in kept.blocks() {
        row.check_local()
            .map_err(|error| error.at(format!("blocks[{index}]")))?;
    }
    Ok(())
}

fn refused(id: uuid::Uuid, location: String) -> Error {
    Error::new(ErrorCode::DisallowedKind)
        .with_id(id)
        .at(location)
}

pub(super) fn check_states(
    kept: &Kept<'_>,
    others: &HashSet<TurnId>,
    owners: &Owners,
) -> Result<(), Error> {
    for (index, row) in kept.entries() {
        let foreign_response =
            others.contains(&row.turn()) && row.kind() == TurnEntryKind::ModelResponse;
        if !foreign_response && row.state() != RepresentationState::Complete {
            return Err(refused(
                row.id().as_uuid(),
                format!("entries[{index}].state"),
            ));
        }
    }
    for (index, row) in kept.blocks() {
        let reduced = owners
            .get(&row.entry())
            .filter(|(turn, _)| others.contains(turn));
        let allowed = match reduced {
            Some((_, state)) => {
                row.state() == RepresentationState::Complete || row.state() == *state
            }
            None => row.state() == RepresentationState::Complete,
        };
        if !allowed {
            return Err(refused(
                row.id().as_uuid(),
                format!("blocks[{index}].state"),
            ));
        }
        let usable_root = row.kind().is_plain() && row.parent().is_none();
        if reduced.is_some() && !usable_root {
            return Err(refused(row.id().as_uuid(), format!("blocks[{index}]")));
        }
    }
    Ok(())
}

pub(super) fn interrupted_foreign_responses(
    set: &RecordSet,
    others: &HashSet<TurnId>,
) -> HashSet<EntryId> {
    set.entries()
        .iter()
        .filter(|row| {
            row.state() == RepresentationState::Interrupted
                && row.kind() == TurnEntryKind::ModelResponse
                && others.contains(&row.turn())
        })
        .map(|row| row.id())
        .collect()
}
