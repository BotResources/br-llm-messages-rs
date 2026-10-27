use super::Conversation;
use crate::error::{Error, ErrorCode};
use crate::id::{ConversationId, EntryId, TurnId};
use crate::turn::{AITurn, Class, InputTurn, Turn};

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Position {
    Turn(TurnId),
    Entry(EntryId),
}

impl From<TurnId> for Position {
    fn from(id: TurnId) -> Self {
        Position::Turn(id)
    }
}

impl From<EntryId> for Position {
    fn from(id: EntryId) -> Self {
        Position::Entry(id)
    }
}

impl Conversation {
    pub fn cut_before(&mut self, position: impl Into<Position>) -> Result<(), Error> {
        *self = self.truncated(position.into(), self.id)?;
        Ok(())
    }

    pub fn fork_before(
        &self,
        position: impl Into<Position>,
        conversation: Option<ConversationId>,
    ) -> Result<Conversation, Error> {
        self.truncated(position.into(), conversation.unwrap_or(self.id))
    }

    fn truncated(&self, position: Position, id: ConversationId) -> Result<Conversation, Error> {
        let (turn, kept_entries) = self.locate(position)?;
        let mut turns = self.turns[..turn].to_vec();
        if let Some(rebuilt) = prefix_of(&self.turns[turn], kept_entries)? {
            turns.push(rebuilt);
        }
        Conversation::from_parts(id, turns)
    }

    fn locate(&self, position: Position) -> Result<(usize, Option<usize>), Error> {
        let unknown = |id: uuid::Uuid| Error::new(ErrorCode::UnknownReference).with_id(id);
        match position {
            Position::Turn(id) => {
                let slot = self.index.get(id.as_uuid());
                match slot {
                    Some(slot) if slot.place.class == Class::Turn => Ok((slot.turn, None)),
                    _ => Err(unknown(id.as_uuid())),
                }
            }
            Position::Entry(id) => match self.index.get(id.as_uuid()) {
                Some(slot) if matches!(slot.place.class, Class::Entry(_)) => {
                    Ok((slot.turn, Some(slot.place.entry)))
                }
                _ => Err(unknown(id.as_uuid())),
            },
        }
    }
}

fn prefix_of(turn: &Turn, kept_entries: Option<usize>) -> Result<Option<Turn>, Error> {
    let Some(kept) = kept_entries else {
        return Ok(None);
    };
    match turn {
        Turn::InputTurn(_) if kept == 0 => Ok(None),
        Turn::InputTurn(input) => {
            let rebuilt = InputTurn::new(input.id(), input.entries()[..kept].to_vec())?;
            Ok(Some(Turn::InputTurn(match input.observed_at() {
                Some(observed_at) => rebuilt.with_observed_at(observed_at.clone()),
                None => rebuilt,
            })))
        }
        Turn::AITurn(ai) => {
            let rebuilt = AITurn::from_parts(
                ai.id(),
                ai.agent(),
                ai.author().clone(),
                ai.entries()[..kept].to_vec(),
            )?;
            Ok(Some(Turn::AITurn(match ai.observed_at() {
                Some(observed_at) => rebuilt.with_observed_at(observed_at.clone()),
                None => rebuilt,
            })))
        }
    }
}
