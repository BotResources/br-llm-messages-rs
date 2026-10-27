use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};

use uuid::Uuid;

use crate::opaque_json::OpaqueJson;

use std::collections::HashSet;

use super::declared::{places_of_incoming, turn_place};
use super::kind::TurnKind;
use crate::entry::IncomingEntry;
use crate::error::{Error, ErrorCode};
use crate::id::{AgentId, EntryId, TurnId};
use crate::value::ObservedAt;

#[derive(Debug, Clone, PartialEq)]
pub struct InputTurn {
    id: TurnId,
    entries: Vec<IncomingEntry>,
    observed_at: Option<ObservedAt>,
}

impl Serialize for InputTurn {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut turn = serializer
            .serialize_struct("InputTurn", 3 + usize::from(self.observed_at.is_some()))?;
        turn.serialize_field("type", TurnKind::InputTurn.as_str())?;
        turn.serialize_field("id", &self.id)?;
        turn.serialize_field("entries", &self.entries)?;
        if let Some(observed_at) = &self.observed_at {
            turn.serialize_field("observed_at", observed_at)?;
        }
        turn.end()
    }
}

impl InputTurn {
    pub fn new(id: TurnId, entries: Vec<IncomingEntry>) -> Result<Self, Error> {
        refuse_agent_output(&entries)?;
        InputTurn::projected(id, entries)
    }

    pub fn new_projected(id: TurnId, entries: Vec<IncomingEntry>) -> Result<Self, Error> {
        InputTurn::projected(id, entries)
    }

    pub(crate) fn projected(id: TurnId, entries: Vec<IncomingEntry>) -> Result<Self, Error> {
        if entries.is_empty() {
            return Err(Error::new(ErrorCode::InvalidRecordStructure)
                .with_id(id.as_uuid())
                .at("entries"));
        }
        let mut seen: HashSet<Uuid> = HashSet::from([id.as_uuid()]);
        for (index, entry) in entries.iter().enumerate() {
            for (declared, _) in places_of_incoming(entry, index) {
                if !seen.insert(declared) {
                    return Err(Error::new(ErrorCode::ReusedIdentity)
                        .with_id(declared)
                        .at(format!("entries[{index}]")));
                }
            }
        }
        Ok(InputTurn {
            id,
            entries,
            observed_at: None,
        })
    }

    pub(crate) fn translated(id: TurnId, entries: Vec<IncomingEntry>) -> Option<Self> {
        if entries.is_empty() {
            return None;
        }
        Some(InputTurn {
            id,
            entries,
            observed_at: None,
        })
    }

    pub fn with_observed_at(mut self, observed_at: ObservedAt) -> Self {
        self.observed_at = Some(observed_at);
        self
    }

    pub fn id(&self) -> TurnId {
        self.id
    }

    pub fn entries(&self) -> &[IncomingEntry] {
        &self.entries
    }

    pub(crate) fn outputs(&self) -> impl Iterator<Item = (usize, EntryId, AgentId)> {
        self.entries
            .iter()
            .enumerate()
            .filter_map(|(index, entry)| {
                entry
                    .projected()
                    .map(|projected| (index, projected.id, projected.agent))
            })
    }

    pub(crate) fn declared(&self) -> Vec<(Uuid, super::Place)> {
        let mut declared = vec![(self.id.as_uuid(), turn_place())];
        for (index, entry) in self.entries.iter().enumerate() {
            declared.extend(places_of_incoming(entry, index));
        }
        declared
    }

    pub fn observed_at(&self) -> Option<&ObservedAt> {
        self.observed_at.as_ref()
    }

    pub fn to_json(&self) -> Result<String, Error> {
        crate::json::encode(self)
    }

    pub fn from_json(text: &str) -> Result<Self, Error> {
        super::json::input_from_value(crate::json::parse(text)?)
    }

    pub fn to_value(&self) -> Result<OpaqueJson, Error> {
        crate::json::encode_value(self).map(OpaqueJson::from_inner)
    }

    pub fn from_value(value: OpaqueJson) -> Result<Self, Error> {
        let value = value.into_inner();
        super::json::input_from_value(value)
    }
}

pub(crate) fn refuse_agent_output(entries: &[IncomingEntry]) -> Result<(), Error> {
    for (index, entry) in entries.iter().enumerate() {
        if entry.projected().is_some() {
            return Err(Error::new(ErrorCode::DisallowedKind)
                .with_id(entry.id().as_uuid())
                .at(format!("entries[{index}]")));
        }
    }
    Ok(())
}
