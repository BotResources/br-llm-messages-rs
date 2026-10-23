use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};

use crate::opaque_json::OpaqueJson;

use super::declared::places_of_incoming;
use super::kind::TurnKind;
use crate::entry::IncomingEntry;
use crate::error::{Error, ErrorCode};
use crate::id::TurnId;
use crate::value::ObservedAt;

#[derive(Debug, Clone, PartialEq)]
pub struct InputTurn {
    id: TurnId,
    entry: IncomingEntry,
    observed_at: Option<ObservedAt>,
}

impl Serialize for InputTurn {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut turn = serializer.serialize_struct("InputTurn", 4)?;
        turn.serialize_field("type", TurnKind::InputTurn.as_str())?;
        turn.serialize_field("id", &self.id)?;
        turn.serialize_field("entry", &self.entry)?;
        if let Some(observed_at) = &self.observed_at {
            turn.serialize_field("observed_at", observed_at)?;
        }
        turn.end()
    }
}

impl InputTurn {
    pub fn new(id: TurnId, entry: impl Into<IncomingEntry>) -> Result<Self, Error> {
        let entry = entry.into();
        let clash = places_of_incoming(&entry, 0)
            .into_iter()
            .any(|(declared, _)| declared == id.as_uuid());
        if clash {
            return Err(Error::new(ErrorCode::ReusedIdentity).with_id(id.as_uuid()));
        }
        Ok(InputTurn {
            id,
            entry,
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

    pub fn entry(&self) -> &IncomingEntry {
        &self.entry
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
