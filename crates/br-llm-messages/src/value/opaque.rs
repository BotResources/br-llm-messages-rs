use serde::Deserializer;
use serde::de::Error as _;
use serde_json::Value;

use crate::error::{Error, ErrorCode};
use crate::opaque_json::OpaqueJson;

pub(crate) const MAX_DEPTH: usize = 64;

pub(crate) fn check(value: &OpaqueJson) -> Result<(), Error> {
    check_value(value.inner())
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

pub(crate) fn deserialize<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<OpaqueJson, D::Error> {
    let value = OpaqueJson::from_inner(crate::json::unique_value(deserializer)?);
    check(&value).map_err(D::Error::custom)?;
    Ok(value)
}
