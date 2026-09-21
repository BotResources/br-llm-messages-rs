use serde::ser::SerializeStruct;

use crate::block::ToolResult;
use crate::error::MessageError;
use crate::step::Step;
use crate::turn::Turn;
use crate::user_input::UserInput;
use crate::value::TurnId;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Entry {
    UserInput(UserInput),
    Turn(Turn),
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Conversation {
    entries: Vec<Entry>,
}

#[derive(serde::Deserialize)]
struct RawConversation {
    schema: String,
    entries: Vec<Entry>,
}

impl TryFrom<RawConversation> for Conversation {
    type Error = MessageError;

    fn try_from(raw: RawConversation) -> Result<Self, Self::Error> {
        if raw.schema != crate::SCHEMA_VERSION {
            return Err(MessageError::SchemaMismatch { found: raw.schema });
        }
        let mut conversation = Conversation::new();
        for entry in raw.entries {
            match entry {
                Entry::UserInput(input) => conversation.push_input(input),
                Entry::Turn(turn) => conversation.push_turn(turn)?,
            }
        }
        Ok(conversation)
    }
}

impl serde::Serialize for Conversation {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("Conversation", 2)?;
        state.serialize_field("schema", crate::SCHEMA_VERSION)?;
        state.serialize_field("entries", &self.entries)?;
        state.end()
    }
}

impl<'de> serde::Deserialize<'de> for Conversation {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = RawConversation::deserialize(deserializer)?;
        Conversation::try_from(raw).map_err(serde::de::Error::custom)
    }
}

impl Conversation {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn push_input(&mut self, input: UserInput) {
        self.entries.push(Entry::UserInput(input));
    }

    pub fn push_turn(&mut self, turn: Turn) -> Result<(), MessageError> {
        if self.turn(turn.id()).is_some() {
            return Err(MessageError::DuplicateTurnId {
                id: turn.id().clone(),
            });
        }
        self.entries.push(Entry::Turn(turn));
        Ok(())
    }

    pub fn push_result(
        &mut self,
        turn_id: &TurnId,
        result: ToolResult,
    ) -> Result<(), MessageError> {
        self.turn_mut(turn_id)?.push_result(result)
    }

    pub fn push_step(&mut self, turn_id: &TurnId, step: Step) -> Result<(), MessageError> {
        self.turn_mut(turn_id)?.push_step(step)
    }

    fn turn_mut(&mut self, turn_id: &TurnId) -> Result<&mut Turn, MessageError> {
        for entry in &mut self.entries {
            if let Entry::Turn(turn) = entry
                && turn.id() == turn_id
            {
                return Ok(turn);
            }
        }
        Err(MessageError::TurnNotFound {
            id: turn_id.clone(),
        })
    }

    pub fn turn(&self, turn_id: &TurnId) -> Option<&Turn> {
        self.entries.iter().find_map(|entry| match entry {
            Entry::Turn(turn) if turn.id() == turn_id => Some(turn),
            Entry::Turn(_) | Entry::UserInput(_) => None,
        })
    }

    pub fn open_turns(&self) -> impl Iterator<Item = &Turn> {
        self.entries.iter().filter_map(|entry| match entry {
            Entry::Turn(turn)
                if !matches!(turn.state(), crate::turn::TurnState::Finished { .. }) =>
            {
                Some(turn)
            }
            Entry::Turn(_) | Entry::UserInput(_) => None,
        })
    }

    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }
}

impl std::fmt::Display for Conversation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "conversation ({} entries)", self.entries.len())?;
        for entry in &self.entries {
            match entry {
                Entry::UserInput(input) => writeln!(f, "  {input}")?,
                Entry::Turn(turn) => writeln!(f, "  {turn}")?,
            }
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "conversation_tests.rs"]
mod tests;
