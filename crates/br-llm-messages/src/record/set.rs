use serde::de::IgnoredAny;
use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;

use crate::opaque_json::OpaqueJson;

use super::block::BlockRecord;
use super::decompose::turn_rows;
use super::entry::EntryRecord;
use super::reconstruct::rebuild;
use super::reduced::rebuild_thread;
use super::turn::TurnRecord;
use crate::conversation::Conversation;
use crate::error::{Error, ErrorCode};
use crate::id::{AgentId, ConversationId};
use crate::projection::{PendingResponse, ProjectedThread};
use crate::turn::Turn;

pub const RECORD_SCHEMA: &str = "br-llm-messages-records/1";

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub struct RecordSet {
    pub(super) turns: Vec<TurnRecord>,
    pub(super) entries: Vec<EntryRecord>,
    pub(super) blocks: Vec<BlockRecord>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RecordSetWire {
    #[serde(rename = "schema")]
    _schema: IgnoredAny,
    turns: Vec<Value>,
    entries: Vec<Value>,
    blocks: Vec<Value>,
}

impl RecordSet {
    #[allow(clippy::new_without_default)] // a Default derive would freeze the field set
    pub fn new() -> Self {
        RecordSet {
            turns: Vec::new(),
            entries: Vec::new(),
            blocks: Vec::new(),
        }
    }

    pub fn with_turns(mut self, turns: Vec<TurnRecord>) -> Self {
        self.turns = turns;
        self
    }

    pub fn with_entries(mut self, entries: Vec<EntryRecord>) -> Self {
        self.entries = entries;
        self
    }

    pub fn with_blocks(mut self, blocks: Vec<BlockRecord>) -> Self {
        self.blocks = blocks;
        self
    }

    pub fn schema() -> &'static str {
        RECORD_SCHEMA
    }

    pub fn turns(&self) -> &[TurnRecord] {
        &self.turns
    }

    pub fn entries(&self) -> &[EntryRecord] {
        &self.entries
    }

    pub fn blocks(&self) -> &[BlockRecord] {
        &self.blocks
    }

    pub fn from_conversation(conversation: &Conversation) -> Result<RecordSet, Error> {
        let mut set = RecordSet::new();
        for (index, turn) in conversation.turns().iter().enumerate() {
            turn_rows(turn, &mut set).map_err(|error| error.at(format!("turns[{index}]")))?;
        }
        Ok(set)
    }

    pub fn from_turn(turn: &Turn) -> Result<RecordSet, Error> {
        let mut set = RecordSet::new();
        turn_rows(turn, &mut set)?;
        Ok(set)
    }

    pub fn check_local(&self) -> Result<(), Error> {
        for (index, row) in self.turns.iter().enumerate() {
            row.check_local()
                .map_err(|error| error.at(format!("turns[{index}]")))?;
        }
        for (index, row) in self.entries.iter().enumerate() {
            row.check_local()
                .map_err(|error| error.at(format!("entries[{index}]")))?;
        }
        for (index, row) in self.blocks.iter().enumerate() {
            row.check_local()
                .map_err(|error| error.at(format!("blocks[{index}]")))?;
        }
        Ok(())
    }

    pub fn check(&self) -> Result<(), Error> {
        rebuild(self).map(drop)
    }

    pub fn to_turns(&self) -> Result<Vec<Turn>, Error> {
        rebuild(self)
    }

    pub fn to_conversation(&self, id: ConversationId) -> Result<Conversation, Error> {
        Conversation::from_parts(id, rebuild(self)?)
    }

    pub fn to_projected_thread(
        &self,
        conversation: ConversationId,
        agent: AgentId,
    ) -> Result<ProjectedThread, Error> {
        rebuild_thread(self, conversation, agent, &[])
    }

    pub fn to_projected_thread_pending(
        &self,
        conversation: ConversationId,
        agent: AgentId,
        pending: &[PendingResponse<'_>],
    ) -> Result<ProjectedThread, Error> {
        rebuild_thread(self, conversation, agent, pending)
    }

    pub fn into_turns(self) -> Result<Vec<Turn>, Error> {
        self.to_turns()
    }

    pub fn into_conversation(self, id: ConversationId) -> Result<Conversation, Error> {
        self.to_conversation(id)
    }

    pub fn into_projected_thread(
        self,
        conversation: ConversationId,
        agent: AgentId,
    ) -> Result<ProjectedThread, Error> {
        self.to_projected_thread(conversation, agent)
    }

    pub fn into_projected_thread_pending(
        self,
        conversation: ConversationId,
        agent: AgentId,
        pending: &[PendingResponse<'_>],
    ) -> Result<ProjectedThread, Error> {
        self.to_projected_thread_pending(conversation, agent, pending)
    }

    pub fn to_json(&self) -> Result<String, Error> {
        self.check_local()?;
        crate::json::encode(&Checked(self))
    }

    pub fn to_value(&self) -> Result<OpaqueJson, Error> {
        self.check_local()?;
        crate::json::encode_value(&Checked(self)).map(OpaqueJson::from_inner)
    }

    pub fn from_json(text: &str) -> Result<RecordSet, Error> {
        RecordSet::from_value(OpaqueJson::from_inner(crate::json::parse(text)?))
    }

    pub fn from_value(value: OpaqueJson) -> Result<RecordSet, Error> {
        let value = value.into_inner();
        match value.get("schema") {
            Some(Value::String(schema)) if schema == RECORD_SCHEMA => {}
            _ => return Err(Error::new(ErrorCode::SchemaMismatch).at("schema")),
        }
        let wire: RecordSetWire = crate::json::decode_value(value)?;
        Ok(RecordSet {
            turns: rows(wire.turns, "turns")?,
            entries: rows(wire.entries, "entries")?,
            blocks: rows(wire.blocks, "blocks")?,
        })
    }
}

fn rows<T: serde::de::DeserializeOwned>(raw: Vec<Value>, name: &str) -> Result<Vec<T>, Error> {
    raw.into_iter()
        .enumerate()
        .map(|(index, value)| {
            crate::json::decode_value(value).map_err(|error| error.at(format!("{name}[{index}]")))
        })
        .collect()
}

struct Checked<'a>(&'a RecordSet);

impl Serialize for Checked<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut set = serializer.serialize_struct("RecordSet", 4)?;
        set.serialize_field("schema", RECORD_SCHEMA)?;
        set.serialize_field("turns", &self.0.turns)?;
        set.serialize_field("entries", &self.0.entries)?;
        set.serialize_field("blocks", &self.0.blocks)?;
        set.end()
    }
}

impl Serialize for RecordSet {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.check_local().map_err(serde::ser::Error::custom)?;
        Checked(self).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for RecordSet {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = crate::json::unique_value(deserializer)?;
        RecordSet::from_value(OpaqueJson::from_inner(value)).map_err(serde::de::Error::custom)
    }
}
