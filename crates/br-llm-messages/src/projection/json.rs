use serde::de::IgnoredAny;
use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;

use super::ProjectedThread;
use crate::error::{Error, ErrorCode};
use crate::id::{AgentId, ConversationId};
use crate::opaque_json::OpaqueJson;

pub const PROJECTED_THREAD_SCHEMA: &str = "br-llm-messages-projected-thread/1";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Wire {
    #[serde(rename = "schema")]
    _schema: IgnoredAny,
    conversation: ConversationId,
    agent: AgentId,
    turns: Vec<Value>,
}

impl Serialize for ProjectedThread {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut thread = serializer.serialize_struct("ProjectedThread", 4)?;
        thread.serialize_field("schema", PROJECTED_THREAD_SCHEMA)?;
        thread.serialize_field("conversation", &self.conversation)?;
        thread.serialize_field("agent", &self.agent)?;
        thread.serialize_field("turns", &self.turns)?;
        thread.end()
    }
}

impl<'de> Deserialize<'de> for ProjectedThread {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = crate::json::unique_value(deserializer)?;
        ProjectedThread::from_value(OpaqueJson::from_inner(value)).map_err(serde::de::Error::custom)
    }
}

impl ProjectedThread {
    pub fn schema() -> &'static str {
        PROJECTED_THREAD_SCHEMA
    }

    pub fn to_json(&self) -> Result<String, Error> {
        crate::json::encode(self)
    }

    pub fn to_value(&self) -> Result<OpaqueJson, Error> {
        crate::json::encode_value(self).map(OpaqueJson::from_inner)
    }

    pub fn from_json(text: &str) -> Result<Self, Error> {
        ProjectedThread::from_value(OpaqueJson::from_inner(crate::json::parse(text)?))
    }

    pub fn from_value(value: OpaqueJson) -> Result<Self, Error> {
        let value = value.into_inner();
        match value.get("schema") {
            Some(Value::String(schema)) if schema == PROJECTED_THREAD_SCHEMA => {}
            _ => return Err(Error::new(ErrorCode::SchemaMismatch).at("schema")),
        }
        let wire: Wire = crate::json::decode_value(value)?;
        let mut turns = Vec::with_capacity(wire.turns.len());
        for (position, raw) in wire.turns.into_iter().enumerate() {
            let turn = crate::turn::projected_turn_from_value(raw)
                .map_err(|error| error.at(format!("turns[{position}]")))?;
            turns.push(turn);
        }
        ProjectedThread::assemble(wire.conversation, wire.agent, turns)
    }
}
