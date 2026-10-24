use serde::de::Error as _;
use serde::{Deserialize, Deserializer};
use serde_json::Value;

use crate::error::{Error, ErrorCode};
use crate::opaque_json::OpaqueJson;

pub(crate) const MAX_DEPTH: usize = 64;

pub(crate) fn check(value: &OpaqueJson) -> Result<(), Error> {
    check_value(value.inner())
}

pub(crate) fn check_present(value: &OpaqueJson) -> Result<(), Error> {
    check_present_value(value.inner())
}

pub(crate) fn check_present_value(value: &Value) -> Result<(), Error> {
    if value.is_null() {
        return Err(Error::new(ErrorCode::InvalidContent));
    }
    check_value(value)
}

pub(crate) fn check_value(value: &Value) -> Result<(), Error> {
    let mut pending = vec![(value, 1usize)];
    while let Some((node, level)) = pending.pop() {
        let children: Vec<&Value> = match node {
            Value::Array(items) => items.iter().collect(),
            Value::Object(fields) => fields.values().collect(),
            _ => continue,
        };
        if level > MAX_DEPTH {
            return Err(Error::new(ErrorCode::InvalidContent));
        }
        pending.extend(children.into_iter().map(|child| (child, level + 1)));
    }
    Ok(())
}

fn deserialize_at<'de, D: Deserializer<'de>>(
    deserializer: D,
    field: &'static str,
) -> Result<OpaqueJson, D::Error> {
    let value = OpaqueJson::deserialize(deserializer)?;
    check(&value).map_err(|error| D::Error::custom(error.at(field)))?;
    Ok(value)
}

pub(crate) fn value<'de, D: Deserializer<'de>>(deserializer: D) -> Result<OpaqueJson, D::Error> {
    deserialize_at(deserializer, "value")
}

pub(crate) fn arguments<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<OpaqueJson, D::Error> {
    deserialize_at(deserializer, "arguments")
}

fn deserialize_present_at<'de, D: Deserializer<'de>>(
    deserializer: D,
    field: &'static str,
) -> Result<Option<OpaqueJson>, D::Error> {
    let value = OpaqueJson::deserialize(deserializer)?;
    check_present(&value).map_err(|error| D::Error::custom(error.at(field)))?;
    Ok(Some(value))
}

pub(crate) fn optional_arguments<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<OpaqueJson>, D::Error> {
    deserialize_present_at(deserializer, "arguments")
}

pub(crate) fn optional_provisional_arguments<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<OpaqueJson>, D::Error> {
    deserialize_present_at(deserializer, "provisional_arguments")
}
