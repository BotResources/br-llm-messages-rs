use serde::de::IgnoredAny;
use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;

use crate::opaque_json::OpaqueJson;

use super::delta::{DeltaOp, ResponseDelta};
use super::kind::DeltaTarget;
use crate::error::{Error, ErrorCode};
use crate::id::{EntryId, TurnId};

pub const DELTA_SCHEMA: &str = "br-llm-messages-delta/1";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Wire {
    #[serde(rename = "schema")]
    _schema: IgnoredAny,
    turn: TurnId,
    entry: EntryId,
    target: DeltaTarget,
    op: DeltaOp,
}

impl Serialize for ResponseDelta {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut delta = serializer.serialize_struct("ResponseDelta", 5)?;
        delta.serialize_field("schema", DELTA_SCHEMA)?;
        delta.serialize_field("turn", &self.turn())?;
        delta.serialize_field("entry", &self.entry())?;
        delta.serialize_field("target", &self.op().target())?;
        delta.serialize_field("op", self.op())?;
        delta.end()
    }
}

impl<'de> Deserialize<'de> for ResponseDelta {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = crate::json::unique_value(deserializer)?;
        ResponseDelta::from_value(OpaqueJson::from_inner(value)).map_err(serde::de::Error::custom)
    }
}

impl ResponseDelta {
    pub fn to_json(&self) -> Result<String, Error> {
        crate::json::encode(self)
    }

    pub fn to_value(&self) -> Result<OpaqueJson, Error> {
        crate::json::encode_value(self).map(OpaqueJson::from_inner)
    }

    pub fn from_json(text: &str) -> Result<Self, Error> {
        ResponseDelta::from_value(OpaqueJson::from_inner(crate::json::parse(text)?))
    }

    pub fn from_value(value: OpaqueJson) -> Result<Self, Error> {
        let value = value.into_inner();
        match value.get("schema") {
            Some(Value::String(schema)) if schema == DELTA_SCHEMA => {}
            _ => return Err(Error::new(ErrorCode::SchemaMismatch).at("schema")),
        }
        let wire: Wire = crate::json::decode_value(value)?;
        if wire.target != wire.op.target() {
            let mut mismatch = Error::new(ErrorCode::StreamTargetError).at("target");
            mismatch = mismatch.with_id(
                wire.op
                    .block_id()
                    .map_or(wire.entry.as_uuid(), |block| block.as_uuid()),
            );
            return Err(mismatch);
        }
        Ok(ResponseDelta::new(wire.turn, wire.entry, wire.op))
    }
}
