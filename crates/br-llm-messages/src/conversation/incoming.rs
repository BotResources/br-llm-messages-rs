use super::Conversation;
use crate::entry::{IncomingEntry, TurnEntry};
use crate::error::{Error, ErrorCode};
use crate::id::TurnId;
use crate::turn::{Declared, InputTurn, Turn, places_of_incoming};

pub(crate) enum Prepared {
    NewTurn {
        turn: Turn,
        declared: Declared,
    },
    Extend {
        position: usize,
        turn: InputTurn,
        declared: Declared,
    },
    Open {
        position: usize,
        entry: TurnEntry,
        declared: Declared,
    },
}

pub(crate) fn extended_input(
    input: &InputTurn,
    entry: IncomingEntry,
    projected: bool,
) -> Result<(InputTurn, Declared), Error> {
    if !projected {
        let declared = input.checked_push(&entry)?;
        let mut turn = input.clone();
        turn.commit_push(entry);
        return Ok((turn, declared));
    }
    let declared = places_of_incoming(&entry, input.entries().len());
    let mut entries = input.entries().to_vec();
    entries.push(entry);
    let rebuilt = InputTurn::projected(input.id(), entries)?;
    let rebuilt = match input.observed_at() {
        Some(observed_at) => rebuilt.with_observed_at(observed_at.clone()),
        None => rebuilt,
    };
    Ok((rebuilt, declared))
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
                turn,
                declared,
            } => {
                self.turns[position] = Turn::InputTurn(turn);
                self.index.insert(position, declared);
            }
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
                let (turn, declared) = extended_input(input, entry, false)?;
                if let Some((id, place)) = self.index.clash(&declared) {
                    return Err(Error::new(ErrorCode::ReusedIdentity)
                        .with_id(id)
                        .at(format!("entries[{}]", place.entry)));
                }
                Ok(Prepared::Extend {
                    position,
                    turn,
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
