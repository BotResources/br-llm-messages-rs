use super::Conversation;
use crate::entry::{IncomingEntry, TurnEntry};
use crate::error::Error;
use crate::id::TurnId;
use crate::turn::{Declared, InputTurn, Turn};

pub(crate) enum Prepared {
    NewTurn {
        turn: Turn,
        declared: Declared,
    },
    Extend {
        position: usize,
        entry: IncomingEntry,
        declared: Declared,
    },
    Open {
        position: usize,
        entry: TurnEntry,
        declared: Declared,
    },
}

impl Conversation {
    pub fn push_incoming(
        &mut self,
        entry: impl Into<IncomingEntry>,
        turn: TurnId,
    ) -> Result<(), Error> {
        match self.prepare_incoming(entry.into(), turn)? {
            Prepared::NewTurn { turn, declared } => self.commit_turn(turn, declared),
            Prepared::Extend {
                position,
                entry,
                declared,
            } => self.commit_incoming(position, entry, declared),
            Prepared::Open {
                position,
                entry,
                declared,
            } => self.commit_entry(position, entry, declared),
        }
        Ok(())
    }

    pub(crate) fn prepare_incoming(
        &self,
        entry: IncomingEntry,
        new_turn: TurnId,
    ) -> Result<Prepared, Error> {
        match self.turns.last() {
            Some(Turn::InputTurn(input)) => {
                let position = self.turns.len() - 1;
                let declared = self.admit_to_input(input, &entry)?;
                Ok(Prepared::Extend {
                    position,
                    entry,
                    declared,
                })
            }
            Some(Turn::AITurn(ai)) if !ai.is_closed() => {
                let entry = TurnEntry::try_from(entry)?;
                let (position, declared) = self.check_entry(&ai.id(), &entry)?;
                Ok(Prepared::Open {
                    position,
                    entry,
                    declared,
                })
            }
            _ => {
                let turn = Turn::InputTurn(InputTurn::new(new_turn, vec![entry])?);
                let declared = self.check_turn(&turn)?;
                Ok(Prepared::NewTurn { turn, declared })
            }
        }
    }
}
