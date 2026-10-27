use serde::Deserialize;
use serde::de::IgnoredAny;
use serde_json::Value;

use super::{AITurn, InputTurn, Turn, TurnKind};
use crate::entry::{AgentOutput, IncomingEntry, PartialAgentOutput, TurnEntry};
use crate::error::{Error, ErrorCode};
use crate::id::{AgentId, TurnId};
use crate::value::{Author, ObservedAt};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct InputWire {
    #[serde(rename = "type")]
    _kind: IgnoredAny,
    id: TurnId,
    entries: Vec<Value>,
    #[serde(default, deserialize_with = "crate::entry::present")]
    author: bool,
    #[serde(default, deserialize_with = "crate::json::optional")]
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
    #[serde(default, deserialize_with = "crate::json::optional")]
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
    input_from_value_with(value, false)
}

pub(crate) fn projected_turn_from_value(value: Value) -> Result<Turn, Error> {
    match kind_of(&value)? {
        TurnKind::InputTurn => input_from_value_with(value, true).map(Turn::InputTurn),
        TurnKind::AITurn => ai_from_value(value).map(Turn::AITurn),
    }
}

fn entry_from_value(raw: Value, projected: bool) -> Result<IncomingEntry, Error> {
    match (projected, raw.get("type").and_then(Value::as_str)) {
        (true, Some("agent_output")) => {
            AgentOutput::from_projected_value(raw).map(IncomingEntry::AgentOutput)
        }
        (true, Some("partial_agent_output")) => {
            PartialAgentOutput::from_projected_value(raw).map(IncomingEntry::PartialAgentOutput)
        }
        _ => crate::json::decode_value::<IncomingEntry>(raw),
    }
}

fn input_from_value_with(value: Value, projected: bool) -> Result<InputTurn, Error> {
    expect(&value, TurnKind::InputTurn)?;
    let wire: InputWire = crate::json::decode_value(value)?;
    if wire.author {
        return Err(Error::new(ErrorCode::AuthorForbidden)
            .with_id(wire.id.as_uuid())
            .at("author"));
    }
    let mut entries = Vec::with_capacity(wire.entries.len());
    for (index, raw) in wire.entries.into_iter().enumerate() {
        let entry = entry_from_value(raw, projected)
            .map_err(|error| error.at(format!("entries[{index}]")))?;
        entries.push(entry);
    }
    let turn = if projected {
        InputTurn::projected(wire.id, entries)?
    } else {
        InputTurn::new(wire.id, entries)?
    };
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
