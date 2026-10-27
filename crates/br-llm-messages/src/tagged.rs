use serde::Deserializer;
use serde::de::Error as _;
use serde_json::Value;

use crate::error::{Error, ErrorCode};

pub(crate) fn deserialize<'de, D, T>(
    deserializer: D,
    check: fn(&Value) -> Result<(), Error>,
    build: impl FnOnce(Value) -> Result<T, serde_json::Error>,
) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
{
    let value = crate::json::unique_value(deserializer)?;
    require_string_tag(&value).map_err(D::Error::custom)?;
    check(&value).map_err(D::Error::custom)?;
    build(value).map_err(D::Error::custom)
}

pub(crate) fn accept_any(_: &Value) -> Result<(), Error> {
    Ok(())
}

pub(crate) fn tag<'de, D: Deserializer<'de>>(deserializer: D) -> Result<String, D::Error> {
    match crate::json::unique_value(deserializer)? {
        Value::String(name) => Ok(name),
        _ => Err(D::Error::custom(
            Error::new(ErrorCode::VariantMismatch).at("type"),
        )),
    }
}

fn require_string_tag(value: &Value) -> Result<(), Error> {
    match value.get("type") {
        Some(tag) if !tag.is_string() => Err(Error::new(ErrorCode::VariantMismatch).at("type")),
        _ => Ok(()),
    }
}

macro_rules! tagged_enum {
    (
        $(#[$meta:meta])*
        $vis:vis enum $name:ident, $wire:ident, remote = $remote:literal, [$($serde:tt)*] {
            $($body:tt)*
        }
    ) => {
        tagged_enum! {
            $(#[$meta])*
            $vis enum $name, $wire, remote = $remote, check = $crate::tagged::accept_any, [$($serde)*] {
                $($body)*
            }
        }
    };
    (
        $(#[$meta:meta])*
        $vis:vis enum $name:ident, $wire:ident, remote = $remote:literal, check = $check:path, [$($serde:tt)*] {
            $($body:tt)*
        }
    ) => {
        $(#[$meta])*
        #[derive(serde::Serialize)]
        #[serde($($serde)*)]
        $vis enum $name {
            $($body)*
        }

        tagged_enum!(@mirror $name, $wire, $remote, $check, [$($serde)*], { $($body)* });
    };
    (@mirror $name:ident, $wire:ident, $remote:literal, $check:path, [$($serde:tt)*], { $($body:tt)* }) => {
        #[allow(
            dead_code,
            reason = "remote mirror: only its deserialize function is called"
        )]
        #[derive(serde::Deserialize)]
        #[serde(remote = $remote, $($serde)*)]
        enum $wire {
            $($body)*
        }

        impl<'de> serde::Deserialize<'de> for $name {
            fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                $crate::tagged::deserialize(deserializer, $check, |value| $wire::deserialize(value))
            }
        }
    };
}
