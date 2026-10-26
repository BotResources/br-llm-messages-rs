mod index;
mod json;
mod lookup;

pub use json::CONVERSATION_SCHEMA;
pub use lookup::{BlockRef, EntryRef};

use index::Index;

use crate::entry::TurnEntry;
use crate::error::{Error, ErrorCode};
use crate::id::{ConversationId, TurnId};
use crate::turn::{Class, Declared, Turn};

#[derive(Debug, Clone, PartialEq)]
pub struct Conversation {
    id: ConversationId,
    turns: Vec<Turn>,
    index: Index,
}

impl Conversation {
    pub fn new(id: ConversationId) -> Self {
        Conversation {
            id,
            turns: Vec::new(),
            index: Index::seeded(id),
        }
    }

    pub fn from_parts(id: ConversationId, turns: Vec<Turn>) -> Result<Self, Error> {
        let mut conversation = Conversation::new(id);
        for (position, turn) in turns.into_iter().enumerate() {
            conversation
                .push_turn(turn)
                .map_err(|error| error.at(format!("turns[{position}]")))?;
        }
        Ok(conversation)
    }

    pub fn id(&self) -> ConversationId {
        self.id
    }

    pub fn turns(&self) -> &[Turn] {
        &self.turns
    }

    pub fn push_turn(&mut self, turn: Turn) -> Result<(), Error> {
        let declared = declared_by(&turn);
        if let Some((id, place)) = self.index.clash(&declared) {
            let clash = Error::new(ErrorCode::ReusedIdentity).with_id(id);
            return Err(match (&turn, place.class) {
                (_, Class::Turn) => clash,
                _ => clash.at(format!("entries[{}]", place.entry)),
            });
        }
        let position = self.turns.len();
        self.turns.push(turn);
        self.index.insert(position, declared);
        Ok(())
    }

    pub fn push_entry(&mut self, turn: &TurnId, entry: TurnEntry) -> Result<(), Error> {
        let reference = |code| Error::new(code).with_id(turn.as_uuid());
        let position = match self.index.get(turn.as_uuid()) {
            None => return Err(reference(ErrorCode::UnknownReference)),
            Some(slot) if slot.place.class == Class::Turn => slot.turn,
            Some(_) => return Err(reference(ErrorCode::UnknownReference)),
        };
        let Some(Turn::AITurn(target)) = self.turns.get_mut(position) else {
            return Err(reference(ErrorCode::UnknownReference));
        };
        let declared = target.check_push(&entry)?;
        if let Some((id, _)) = self.index.clash(&declared) {
            return Err(Error::new(ErrorCode::ReusedIdentity).with_id(id));
        }
        target.commit_push(entry, declared.clone());
        self.index.insert(position, declared);
        Ok(())
    }
}

fn declared_by(turn: &Turn) -> Declared {
    match turn {
        Turn::InputTurn(input) => input.declared(),
        Turn::AITurn(ai) => ai.declared(),
    }
}
