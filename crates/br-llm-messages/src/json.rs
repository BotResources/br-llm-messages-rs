use std::collections::BTreeMap;
use std::sync::OnceLock;

use serde::de::{DeserializeOwned, Error as _, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Map, Number, Value};

use crate::error::{Error, ErrorCode};

pub(crate) fn encode<T: Serialize>(value: &T) -> Result<String, Error> {
    serde_json::to_string(value).map_err(|_| Error::new(ErrorCode::InvalidContent))
}

pub(crate) fn encode_value<T: Serialize>(value: &T) -> Result<Value, Error> {
    crate::strict_serializer::to_value(value)
}

pub(crate) fn parse(text: &str) -> Result<Value, Error> {
    serde_json::from_str::<UniqueValue>(text)
        .map(|UniqueValue(value)| value)
        .map_err(|error| Error::from_json_error(&error))
}

pub(crate) fn decode<T: DeserializeOwned>(text: &str) -> Result<T, Error> {
    decode_value(parse(text)?)
}

pub(crate) fn decode_value<T: DeserializeOwned>(value: Value) -> Result<T, Error> {
    T::deserialize(value).map_err(|error| Error::from_json_error(&error))
}

pub(crate) fn decode_ref<T: DeserializeOwned>(value: &Value) -> Result<T, Error> {
    T::deserialize(value).map_err(|error| Error::from_json_error(&error))
}

struct UniqueValue(Value);

impl<'de> Deserialize<'de> for UniqueValue {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        unique_value(deserializer).map(UniqueValue)
    }
}

pub(crate) fn unique_value<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Value, D::Error> {
    deserializer.deserialize_any(UniqueValueVisitor)
}

struct UniqueValueVisitor;

impl<'de> Visitor<'de> for UniqueValueVisitor {
    type Value = Value;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("any JSON value")
    }

    fn visit_bool<E>(self, value: bool) -> Result<Value, E> {
        Ok(Value::Bool(value))
    }

    fn visit_i64<E: serde::de::Error>(self, value: i64) -> Result<Value, E> {
        if value == 0 && numbers_keep_text() {
            return finite(-0.0).map_err(E::custom);
        }
        Ok(Value::Number(value.into()))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Value, E> {
        Ok(Value::Number(value.into()))
    }

    fn visit_f64<E: serde::de::Error>(self, value: f64) -> Result<Value, E> {
        finite(value).map_err(E::custom)
    }

    fn visit_str<E>(self, value: &str) -> Result<Value, E> {
        Ok(Value::String(value.to_owned()))
    }

    fn visit_string<E>(self, value: String) -> Result<Value, E> {
        Ok(Value::String(value))
    }

    fn visit_none<E>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }

    fn visit_unit<E>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }

    fn visit_some<D: Deserializer<'de>>(self, deserializer: D) -> Result<Value, D::Error> {
        unique_value(deserializer)
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Value, A::Error> {
        let mut items = Vec::new();
        while let Some(UniqueValue(item)) = seq.next_element()? {
            items.push(item);
        }
        Ok(Value::Array(items))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Value, A::Error> {
        let mut entries = BTreeMap::new();
        let mut key = map.next_key::<String>()?;
        if key.as_deref() == Some(NUMBER_KEY) && numbers_keep_text() {
            let text: String = map.next_value()?;
            return number_from_text(&text).map_err(A::Error::custom);
        }
        while let Some(name) = key {
            let UniqueValue(value) = map.next_value()?;
            if entries.insert(name, value).is_some() {
                return Err(A::Error::custom(Error::new(ErrorCode::MalformedJson)));
            }
            key = map.next_key::<String>()?;
        }
        Ok(Value::Object(sorted(entries)))
    }
}

pub(crate) const NUMBER_KEY: &str = "$serde_json::private::Number";

pub(crate) fn sorted(entries: BTreeMap<String, Value>) -> Map<String, Value> {
    entries.into_iter().collect()
}

pub(crate) fn canonical(value: Value) -> Value {
    if !map_keeps_insertion_order() {
        return value;
    }
    match value {
        Value::Array(items) => Value::Array(items.into_iter().map(canonical).collect()),
        Value::Object(map) => Value::Object(sorted(
            map.into_iter()
                .map(|(key, child)| (key, canonical(child)))
                .collect(),
        )),
        other => other,
    }
}

fn map_keeps_insertion_order() -> bool {
    static KEEPS_ORDER: OnceLock<bool> = OnceLock::new();
    *KEEPS_ORDER.get_or_init(|| {
        let mut probe = Map::new();
        probe.insert("b".to_owned(), Value::Null);
        probe.insert("a".to_owned(), Value::Null);
        probe.keys().next().is_some_and(|key| key == "b")
    })
}

pub(crate) fn finite(value: f64) -> Result<Value, Error> {
    Number::from_f64(value)
        .map(Value::Number)
        .ok_or_else(|| Error::new(ErrorCode::InvalidContent))
}

pub(crate) fn number_from_text(text: &str) -> Result<Value, Error> {
    if let Ok(unsigned) = text.parse::<u64>() {
        return Ok(Value::Number(unsigned.into()));
    }
    match text.parse::<i64>() {
        Ok(0) => finite(-0.0),
        Ok(signed) => Ok(Value::Number(signed.into())),
        Err(_) => text
            .parse::<f64>()
            .ok()
            .and_then(Number::from_f64)
            .map(Value::Number)
            .ok_or_else(|| Error::new(ErrorCode::MalformedJson)),
    }
}

pub(crate) fn numbers_keep_text() -> bool {
    static KEEPS_TEXT: OnceLock<bool> = OnceLock::new();
    *KEEPS_TEXT
        .get_or_init(|| serde_json::from_str::<NumberProbe>("0.5").is_ok_and(|probe| probe.0))
}

struct NumberProbe(bool);

impl<'de> Deserialize<'de> for NumberProbe {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(NumberProbeVisitor)
    }
}

struct NumberProbeVisitor;

impl<'de> Visitor<'de> for NumberProbeVisitor {
    type Value = NumberProbe;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a number")
    }

    fn visit_f64<E>(self, _: f64) -> Result<NumberProbe, E> {
        Ok(NumberProbe(false))
    }

    fn visit_map<A: MapAccess<'de>>(self, _: A) -> Result<NumberProbe, A::Error> {
        Ok(NumberProbe(true))
    }
}

#[cfg(test)]
#[path = "json_tests.rs"]
mod tests;
