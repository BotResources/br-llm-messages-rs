use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

use crate::opaque_json::OpaqueJson;
use uuid::Uuid;

use super::{
    AgentMail, AgentRequest, HumanMessage, QueuePosition, RequestAccepted, TaskCompleted,
    TaskFailed, TaskStarted, TurnEntry, TurnEntryKind,
};
use crate::error::{Error, ErrorCode};
use crate::id::EntryId;

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum IncomingEntry {
    HumanMessage(HumanMessage),
    AgentRequest(AgentRequest),
    AgentMail(AgentMail),
    RequestAccepted(RequestAccepted),
    QueuePosition(QueuePosition),
    TaskStarted(TaskStarted),
    TaskCompleted(TaskCompleted),
    TaskFailed(TaskFailed),
}

macro_rules! incoming_variants {
    ($($variant:ident($entry:ty)),+ $(,)?) => {
        impl IncomingEntry {
            pub fn kind(&self) -> TurnEntryKind {
                match self {
                    $(IncomingEntry::$variant(_) => TurnEntryKind::$variant,)+
                }
            }

            pub fn id(&self) -> EntryId {
                match self {
                    $(IncomingEntry::$variant(entry) => entry.id(),)+
                }
            }
        }

        $(
            impl From<$entry> for IncomingEntry {
                fn from(entry: $entry) -> Self {
                    IncomingEntry::$variant(entry)
                }
            }
        )+

        impl From<IncomingEntry> for TurnEntry {
            fn from(entry: IncomingEntry) -> Self {
                match entry {
                    $(IncomingEntry::$variant(entry) => TurnEntry::$variant(entry),)+
                }
            }
        }

        impl TryFrom<TurnEntry> for IncomingEntry {
            type Error = Error;

            fn try_from(entry: TurnEntry) -> Result<Self, Error> {
                match entry {
                    $(TurnEntry::$variant(entry) => Ok(IncomingEntry::$variant(entry)),)+
                    other => Err(Error::new(ErrorCode::DisallowedEntryKind)
                        .with_id(other.id().as_uuid())),
                }
            }
        }
    };
}

incoming_variants!(
    HumanMessage(HumanMessage),
    AgentRequest(AgentRequest),
    AgentMail(AgentMail),
    RequestAccepted(RequestAccepted),
    QueuePosition(QueuePosition),
    TaskStarted(TaskStarted),
    TaskCompleted(TaskCompleted),
    TaskFailed(TaskFailed),
);

impl TryFrom<&TurnEntry> for IncomingEntry {
    type Error = Error;

    fn try_from(entry: &TurnEntry) -> Result<Self, Error> {
        IncomingEntry::try_from(entry.clone())
    }
}

impl IncomingEntry {
    pub fn to_json(&self) -> Result<String, Error> {
        crate::json::encode(self)
    }

    pub fn from_json(text: &str) -> Result<Self, Error> {
        IncomingEntry::from_checked(crate::json::parse(text)?)
    }

    pub fn to_value(&self) -> Result<OpaqueJson, Error> {
        crate::json::encode_value(self).map(OpaqueJson::from_inner)
    }

    pub fn from_value(value: OpaqueJson) -> Result<Self, Error> {
        IncomingEntry::from_checked(value.into_inner())
    }

    fn from_checked(value: Value) -> Result<Self, Error> {
        refuse_non_incoming(&value)?;
        let entry: TurnEntry = crate::json::decode_value(value)?;
        IncomingEntry::try_from(entry)
    }
}

fn refuse_non_incoming(value: &Value) -> Result<(), Error> {
    let Some(kind) = value
        .get("type")
        .and_then(Value::as_str)
        .and_then(|name| name.parse::<TurnEntryKind>().ok())
    else {
        return Ok(());
    };
    if kind.is_incoming() {
        return Ok(());
    }
    let mut error = Error::new(ErrorCode::DisallowedEntryKind);
    if let Some(id) = value
        .get("id")
        .and_then(Value::as_str)
        .and_then(|text| Uuid::try_parse(text).ok())
    {
        error = error.with_id(id);
    }
    Err(error)
}

impl<'de> Deserialize<'de> for IncomingEntry {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = crate::json::unique_value(deserializer)?;
        IncomingEntry::from_checked(value).map_err(serde::de::Error::custom)
    }
}
