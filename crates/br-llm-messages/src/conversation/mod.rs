mod cut;
mod dry_run;
mod identity;
mod incoming;
mod index;
mod json;
mod lookup;
mod view;

pub use cut::Position;
pub use json::CONVERSATION_SCHEMA;
pub use lookup::{BlockRef, EntryRef};
pub use view::{EntryView, PlacedEntry};

pub(crate) use index::Index;
pub(crate) use lookup::Lookup;
pub(crate) use view::placed_entries;

use crate::entry::{IncomingEntry, TurnEntry};
use crate::error::{Error, ErrorCode};
use crate::id::{ConversationId, TurnId};
use crate::turn::{Class, Declared, InputTurn, Turn, refuse_agent_output};

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
        let declared = self.check_turn(&turn)?;
        self.commit_turn(turn, declared);
        Ok(())
    }

    pub(crate) fn commit_turn(&mut self, turn: Turn, declared: Declared) {
        let position = self.turns.len();
        self.turns.push(turn);
        self.index.insert(position, declared);
    }

    pub(crate) fn check_turn(&self, turn: &Turn) -> Result<Declared, Error> {
        if let Turn::InputTurn(input) = turn {
            refuse_agent_output(input.entries())?;
        }
        let declared = turn.declared();
        if let Some((id, place)) = self.index.clash(&declared) {
            let clash = Error::new(ErrorCode::ReusedIdentity).with_id(id);
            return Err(match (turn, place.class) {
                (_, Class::Turn) => clash,
                _ => clash.at(format!("entries[{}]", place.entry)),
            });
        }
        Ok(declared)
    }

    pub fn push_entry(&mut self, turn: &TurnId, entry: TurnEntry) -> Result<(), Error> {
        let (position, declared) = self.check_entry(turn, &entry)?;
        self.commit_entry(position, entry, declared);
        Ok(())
    }

    pub(crate) fn check_entry(
        &self,
        turn: &TurnId,
        entry: &TurnEntry,
    ) -> Result<(usize, Declared), Error> {
        let reference = |code| Error::new(code).with_id(turn.as_uuid());
        let position = match self.index.get(turn.as_uuid()) {
            None => return Err(reference(ErrorCode::UnknownReference)),
            Some(slot) if slot.place.class == Class::Turn => slot.turn,
            Some(_) => return Err(reference(ErrorCode::UnknownReference)),
        };
        match self.turns.get(position) {
            Some(Turn::AITurn(target)) => {
                let declared = target.checked_push(entry)?;
                match self.index.clash(&declared) {
                    Some((id, _)) => Err(Error::new(ErrorCode::ReusedIdentity).with_id(id)),
                    None => Ok((position, declared)),
                }
            }
            Some(Turn::InputTurn(target)) => {
                let declared = self.admit_to_input(target, &incoming_of(entry)?)?;
                Ok((position, declared))
            }
            None => Err(reference(ErrorCode::UnknownReference)),
        }
    }

    pub(crate) fn admit_to_input(
        &self,
        target: &InputTurn,
        entry: &IncomingEntry,
    ) -> Result<Declared, Error> {
        let declared = target.checked_push(entry)?;
        match self.index.clash(&declared) {
            Some((id, _)) => Err(Error::new(ErrorCode::ReusedIdentity)
                .with_id(id)
                .at(format!("entries[{}]", target.entries().len()))),
            None => Ok(declared),
        }
    }

    pub(crate) fn commit_incoming(
        &mut self,
        position: usize,
        entry: IncomingEntry,
        declared: Declared,
    ) {
        if let Some(Turn::InputTurn(target)) = self.turns.get_mut(position) {
            target.commit_push(entry);
            self.index.insert(position, declared);
        }
    }

    pub(crate) fn commit_entry(&mut self, position: usize, entry: TurnEntry, declared: Declared) {
        match self.turns.get_mut(position) {
            Some(Turn::AITurn(target)) => target.commit_push(entry, declared.clone()),
            Some(Turn::InputTurn(target)) => match incoming_of(&entry) {
                Ok(incoming) => target.commit_push(incoming),
                Err(_) => return,
            },
            None => return,
        }
        self.index.insert(position, declared);
    }
}

fn incoming_of(entry: &TurnEntry) -> Result<IncomingEntry, Error> {
    IncomingEntry::recorded(entry)
        .ok_or_else(|| Error::new(ErrorCode::DisallowedKind).with_id(entry.id().as_uuid()))
}
