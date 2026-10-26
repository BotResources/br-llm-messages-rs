use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

use crate::opaque_json::OpaqueJson;
use uuid::Uuid;

use super::{
    AgentMail, AgentOutput, AgentRequest, HumanMessage, PartialAgentOutput, QueuePosition,
    RequestAccepted, TaskCompleted, TaskFailed, TaskStarted, TurnEntry, TurnEntryKind,
};
use crate::error::{Error, ErrorCode};
use crate::id::{AgentId, EntryId};
use crate::value::{Author, ObservedAt};

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
    AgentOutput(AgentOutput),
    PartialAgentOutput(PartialAgentOutput),
}

macro_rules! incoming_variants {
    ($($variant:ident($entry:ty)),+ $(,)?) => {
        impl IncomingEntry {
            pub fn kind(&self) -> Option<TurnEntryKind> {
                match self {
                    $(IncomingEntry::$variant(_) => Some(TurnEntryKind::$variant),)+
                    IncomingEntry::AgentOutput(_) | IncomingEntry::PartialAgentOutput(_) => None,
                }
            }

            pub fn id(&self) -> EntryId {
                match self {
                    $(IncomingEntry::$variant(entry) => entry.id(),)+
                    IncomingEntry::AgentOutput(entry) => entry.id(),
                    IncomingEntry::PartialAgentOutput(entry) => entry.id(),
                }
            }

            pub fn observed_at(&self) -> Option<&ObservedAt> {
                match self {
                    $(IncomingEntry::$variant(entry) => entry.observed_at(),)+
                    IncomingEntry::AgentOutput(_) | IncomingEntry::PartialAgentOutput(_) => None,
                }
            }

            pub(crate) fn recorded(entry: &TurnEntry) -> Option<IncomingEntry> {
                match entry {
                    $(TurnEntry::$variant(entry) => Some(IncomingEntry::$variant(entry.clone())),)+
                    TurnEntry::ModelResponse(_)
                    | TurnEntry::RuntimeToolResults(_)
                    | TurnEntry::TurnClosure(_) => None,
                }
            }

            pub(crate) fn projected_source(&self) -> Option<(EntryId, AgentId)> {
                match self {
                    IncomingEntry::AgentOutput(entry) => Some((entry.id(), entry.agent())),
                    IncomingEntry::PartialAgentOutput(entry) => Some((entry.id(), entry.agent())),
                    _ => None,
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

        impl TryFrom<IncomingEntry> for TurnEntry {
            type Error = Error;

            fn try_from(entry: IncomingEntry) -> Result<Self, Error> {
                match entry {
                    $(IncomingEntry::$variant(entry) => Ok(TurnEntry::$variant(entry)),)+
                    projected => Err(Error::new(ErrorCode::DisallowedKind)
                        .with_id(projected.id().as_uuid())),
                }
            }
        }

        impl TryFrom<TurnEntry> for IncomingEntry {
            type Error = Error;

            fn try_from(entry: TurnEntry) -> Result<Self, Error> {
                match entry {
                    $(TurnEntry::$variant(entry) => Ok(IncomingEntry::$variant(entry)),)+
                    other => Err(Error::new(ErrorCode::DisallowedKind)
                        .with_id(other.id().as_uuid())),
                }
            }
        }
    };
}

impl From<AgentOutput> for IncomingEntry {
    fn from(entry: AgentOutput) -> Self {
        IncomingEntry::AgentOutput(entry)
    }
}

impl From<PartialAgentOutput> for IncomingEntry {
    fn from(entry: PartialAgentOutput) -> Self {
        IncomingEntry::PartialAgentOutput(entry)
    }
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
    pub(crate) fn projected_author(&self) -> Option<&Author> {
        match self {
            IncomingEntry::AgentOutput(entry) => Some(entry.author()),
            IncomingEntry::PartialAgentOutput(entry) => Some(entry.author()),
            _ => None,
        }
    }

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
    let mut error = Error::new(ErrorCode::DisallowedKind);
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
