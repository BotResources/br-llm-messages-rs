use super::ProjectedThread;
use super::validate::check_input;
use crate::conversation::extended_input;
use crate::entry::{IncomingEntry, TurnEntry};
use crate::error::{Error, ErrorCode};
use crate::id::TurnId;
use crate::turn::{Class, Declared, InputTurn, Turn};

enum Addition {
    Incoming(IncomingEntry),
    Recorded(TurnEntry),
}

enum Prepared {
    Input {
        position: usize,
        turn: InputTurn,
        declared: Declared,
    },
    Ai {
        position: usize,
        entry: TurnEntry,
        declared: Declared,
    },
}

impl ProjectedThread {
    pub fn push_turn(&mut self, turn: Turn) -> Result<(), Error> {
        let declared = self.check_turn(&turn)?;
        let position = self.turns.len();
        self.turns.push(turn);
        self.index.insert(position, declared);
        Ok(())
    }

    pub fn push_incoming(
        &mut self,
        turn: &TurnId,
        entry: impl Into<IncomingEntry>,
    ) -> Result<(), Error> {
        let prepared = self.prepare(turn, Addition::Incoming(entry.into()))?;
        self.commit(prepared);
        Ok(())
    }

    pub fn push_entry(&mut self, turn: &TurnId, entry: TurnEntry) -> Result<(), Error> {
        let prepared = self.prepare(turn, Addition::Recorded(entry))?;
        self.commit(prepared);
        Ok(())
    }

    pub fn would_refuse_turn(&self, turn: &Turn) -> Option<Error> {
        self.check_turn(turn).err()
    }

    pub fn would_refuse_incoming(&self, turn: &TurnId, entry: &IncomingEntry) -> Option<Error> {
        self.prepare(turn, Addition::Incoming(entry.clone())).err()
    }

    pub fn would_refuse_entry(&self, turn: &TurnId, entry: &TurnEntry) -> Option<Error> {
        self.prepare(turn, Addition::Recorded(entry.clone())).err()
    }

    pub fn check(&self) -> Result<(), Error> {
        ProjectedThread::from_value(self.to_value()?).map(drop)
    }

    fn prepare(&self, turn: &TurnId, addition: Addition) -> Result<Prepared, Error> {
        let unknown = || Error::new(ErrorCode::UnknownReference).with_id(turn.as_uuid());
        let position = match self.index.get(turn.as_uuid()) {
            Some(slot) if slot.place.class == Class::Turn => slot.turn,
            _ => return Err(unknown()),
        };
        match (self.turns.get(position).ok_or_else(unknown)?, addition) {
            (Turn::InputTurn(input), Addition::Incoming(entry)) => {
                self.prepare_input(position, input, entry)
            }
            (Turn::InputTurn(input), Addition::Recorded(entry)) => {
                self.prepare_input(position, input, IncomingEntry::try_from(entry)?)
            }
            (Turn::AITurn(ai), Addition::Recorded(entry)) => {
                let declared = ai.check_push(&entry)?;
                self.refuse_clash(&declared, ai.entries().len())?;
                Ok(Prepared::Ai {
                    position,
                    entry,
                    declared,
                })
            }
            (Turn::AITurn(ai), Addition::Incoming(entry)) => {
                let entry = TurnEntry::try_from(entry)?;
                let declared = ai.check_push(&entry)?;
                self.refuse_clash(&declared, ai.entries().len())?;
                Ok(Prepared::Ai {
                    position,
                    entry,
                    declared,
                })
            }
        }
    }

    fn prepare_input(
        &self,
        position: usize,
        input: &InputTurn,
        entry: IncomingEntry,
    ) -> Result<Prepared, Error> {
        let at = input.entries().len();
        let (turn, declared) = extended_input(input, entry, true)?;
        check_input(self.agent, &turn)?;
        self.refuse_clash(&declared, at)?;
        Ok(Prepared::Input {
            position,
            turn,
            declared,
        })
    }

    fn refuse_clash(&self, declared: &Declared, entry: usize) -> Result<(), Error> {
        match self.index.clash(declared) {
            Some((id, _)) => Err(Error::new(ErrorCode::ReusedIdentity)
                .with_id(id)
                .at(format!("entries[{entry}]"))),
            None => Ok(()),
        }
    }

    fn commit(&mut self, prepared: Prepared) {
        match prepared {
            Prepared::Input {
                position,
                turn,
                declared,
            } => {
                self.turns[position] = Turn::InputTurn(turn);
                self.index.insert(position, declared);
            }
            Prepared::Ai {
                position,
                entry,
                declared,
            } => {
                if let Some(Turn::AITurn(target)) = self.turns.get_mut(position) {
                    target.commit_push(entry, declared.clone());
                    self.index.insert(position, declared);
                }
            }
        }
    }
}
