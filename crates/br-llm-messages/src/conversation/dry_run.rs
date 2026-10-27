use super::Conversation;
use crate::entry::{IncomingEntry, TurnEntry};
use crate::error::Error;
use crate::id::TurnId;
use crate::turn::Turn;

impl Conversation {
    pub fn would_refuse_turn(&self, turn: &Turn) -> Option<Error> {
        self.check_turn(turn).err()
    }

    pub fn would_refuse_entry(&self, turn: &TurnId, entry: &TurnEntry) -> Option<Error> {
        self.check_entry(turn, entry).err()
    }

    pub fn would_refuse_incoming(&self, entry: &IncomingEntry, turn: TurnId) -> Option<Error> {
        self.prepare_incoming(entry.clone(), turn).err()
    }

    pub fn check(&self) -> Result<(), Error> {
        Conversation::from_value(self.to_value()?).map(drop)
    }
}
