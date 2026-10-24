use std::collections::BTreeMap;

use serde::ser::{Serialize, SerializeMap, SerializeStruct, SerializeStructVariant};
use serde_json::Value;

use super::{Refused, Strict, wrap};

pub(crate) struct Fields {
    variant: Option<&'static str>,
    entries: BTreeMap<String, Value>,
    key: Option<String>,
    number: bool,
}

impl Fields {
    pub(crate) fn new(variant: Option<&'static str>, number: bool) -> Self {
        Fields {
            variant,
            entries: BTreeMap::new(),
            key: None,
            number,
        }
    }

    fn insert<T: ?Sized + Serialize>(&mut self, key: String, value: &T) -> Result<(), Refused> {
        match self.entries.insert(key, value.serialize(Strict)?) {
            None => Ok(()),
            Some(_) => Err(Refused),
        }
    }

    fn finish(self) -> Result<Value, Refused> {
        if self.number {
            return match self.entries.into_iter().next() {
                Some((_, Value::String(text))) => {
                    crate::json::number_from_text(&text).map_err(|_| Refused)
                }
                _ => Err(Refused),
            };
        }
        let object = Value::Object(crate::json::sorted(self.entries));
        Ok(wrap(self.variant, object))
    }
}

fn key_text(key: Value) -> Result<String, Refused> {
    match key {
        Value::String(text) => Ok(text),
        Value::Number(number) => Ok(number.to_string()),
        Value::Bool(flag) => Ok(flag.to_string()),
        _ => Err(Refused),
    }
}

impl SerializeMap for Fields {
    type Ok = Value;
    type Error = Refused;

    fn serialize_key<T: ?Sized + Serialize>(&mut self, key: &T) -> Result<(), Refused> {
        self.key = Some(key_text(key.serialize(Strict)?)?);
        Ok(())
    }

    fn serialize_value<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Refused> {
        let key = self.key.take().ok_or(Refused)?;
        self.insert(key, value)
    }

    fn end(self) -> Result<Value, Refused> {
        self.finish()
    }
}

impl SerializeStruct for Fields {
    type Ok = Value;
    type Error = Refused;

    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Refused> {
        self.insert(key.to_owned(), value)
    }

    fn end(self) -> Result<Value, Refused> {
        self.finish()
    }
}

impl SerializeStructVariant for Fields {
    type Ok = Value;
    type Error = Refused;

    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Refused> {
        self.insert(key.to_owned(), value)
    }

    fn end(self) -> Result<Value, Refused> {
        self.finish()
    }
}
