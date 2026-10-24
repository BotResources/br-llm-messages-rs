use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{Map, Value};

use crate::opaque_json::OpaqueJson;

use super::delta::{DeltaOp, ResponseDelta};
use crate::error::{Error, ErrorCode};
use crate::id::{EntryId, TurnId};

const TYPE_INSERTED: &str = "block_inserted";

impl Serialize for ResponseDelta {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        flat(self)
            .map_err(serde::ser::Error::custom)?
            .serialize(serializer)
    }
}

fn flat(delta: &ResponseDelta) -> Result<Value, Error> {
    let Value::Object(mut object) = crate::json::encode_value(delta.op())? else {
        return Err(Error::new(ErrorCode::InvalidContent));
    };
    object.insert("turn".to_owned(), crate::json::encode_value(&delta.turn())?);
    object.insert(
        "entry".to_owned(),
        crate::json::encode_value(&delta.entry())?,
    );
    if let DeltaOp::BlockInserted { block } = delta.op() {
        object.insert("block".to_owned(), crate::json::encode_value(&block.id())?);
    }
    Ok(Value::Object(object))
}

impl<'de> Deserialize<'de> for ResponseDelta {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = crate::json::unique_value(deserializer)?;
        ResponseDelta::from_value(OpaqueJson::from_inner(value)).map_err(serde::de::Error::custom)
    }
}

fn take<T: serde::de::DeserializeOwned>(
    object: &mut Map<String, Value>,
    key: &str,
) -> Result<T, Error> {
    let value = object
        .remove(key)
        .ok_or_else(|| Error::new(ErrorCode::InvalidContent).at(key))?;
    crate::json::decode_value(value).map_err(|error| error.at(key))
}

fn check_inserted_identity(object: &mut Map<String, Value>) -> Result<(), Error> {
    let declared = object
        .remove("block")
        .ok_or_else(|| Error::new(ErrorCode::InvalidContent).at("block"))?;
    let embedded = object.get("content").and_then(|content| content.get("id"));
    if embedded == Some(&declared) {
        return Ok(());
    }
    let mut mismatch = Error::new(ErrorCode::StreamTargetError).at("block");
    if let Some(id) = declared
        .as_str()
        .and_then(|text| uuid::Uuid::try_parse(text).ok())
    {
        mismatch = mismatch.with_id(id);
    }
    Err(mismatch)
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
        let Value::Object(mut object) = value.into_inner() else {
            return Err(Error::new(ErrorCode::InvalidContent));
        };
        let turn: TurnId = take(&mut object, "turn")?;
        let entry: EntryId = take(&mut object, "entry")?;
        if object.get("type").and_then(Value::as_str) == Some(TYPE_INSERTED) {
            check_inserted_identity(&mut object).map_err(|error| match error.object_id() {
                Some(_) => error,
                None => error.with_id(entry.as_uuid()),
            })?;
        }
        let op: DeltaOp = crate::json::decode_value(Value::Object(object))?;
        Ok(ResponseDelta::new(turn, entry, op))
    }
}
