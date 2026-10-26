use std::collections::HashSet;

use super::ProjectedThread;
use crate::entry::IncomingEntry;
use crate::error::{Error, ErrorCode};
use crate::id::{AgentId, ConversationId};
use crate::turn::{Class, Turn};

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
            for (id, place) in turn.declared() {
                if !seen.insert(id) {
                    let clash = Error::new(ErrorCode::ReusedIdentity).with_id(id);
                    return Err(located(match place.class {
                        Class::Turn => clash,
                        _ => clash.at(format!("entries[{}]", place.entry)),
                    }));
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
    let mut outputs = entries.iter().enumerate().filter_map(|(index, entry)| {
        Some((
            index,
            entry.projected_source()?.1,
            entry.projected_author()?,
        ))
    });
    let Some((_, agent, author)) = outputs.next() else {
        return Ok(());
    };
    match outputs.find(|(_, other, label)| *other != agent || *label != author) {
        Some((index, _, _)) => Err(Error::new(ErrorCode::InvalidRecordStructure)
            .with_id(entries[index].id().as_uuid())
            .at(format!("entries[{index}]"))),
        None => Ok(()),
    }
}
