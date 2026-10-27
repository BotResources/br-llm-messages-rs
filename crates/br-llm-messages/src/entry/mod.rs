mod agent_mail;
mod agent_request;
mod closure;
mod human;
mod incoming;
mod kind;
mod metadata;
mod notice;
mod output;
mod request_origin;
mod response;
mod results;
mod runtime_notice;
mod task;

pub use agent_mail::AgentMail;
pub use agent_request::AgentRequest;
pub use closure::{ClosureReason, TurnClosure};
pub use human::HumanMessage;
pub use incoming::IncomingEntry;
pub use kind::TurnEntryKind;
pub use metadata::ResponseMetadata;
pub use notice::{QueuePosition, RequestAccepted};
pub use output::{AgentOutput, PartialAgentOutput};
pub use request_origin::RequestOrigin;
pub use response::ModelResponse;
pub use results::RuntimeToolResults;
pub use runtime_notice::RuntimeNotice;
pub use task::{TaskCompleted, TaskFailed, TaskStarted};

use serde::de::IgnoredAny;
use serde::{Deserialize, Deserializer};

use crate::opaque_json::OpaqueJson;

use crate::block::{IdSink, PlainBlock};
use crate::error::{Error, ErrorCode};
use crate::id::EntryId;
use crate::value::ObservedAt;

pub(crate) fn present<'de, D: Deserializer<'de>>(deserializer: D) -> Result<bool, D::Error> {
    IgnoredAny::deserialize(deserializer).map(|_| true)
}

pub(crate) fn refuse_author(present: bool, id: EntryId) -> Result<(), Error> {
    if present {
        Err(Error::new(ErrorCode::AuthorForbidden)
            .with_id(id.as_uuid())
            .at("author"))
    } else {
        Ok(())
    }
}

pub(crate) fn locate(error: Error, id: EntryId) -> Error {
    if error.object_id().is_some() {
        error
    } else {
        error.with_id(id.as_uuid())
    }
}

pub(crate) fn collect_content(
    entry: EntryId,
    content: &[PlainBlock],
    sink: &mut IdSink,
) -> Result<(), Error> {
    sink.insert(entry.as_uuid())?;
    PlainBlock::collect_all(content, sink).map_err(|error| error.at("content"))
}

pub(crate) fn check_content(entry: EntryId, content: &[PlainBlock]) -> Result<(), Error> {
    if content.is_empty() {
        return Err(Error::new(ErrorCode::InvalidContent)
            .with_id(entry.as_uuid())
            .at("content"));
    }
    collect_content(entry, content, &mut IdSink::default())
}

tagged_enum! {
    #[non_exhaustive]
    #[derive(Debug, Clone, PartialEq)]
    pub enum TurnEntry, TurnEntryWire, remote = "TurnEntry", check = crate::entry::kind::refuse_type, [tag = "type", rename_all = "snake_case"] {
        HumanMessage(HumanMessage),
        AgentRequest(AgentRequest),
        AgentMail(AgentMail),
        RequestAccepted(RequestAccepted),
        QueuePosition(QueuePosition),
        TaskStarted(TaskStarted),
        TaskCompleted(TaskCompleted),
        TaskFailed(TaskFailed),
        RuntimeNotice(RuntimeNotice),
        ModelResponse(ModelResponse),
        RuntimeToolResults(RuntimeToolResults),
        TurnClosure(TurnClosure),
    }
}

macro_rules! entry_variants {
    ($($variant:ident($entry:ty)),+ $(,)?) => {
        impl TurnEntry {
            pub fn kind(&self) -> TurnEntryKind {
                match self {
                    $(TurnEntry::$variant(_) => TurnEntryKind::$variant,)+
                }
            }

            pub fn id(&self) -> EntryId {
                match self {
                    $(TurnEntry::$variant(entry) => entry.id(),)+
                }
            }

            pub fn observed_at(&self) -> Option<&ObservedAt> {
                match self {
                    $(TurnEntry::$variant(entry) => entry.observed_at(),)+
                }
            }
        }

        $(
            impl From<$entry> for TurnEntry {
                fn from(entry: $entry) -> Self {
                    TurnEntry::$variant(entry)
                }
            }
        )+
    };
}

entry_variants!(
    HumanMessage(HumanMessage),
    AgentRequest(AgentRequest),
    AgentMail(AgentMail),
    RequestAccepted(RequestAccepted),
    QueuePosition(QueuePosition),
    TaskStarted(TaskStarted),
    TaskCompleted(TaskCompleted),
    TaskFailed(TaskFailed),
    RuntimeNotice(RuntimeNotice),
    ModelResponse(ModelResponse),
    RuntimeToolResults(RuntimeToolResults),
    TurnClosure(TurnClosure),
);

impl TurnEntry {
    pub fn is_incoming(&self) -> bool {
        self.kind().is_incoming()
    }

    pub fn to_json(&self) -> Result<String, Error> {
        crate::json::encode(self)
    }

    pub fn from_json(text: &str) -> Result<Self, Error> {
        crate::json::decode(text)
    }

    pub fn to_value(&self) -> Result<OpaqueJson, Error> {
        crate::json::encode_value(self).map(OpaqueJson::from_inner)
    }

    pub fn from_value(value: OpaqueJson) -> Result<Self, Error> {
        let value = value.into_inner();
        crate::json::decode_value(value)
    }
}
