mod fields;
mod items;

use std::fmt::Display;

use serde::ser::{self, Serialize, Serializer};
use serde_json::Value;
use serde_json::value::Serializer as ValueSerializer;

use self::fields::Fields;
use self::items::Items;
use crate::error::{Error, ErrorCode};

pub(crate) fn to_value<T: Serialize>(value: &T) -> Result<Value, Error> {
    value
        .serialize(Strict)
        .map_err(|_| Error::new(ErrorCode::InvalidContent))
}

#[derive(Debug)]
pub(crate) struct Refused;

impl Display for Refused {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("value refused")
    }
}

impl std::error::Error for Refused {}

impl ser::Error for Refused {
    fn custom<T: Display>(_: T) -> Self {
        Refused
    }
}

pub(crate) struct Strict;

macro_rules! forward {
    ($($method:ident($ty:ty)),* $(,)?) => {
        $(
            fn $method(self, value: $ty) -> Result<Value, Refused> {
                ValueSerializer.$method(value).map_err(|_| Refused)
            }
        )*
    };
}

pub(super) fn wrap(variant: Option<&'static str>, value: Value) -> Value {
    match variant {
        Some(name) => Value::Object(std::iter::once((name.to_owned(), value)).collect()),
        None => value,
    }
}

impl Serializer for Strict {
    type Ok = Value;
    type Error = Refused;
    type SerializeSeq = Items;
    type SerializeTuple = Items;
    type SerializeTupleStruct = Items;
    type SerializeTupleVariant = Items;
    type SerializeMap = Fields;
    type SerializeStruct = Fields;
    type SerializeStructVariant = Fields;

    forward!(
        serialize_bool(bool),
        serialize_i8(i8),
        serialize_i16(i16),
        serialize_i32(i32),
        serialize_i64(i64),
        serialize_u8(u8),
        serialize_u16(u16),
        serialize_u32(u32),
        serialize_u64(u64),
        serialize_char(char),
        serialize_str(&str),
        serialize_bytes(&[u8]),
    );

    fn serialize_f32(self, value: f32) -> Result<Value, Refused> {
        if !value.is_finite() {
            return Err(Refused);
        }
        ValueSerializer.serialize_f32(value).map_err(|_| Refused)
    }

    fn serialize_f64(self, value: f64) -> Result<Value, Refused> {
        if !value.is_finite() {
            return Err(Refused);
        }
        ValueSerializer.serialize_f64(value).map_err(|_| Refused)
    }

    fn serialize_none(self) -> Result<Value, Refused> {
        Ok(Value::Null)
    }

    fn serialize_some<T: ?Sized + Serialize>(self, value: &T) -> Result<Value, Refused> {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Value, Refused> {
        Ok(Value::Null)
    }

    fn serialize_unit_struct(self, _: &'static str) -> Result<Value, Refused> {
        Ok(Value::Null)
    }

    fn serialize_unit_variant(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
    ) -> Result<Value, Refused> {
        Ok(Value::String(variant.to_owned()))
    }

    fn serialize_newtype_struct<T: ?Sized + Serialize>(
        self,
        _: &'static str,
        value: &T,
    ) -> Result<Value, Refused> {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Value, Refused> {
        Ok(wrap(Some(variant), value.serialize(Strict)?))
    }

    fn serialize_seq(self, _: Option<usize>) -> Result<Items, Refused> {
        Ok(Items::new(None))
    }

    fn serialize_tuple(self, _: usize) -> Result<Items, Refused> {
        Ok(Items::new(None))
    }

    fn serialize_tuple_struct(self, _: &'static str, _: usize) -> Result<Items, Refused> {
        Ok(Items::new(None))
    }

    fn serialize_tuple_variant(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
        _: usize,
    ) -> Result<Items, Refused> {
        Ok(Items::new(Some(variant)))
    }

    fn serialize_map(self, _: Option<usize>) -> Result<Fields, Refused> {
        Ok(Fields::new(None, false))
    }

    fn serialize_struct(self, name: &'static str, _: usize) -> Result<Fields, Refused> {
        let number = name == crate::json::NUMBER_KEY && crate::json::numbers_keep_text();
        Ok(Fields::new(None, number))
    }

    fn serialize_struct_variant(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
        _: usize,
    ) -> Result<Fields, Refused> {
        Ok(Fields::new(Some(variant), false))
    }
}
