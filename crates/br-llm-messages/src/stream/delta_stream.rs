use serde::de::IgnoredAny;
use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;

use super::delta::ResponseDelta;
use crate::error::{Error, ErrorCode};
use crate::opaque_json::OpaqueJson;

pub const DELTA_SCHEMA: &str = "br-llm-messages-delta/1";

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub struct DeltaStream {
    deltas: Vec<ResponseDelta>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Wire {
    #[serde(rename = "schema")]
    _schema: IgnoredAny,
    deltas: Vec<Value>,
}

impl DeltaStream {
    pub fn new(deltas: Vec<ResponseDelta>) -> Self {
        DeltaStream { deltas }
    }

    pub fn schema() -> &'static str {
        DELTA_SCHEMA
    }

    pub fn deltas(&self) -> &[ResponseDelta] {
        &self.deltas
    }

    pub fn into_deltas(self) -> Vec<ResponseDelta> {
        self.deltas
    }

    pub fn to_json(&self) -> Result<String, Error> {
        crate::json::encode(self)
    }

    pub fn to_value(&self) -> Result<OpaqueJson, Error> {
        crate::json::encode_value(self).map(OpaqueJson::from_inner)
    }

    pub fn from_json(text: &str) -> Result<Self, Error> {
        DeltaStream::from_value(OpaqueJson::from_inner(crate::json::parse(text)?))
    }

    pub fn from_value(value: OpaqueJson) -> Result<Self, Error> {
        let value = value.into_inner();
        match value.get("schema") {
            Some(Value::String(schema)) if schema == DELTA_SCHEMA => {}
            _ => return Err(Error::new(ErrorCode::SchemaMismatch).at("schema")),
        }
        let wire: Wire = crate::json::decode_value(value)?;
        let mut deltas = Vec::with_capacity(wire.deltas.len());
        for (index, raw) in wire.deltas.into_iter().enumerate() {
            let delta = ResponseDelta::from_value(OpaqueJson::from_inner(raw))
                .map_err(|error| error.at(format!("deltas[{index}]")))?;
            deltas.push(delta);
        }
        Ok(DeltaStream { deltas })
    }
}

impl Serialize for DeltaStream {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut stream = serializer.serialize_struct("DeltaStream", 2)?;
        stream.serialize_field("schema", DELTA_SCHEMA)?;
        stream.serialize_field("deltas", &self.deltas)?;
        stream.end()
    }
}

impl<'de> Deserialize<'de> for DeltaStream {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = crate::json::unique_value(deserializer)?;
        DeltaStream::from_value(OpaqueJson::from_inner(value)).map_err(serde::de::Error::custom)
    }
}
