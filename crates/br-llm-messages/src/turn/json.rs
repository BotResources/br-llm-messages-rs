use serde::Deserialize;
use serde::de::IgnoredAny;
use serde_json::Value;

use super::{AITurn, InputTurn, Turn, TurnKind};
use crate::entry::{IncomingEntry, TurnEntry};
use crate::error::{Error, ErrorCode};
use crate::id::{AgentId, TurnId};
use crate::value::Author;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct InputWire {
    #[serde(rename = "type")]
    _kind: IgnoredAny,
    id: TurnId,
    entry: Value,
    #[serde(default, deserialize_with = "crate::entry::present")]
    author: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AiWire {
    #[serde(rename = "type")]
    _kind: IgnoredAny,
    id: TurnId,
    agent: AgentId,
    author: Author,
    entries: Vec<Value>,
}

pub(crate) fn kind_of(value: &Value) -> Result<TurnKind, Error> {
    match value.get("type") {
        None => Err(Error::new(ErrorCode::MissingContent).at("type")),
        Some(Value::String(name)) => name.parse().map_err(|error: Error| error.at("type")),
        Some(_) => Err(Error::new(ErrorCode::InvalidContent).at("type")),
    }
}

fn expect(value: &Value, wanted: TurnKind) -> Result<(), Error> {
    if kind_of(value)? == wanted {
        Ok(())
    } else {
        Err(Error::new(ErrorCode::UnsupportedVariant).at("type"))
    }
}

pub(crate) fn turn_from_value(value: Value) -> Result<Turn, Error> {
    match kind_of(&value)? {
        TurnKind::InputTurn => input_from_value(value).map(Turn::InputTurn),
        TurnKind::AITurn => ai_from_value(value).map(Turn::AITurn),
    }
}

pub(crate) fn input_from_value(value: Value) -> Result<InputTurn, Error> {
    expect(&value, TurnKind::InputTurn)?;
    let wire: InputWire = crate::json::decode_value(value)?;
    if wire.author {
        return Err(Error::new(ErrorCode::AuthorForbidden)
            .with_id(wire.id.as_uuid())
            .at("author"));
    }
    let entry = IncomingEntry::from_value(wire.entry).map_err(|error| error.at("entry"))?;
    InputTurn::new(wire.id, entry)
}

pub(crate) fn ai_from_value(value: Value) -> Result<AITurn, Error> {
    expect(&value, TurnKind::AITurn)?;
    let wire: AiWire = crate::json::decode_value(value)?;
    let mut entries = Vec::with_capacity(wire.entries.len());
    for (index, raw) in wire.entries.into_iter().enumerate() {
        let entry =
            TurnEntry::from_value(raw).map_err(|error| error.at(format!("entries[{index}]")))?;
        entries.push(entry);
    }
    AITurn::from_parts(wire.id, wire.agent, wire.author, entries)
}
