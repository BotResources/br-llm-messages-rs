use serde::Deserialize;
use serde::de::IgnoredAny;
use serde_json::Value;

use super::{AITurn, InputTurn, Turn, TurnKind};
use crate::entry::{IncomingEntry, TurnEntry};
use crate::error::{Error, ErrorCode};
use crate::id::{AgentId, TurnId};
use crate::value::{Author, ObservedAt};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct InputWire {
    #[serde(rename = "type")]
    _kind: IgnoredAny,
    id: TurnId,
    entry: Value,
    #[serde(default, deserialize_with = "crate::entry::present")]
    author: bool,
    observed_at: Option<ObservedAt>,
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
    observed_at: Option<ObservedAt>,
}

pub(crate) fn kind_of(value: &Value) -> Result<TurnKind, Error> {
    match value.get("type") {
        None => Err(Error::new(ErrorCode::VariantMismatch).at("type")),
        Some(Value::String(name)) => name.parse().map_err(|error: Error| error.at("type")),
        Some(_) => Err(Error::new(ErrorCode::VariantMismatch).at("type")),
    }
}

fn expect(value: &Value, wanted: TurnKind) -> Result<(), Error> {
    if kind_of(value)? == wanted {
        Ok(())
    } else {
        Err(Error::new(ErrorCode::VariantMismatch).at("type"))
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
    let entry = crate::json::decode_value::<IncomingEntry>(wire.entry)
        .map_err(|error| error.at("entry"))?;
    let turn = InputTurn::new(wire.id, entry)?;
    Ok(match wire.observed_at {
        Some(observed_at) => turn.with_observed_at(observed_at),
        None => turn,
    })
}

pub(crate) fn ai_from_value(value: Value) -> Result<AITurn, Error> {
    expect(&value, TurnKind::AITurn)?;
    let wire: AiWire = crate::json::decode_value(value)?;
    let mut entries = Vec::with_capacity(wire.entries.len());
    for (index, raw) in wire.entries.into_iter().enumerate() {
        let entry = crate::json::decode_value::<TurnEntry>(raw)
            .map_err(|error| error.at(format!("entries[{index}]")))?;
        entries.push(entry);
    }
    let turn = AITurn::from_parts(wire.id, wire.agent, wire.author, entries)?;
    Ok(match wire.observed_at {
        Some(observed_at) => turn.with_observed_at(observed_at),
        None => turn,
    })
}
