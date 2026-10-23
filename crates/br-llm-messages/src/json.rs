use serde::de::{DeserializeOwned, Error as _, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Map, Number, Value};

use crate::error::{Error, ErrorCode};

pub(crate) fn encode<T: Serialize>(value: &T) -> Result<String, Error> {
    serde_json::to_string(value).map_err(|_| Error::new(ErrorCode::InvalidContent))
}

pub(crate) fn encode_value<T: Serialize>(value: &T) -> Result<Value, Error> {
    serde_json::to_value(value).map_err(|_| Error::new(ErrorCode::InvalidContent))
}

pub(crate) fn decode<T: DeserializeOwned>(text: &str) -> Result<T, Error> {
    serde_json::from_str::<UniqueValue>(text).map_err(|error| Error::from_json_error(&error))?;
    serde_json::from_str(text).map_err(|error| Error::from_json_error(&error))
}

pub(crate) fn decode_value<T: DeserializeOwned>(value: Value) -> Result<T, Error> {
    decode_ref(&value)
}

pub(crate) fn decode_ref<T: DeserializeOwned>(value: &Value) -> Result<T, Error> {
    let text = serde_json::to_string(value).map_err(|_| Error::new(ErrorCode::InvalidContent))?;
    serde_json::from_str(&text).map_err(|error| Error::from_json_error(&error))
}

const NUMBER_TOKEN: &str = "$serde_json::private::Number";

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

    fn visit_i64<E>(self, value: i64) -> Result<Value, E> {
        Ok(Value::Number(value.into()))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Value, E> {
        Ok(Value::Number(value.into()))
    }

    fn visit_f64<E>(self, value: f64) -> Result<Value, E> {
        Ok(Number::from_f64(value).map_or(Value::Null, Value::Number))
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
        let mut object = Map::new();
        let Some(mut key) = map.next_key::<String>()? else {
            return Ok(Value::Object(object));
        };
        if key == NUMBER_TOKEN {
            let text: String = map.next_value()?;
            return serde_json::from_str::<Number>(&text)
                .map(Value::Number)
                .map_err(|_| A::Error::custom(Error::new(ErrorCode::InvalidContent)));
        }
        loop {
            let UniqueValue(value) = map.next_value()?;
            if object.insert(key, value).is_some() {
                return Err(A::Error::custom(Error::new(ErrorCode::MalformedJson)));
            }
            match map.next_key::<String>()? {
                Some(next) => key = next,
                None => return Ok(Value::Object(object)),
            }
        }
    }
}
