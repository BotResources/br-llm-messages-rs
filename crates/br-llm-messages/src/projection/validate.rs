use super::ProjectedThread;
use crate::entry::IncomingEntry;
use crate::error::{Error, ErrorCode};
use crate::id::{AgentId, ConversationId};
use crate::turn::{Class, Declared, InputTurn, Turn};

impl ProjectedThread {
    pub fn from_turns(
        conversation: ConversationId,
        agent: AgentId,
        turns: Vec<Turn>,
    ) -> Result<Self, Error> {
        let mut thread = ProjectedThread::unchecked(conversation, agent, Vec::new());
        for (position, turn) in turns.into_iter().enumerate() {
            thread
                .push_turn(turn)
                .map_err(|error| error.at(format!("turns[{position}]")))?;
        }
        Ok(thread)
    }

    pub(crate) fn check_turn(&self, turn: &Turn) -> Result<Declared, Error> {
        check_member(self.agent, turn)?;
        let declared = turn.declared();
        if let Some((id, place)) = self.index.clash(&declared) {
            let clash = Error::new(ErrorCode::ReusedIdentity).with_id(id);
            return Err(match place.class {
                Class::Turn => clash,
                _ => clash.at(format!("entries[{}]", place.entry)),
            });
        }
        Ok(declared)
    }
}

pub(super) fn check_member(agent: AgentId, turn: &Turn) -> Result<(), Error> {
    match turn {
        Turn::AITurn(ai) if ai.agent() != agent => {
            Err(Error::new(ErrorCode::DisallowedKind).with_id(ai.id().as_uuid()))
        }
        Turn::AITurn(_) => Ok(()),
        Turn::InputTurn(input) => check_input(agent, input),
    }
}

pub(super) fn check_input(agent: AgentId, input: &InputTurn) -> Result<(), Error> {
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

fn check_shape(entries: &[IncomingEntry]) -> Result<(), Error> {
    let mut outputs = entries.iter().enumerate().filter_map(|(index, entry)| {
        let projected = entry.projected()?;
        Some((index, projected.agent, projected.author))
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
