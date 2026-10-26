use std::collections::HashSet;

use super::ProjectedThread;
use crate::entry::IncomingEntry;
use crate::error::{Error, ErrorCode};
use crate::id::{AgentId, ConversationId};
use crate::turn::Turn;

impl ProjectedThread {
    pub(crate) fn assemble(
        conversation: ConversationId,
        agent: AgentId,
        turns: Vec<Turn>,
    ) -> Result<Self, Error> {
        let mut seen = HashSet::from([conversation.as_uuid()]);
        for (position, turn) in turns.iter().enumerate() {
            let located = |error: Error| error.at(format!("turns[{position}]"));
            check_turn(agent, turn).map_err(located)?;
            for (id, _) in turn.declared() {
                if !seen.insert(id) {
                    return Err(located(Error::new(ErrorCode::ReusedIdentity).with_id(id)));
                }
            }
        }
        Ok(ProjectedThread::unchecked(conversation, agent, turns))
    }
}

fn check_turn(agent: AgentId, turn: &Turn) -> Result<(), Error> {
    match turn {
        Turn::AITurn(ai) if ai.agent() != agent => {
            Err(Error::new(ErrorCode::DisallowedKind).with_id(ai.id().as_uuid()))
        }
        Turn::AITurn(_) => Ok(()),
        Turn::InputTurn(input) => {
            check_shape(input.entries())?;
            for (index, id, owner) in input.outputs() {
                if owner == agent {
                    return Err(Error::new(ErrorCode::DisallowedKind)
                        .with_id(id.as_uuid())
                        .at(format!("entries[{index}]")));
                }
            }
            Ok(())
        }
    }
}

fn check_shape(entries: &[IncomingEntry]) -> Result<(), Error> {
    let projected = entries
        .iter()
        .filter(|entry| entry.projected_source().is_some())
        .count();
    let structure = |index: usize| {
        Error::new(ErrorCode::InvalidRecordStructure)
            .with_id(entries[index].id().as_uuid())
            .at(format!("entries[{index}]"))
    };
    if projected != 0 && projected != entries.len() {
        let first_recorded = entries
            .iter()
            .position(|entry| entry.projected_source().is_none())
            .unwrap_or(0);
        return Err(structure(first_recorded));
    }
    let source = entries
        .first()
        .and_then(|entry| Some((entry.projected_source()?.1, entry.projected_author()?)));
    if let Some((agent, author)) = source {
        let foreign = entries.iter().position(|entry| {
            entry.projected_source().map(|(_, owner)| owner) != Some(agent)
                || entry.projected_author() != Some(author)
        });
        if let Some(index) = foreign {
            return Err(structure(index));
        }
    }
    let partials: Vec<usize> = entries
        .iter()
        .enumerate()
        .filter(|(_, entry)| matches!(entry, IncomingEntry::PartialAgentOutput(_)))
        .map(|(index, _)| index)
        .collect();
    match partials.as_slice() {
        [] => Ok(()),
        [only] if *only + 1 == entries.len() => Ok(()),
        [only] => Err(structure(*only)),
        [_, second, ..] => Err(structure(*second)),
    }
}
