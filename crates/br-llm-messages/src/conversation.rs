use std::collections::HashSet;

use serde::ser::SerializeStruct;

use crate::block::{ToolCall, ToolResult};
use crate::error::MessageError;
use crate::step::Step;
use crate::turn::{Turn, TurnState};
use crate::user_input::UserInput;
use crate::value::{ToolCallId, TurnId};

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Entry {
    UserInput(UserInput),
    Turn(Turn),
}

/// The ordered entries of a conversation. It only grows: entries are never
/// removed or reordered, and a turn only receives steps and results at its
/// end, so the position of an entry, and of an item or a result within its
/// turn, is a stable address.
#[derive(Clone, Default)]
pub struct Conversation {
    entries: Vec<Entry>,
    /// The id of every tool call of every turn, derived from `entries` and
    /// kept beside them so that the conversation-wide uniqueness of a call id
    /// costs one lookup per call, on build and on load alike.
    call_ids: HashSet<ToolCallId>,
}

impl PartialEq for Conversation {
    fn eq(&self, other: &Self) -> bool {
        self.entries == other.entries
    }
}

impl std::fmt::Debug for Conversation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Conversation")
            .field("entries", &self.entries)
            .finish_non_exhaustive()
    }
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
        Self::default()
    }

    /// Entries taken from a conversation, kept in order with their turn ids
    /// and tool call ids: both are already unique.
    pub(crate) fn from_entries(entries: Vec<Entry>) -> Self {
        let call_ids = entries
            .iter()
            .filter_map(Entry::as_turn)
            .flat_map(Turn::tool_calls)
            .map(|call| call.id.clone())
            .collect();
        Self { entries, call_ids }
    }

    pub fn push_input(&mut self, input: UserInput) {
        self.entries.push(Entry::UserInput(input));
    }

    /// Accepts a turn whose id is new to the conversation and none of whose
    /// tool call ids is used by a call of any turn.
    pub fn push_turn(&mut self, turn: Turn) -> Result<(), MessageError> {
        if self.turn(turn.id()).is_some() {
            return Err(MessageError::DuplicateTurnId {
                id: turn.id().clone(),
            });
        }
        let ids = self.unused_call_ids(turn.tool_calls())?;
        self.entries.push(Entry::Turn(turn));
        self.call_ids.extend(ids);
        Ok(())
    }

    pub fn push_result(
        &mut self,
        turn_id: &TurnId,
        result: ToolResult,
    ) -> Result<(), MessageError> {
        self.turn_mut(turn_id)?.push_result(result)
    }

    /// Accepts a step under the rules of its turn, then when none of its tool
    /// call ids is used by a call of any turn.
    pub fn push_step(&mut self, turn_id: &TurnId, step: Step) -> Result<(), MessageError> {
        let turn = self
            .turn(turn_id)
            .ok_or_else(|| MessageError::TurnNotFound {
                id: turn_id.clone(),
            })?;
        turn.check_step(&step)?;
        let ids = self.unused_call_ids(step.tool_calls())?;
        self.turn_mut(turn_id)?.push_step(step)?;
        self.call_ids.extend(ids);
        Ok(())
    }

    /// The ids of `calls`, refused when one is already used in the
    /// conversation.
    fn unused_call_ids<'a>(
        &self,
        calls: impl Iterator<Item = &'a ToolCall>,
    ) -> Result<Vec<ToolCallId>, MessageError> {
        calls
            .map(|call| {
                if self.call_ids.contains(&call.id) {
                    Err(MessageError::DuplicateToolCallIdInConversation {
                        id: call.id.clone(),
                    })
                } else {
                    Ok(call.id.clone())
                }
            })
            .collect()
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
        self.entries
            .iter()
            .filter_map(Entry::as_turn)
            .find(|turn| turn.id() == turn_id)
    }

    pub fn open_turns(&self) -> impl Iterator<Item = &Turn> {
        self.entries
            .iter()
            .filter_map(Entry::as_turn)
            .filter(|turn| !matches!(turn.state(), TurnState::Finished { .. }))
    }

    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }
}

impl Entry {
    fn as_turn(&self) -> Option<&Turn> {
        match self {
            Entry::Turn(turn) => Some(turn),
            Entry::UserInput(_) => None,
        }
    }
}

#[cfg(test)]
#[path = "conversation_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "conversation_rules_tests.rs"]
mod rules_tests;

#[cfg(test)]
#[path = "conversation_pieces_tests.rs"]
mod pieces_tests;
