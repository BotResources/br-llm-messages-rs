use serde::ser::{
    Serialize, SerializeSeq, SerializeTuple, SerializeTupleStruct, SerializeTupleVariant,
};
use serde_json::Value;

use super::{Refused, Strict, wrap};

pub(crate) struct Items {
    variant: Option<&'static str>,
    values: Vec<Value>,
}

impl Items {
    pub(crate) fn new(variant: Option<&'static str>) -> Self {
        Items {
            variant,
            values: Vec::new(),
        }
    }

    fn push<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Refused> {
        self.values.push(value.serialize(Strict)?);
        Ok(())
    }

    fn finish(self) -> Result<Value, Refused> {
        Ok(wrap(self.variant, Value::Array(self.values)))
    }
}

impl SerializeSeq for Items {
    type Ok = Value;
    type Error = Refused;

    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Refused> {
        self.push(value)
    }

    fn end(self) -> Result<Value, Refused> {
        self.finish()
    }
}

impl SerializeTuple for Items {
    type Ok = Value;
    type Error = Refused;

    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Refused> {
        self.push(value)
    }

    fn end(self) -> Result<Value, Refused> {
        self.finish()
    }
}

impl SerializeTupleStruct for Items {
    type Ok = Value;
    type Error = Refused;

    fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Refused> {
        self.push(value)
    }

    fn end(self) -> Result<Value, Refused> {
        self.finish()
    }
}

impl SerializeTupleVariant for Items {
    type Ok = Value;
    type Error = Refused;

    fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Refused> {
        self.push(value)
    }

    fn end(self) -> Result<Value, Refused> {
        self.finish()
    }
}
