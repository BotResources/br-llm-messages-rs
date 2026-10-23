use serde::de::IgnoredAny;
use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;

use crate::opaque_json::OpaqueJson;

use super::Conversation;
use crate::error::{Error, ErrorCode};
use crate::id::ConversationId;
use crate::turn::Turn;

pub const CONVERSATION_SCHEMA: &str = "br-llm-messages/4";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Wire {
    #[serde(rename = "schema")]
    _schema: IgnoredAny,
    id: ConversationId,
    turns: Vec<Value>,
}

impl Serialize for Conversation {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut conversation = serializer.serialize_struct("Conversation", 3)?;
        conversation.serialize_field("schema", CONVERSATION_SCHEMA)?;
        conversation.serialize_field("id", &self.id)?;
        conversation.serialize_field("turns", &self.turns)?;
        conversation.end()
    }
}

impl<'de> Deserialize<'de> for Conversation {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = crate::json::unique_value(deserializer)?;
        Conversation::from_value(OpaqueJson::from_inner(value)).map_err(serde::de::Error::custom)
    }
}

impl Conversation {
    pub fn to_json(&self) -> Result<String, Error> {
        crate::json::encode(self)
    }

    pub fn to_value(&self) -> Result<OpaqueJson, Error> {
        crate::json::encode_value(self).map(OpaqueJson::from_inner)
    }

    pub fn from_json(text: &str) -> Result<Self, Error> {
        Conversation::from_value(OpaqueJson::from_inner(crate::json::parse(text)?))
    }

    pub fn from_value(value: OpaqueJson) -> Result<Self, Error> {
        let value = value.into_inner();
        match value.get("schema") {
            Some(Value::String(schema)) if schema == CONVERSATION_SCHEMA => {}
            _ => return Err(Error::new(ErrorCode::SchemaMismatch).at("schema")),
        }
        let wire: Wire = crate::json::decode_value(value)?;
        let mut turns = Vec::with_capacity(wire.turns.len());
        for (position, raw) in wire.turns.into_iter().enumerate() {
            let turn = Turn::from_value(OpaqueJson::from_inner(raw))
                .map_err(|error| error.at(format!("turns[{position}]")))?;
            turns.push(turn);
        }
        Conversation::from_parts(wire.id, turns)
    }
}
