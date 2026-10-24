use serde::de::IgnoredAny;
use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;

use crate::opaque_json::OpaqueJson;

use super::block::BlockRecord;
use super::decompose::turn_rows;
use super::entry::EntryRecord;
use super::reconstruct::rebuild;
use super::turn::TurnRecord;
use crate::conversation::Conversation;
use crate::error::{Error, ErrorCode};
use crate::id::ConversationId;
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

    pub fn into_turns(self) -> Result<Vec<Turn>, Error> {
        rebuild(&self)
    }

    pub fn into_conversation(self, id: ConversationId) -> Result<Conversation, Error> {
        let turns = rebuild(&self)?;
        Conversation::from_parts(id, turns)
    }

    pub fn to_json(&self) -> Result<String, Error> {
        crate::json::encode(self)
    }

    pub fn to_value(&self) -> Result<OpaqueJson, Error> {
        crate::json::encode_value(self).map(OpaqueJson::from_inner)
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

impl Serialize for RecordSet {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut set = serializer.serialize_struct("RecordSet", 4)?;
        set.serialize_field("schema", RECORD_SCHEMA)?;
        set.serialize_field("turns", &self.turns)?;
        set.serialize_field("entries", &self.entries)?;
        set.serialize_field("blocks", &self.blocks)?;
        set.end()
    }
}

impl<'de> Deserialize<'de> for RecordSet {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = crate::json::unique_value(deserializer)?;
        RecordSet::from_value(OpaqueJson::from_inner(value)).map_err(serde::de::Error::custom)
    }
}
