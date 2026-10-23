use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;

use crate::error::Error;

#[derive(Clone, PartialEq)]
pub struct OpaqueJson(Value);

impl OpaqueJson {
    pub fn parse(text: &str) -> Result<Self, Error> {
        crate::json::parse(text).map(OpaqueJson)
    }

    pub fn from_serialize<T: Serialize>(value: &T) -> Result<Self, Error> {
        crate::strict_serializer::to_value(value).map(OpaqueJson)
    }

    pub fn to_json_string(&self) -> Result<String, Error> {
        crate::json::encode(&self.0)
    }

    pub fn deserialize_into<T: DeserializeOwned>(&self) -> Result<T, Error> {
        crate::json::decode_ref(&self.0)
    }

    pub(crate) fn from_inner(value: Value) -> Self {
        OpaqueJson(crate::json::canonical(value))
    }

    pub(crate) fn inner(&self) -> &Value {
        &self.0
    }

    pub(crate) fn into_inner(self) -> Value {
        self.0
    }

    pub(crate) fn empty_object() -> Self {
        OpaqueJson(Value::Object(serde_json::Map::new()))
    }
}

impl Serialize for OpaqueJson {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for OpaqueJson {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        crate::json::unique_value(deserializer).map(OpaqueJson)
    }
}

impl std::fmt::Debug for OpaqueJson {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("OpaqueJson(<redacted>)")
    }
}
