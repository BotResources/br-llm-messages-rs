use serde::{Deserialize, Serialize};

use super::kinds::{ActorRef, RepresentationState};
use super::schema::{as_object, check_object, entry_fields, header_fields};
use crate::entry::TurnEntryKind;
use crate::error::{Error, ErrorCode};
use crate::id::{EntryId, TurnId};
use crate::opaque_json::OpaqueJson;
use crate::value::{Author, ObservedAt};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Ending {
    Required,
    Optional,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ReducedRole {
    Incoming,
    Response(RepresentationState),
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "EntryRecordWire")]
pub struct EntryRecord {
    id: EntryId,
    turn: TurnId,
    kind: TurnEntryKind,
    state: RepresentationState,
    #[serde(skip_serializing_if = "Option::is_none")]
    sender: Option<ActorRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    author: Option<Author>,
    #[serde(skip_serializing_if = "Option::is_none")]
    observed_at: Option<ObservedAt>,
    payload: OpaqueJson,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EntryRecordWire {
    id: EntryId,
    turn: TurnId,
    kind: TurnEntryKind,
    state: RepresentationState,
    #[serde(default, deserialize_with = "crate::json::optional")]
    sender: Option<ActorRef>,
    #[serde(default, deserialize_with = "crate::json::optional")]
    author: Option<Author>,
    #[serde(default, deserialize_with = "crate::json::optional")]
    observed_at: Option<ObservedAt>,
    payload: OpaqueJson,
}

impl EntryRecord {
    pub fn new(
        id: EntryId,
        turn: TurnId,
        kind: TurnEntryKind,
        state: RepresentationState,
        payload: OpaqueJson,
    ) -> Self {
        EntryRecord {
            id,
            turn,
            kind,
            state,
            sender: None,
            author: None,
            observed_at: None,
            payload,
        }
    }

    pub fn with_sender(mut self, sender: ActorRef) -> Self {
        self.sender = Some(sender);
        self
    }

    pub fn with_author(mut self, author: Author) -> Self {
        self.author = Some(author);
        self
    }

    pub fn with_observed_at(mut self, observed_at: ObservedAt) -> Self {
        self.observed_at = Some(observed_at);
        self
    }

    pub fn id(&self) -> EntryId {
        self.id
    }

    pub fn turn(&self) -> TurnId {
        self.turn
    }

    pub fn kind(&self) -> TurnEntryKind {
        self.kind
    }

    pub fn state(&self) -> RepresentationState {
        self.state
    }

    pub fn sender(&self) -> Option<&ActorRef> {
        self.sender.as_ref()
    }

    pub fn author(&self) -> Option<&Author> {
        self.author.as_ref()
    }

    pub fn observed_at(&self) -> Option<&ObservedAt> {
        self.observed_at.as_ref()
    }

    pub fn payload(&self) -> &OpaqueJson {
        &self.payload
    }

    pub fn check_local(&self) -> Result<(), Error> {
        self.check_with(Ending::Required)
    }

    fn check_with(&self, ending: Ending) -> Result<(), Error> {
        self.check_inner(ending)
            .map_err(|error| match error.object_id() {
                Some(_) => error,
                None => error.with_id(self.id.as_uuid()),
            })
    }

    fn check_inner(&self, ending: Ending) -> Result<(), Error> {
        self.check_columns()?;
        let object = as_object(self.payload.inner())?;
        let checked = match (self.kind, self.state) {
            (_, RepresentationState::Complete) => {
                let mut fields = entry_fields(self.kind);
                if ending == Ending::Optional && self.kind == TurnEntryKind::ModelResponse {
                    fields
                        .iter_mut()
                        .filter(|field| field.key == "ending")
                        .for_each(|field| field.required = false);
                }
                check_object(object, &fields)
            }
            (TurnEntryKind::ModelResponse, RepresentationState::Producing) => {
                check_object(object, &header_fields(false))
            }
            (TurnEntryKind::ModelResponse, RepresentationState::Interrupted) => {
                check_object(object, &header_fields(true))
            }
            _ => return Err(Error::new(ErrorCode::DisallowedKind).at("state")),
        };
        checked.map_err(|error| error.at("payload"))
    }

    pub(super) fn reduced_role(&self) -> Result<ReducedRole, Error> {
        if self.kind.is_incoming() {
            Ok(ReducedRole::Incoming)
        } else if self.kind == TurnEntryKind::ModelResponse {
            Ok(ReducedRole::Response(self.state))
        } else {
            Err(Error::new(ErrorCode::DisallowedKind)
                .with_id(self.id.as_uuid())
                .at("kind"))
        }
    }

    pub(super) fn check_reduced(&self) -> Result<(), Error> {
        match self.reduced_role()? {
            ReducedRole::Incoming => self.check_local(),
            ReducedRole::Response(_) => self.check_with(Ending::Optional),
        }
    }

    fn check_columns(&self) -> Result<(), Error> {
        let (sender_ok, authored) = match self.kind {
            TurnEntryKind::HumanMessage => (
                match &self.sender {
                    Some(ActorRef::Human { .. }) => true,
                    Some(ActorRef::Agent { .. }) => false,
                    None => return Err(Error::new(ErrorCode::InvalidContent).at("sender")),
                },
                false,
            ),
            TurnEntryKind::AgentRequest | TurnEntryKind::AgentMail => (
                match &self.sender {
                    Some(ActorRef::Agent { .. }) => true,
                    Some(ActorRef::Human { .. }) => false,
                    None => return Err(Error::new(ErrorCode::InvalidContent).at("sender")),
                },
                true,
            ),
            TurnEntryKind::RequestAccepted
            | TurnEntryKind::QueuePosition
            | TurnEntryKind::TaskStarted
            | TurnEntryKind::TaskCompleted
            | TurnEntryKind::TaskFailed
            | TurnEntryKind::RuntimeNotice
            | TurnEntryKind::ModelResponse
            | TurnEntryKind::RuntimeToolResults
            | TurnEntryKind::TurnClosure => (self.sender.is_none(), false),
        };
        if !sender_ok {
            return Err(Error::new(ErrorCode::DisallowedKind).at("sender"));
        }
        match (&self.author, authored) {
            (None, true) => Err(Error::new(ErrorCode::InvalidContent).at("author")),
            (Some(_), false) => Err(Error::new(ErrorCode::AuthorForbidden).at("author")),
            _ => Ok(()),
        }
    }
}

impl TryFrom<EntryRecordWire> for EntryRecord {
    type Error = Error;

    fn try_from(wire: EntryRecordWire) -> Result<Self, Error> {
        let record = EntryRecord {
            id: wire.id,
            turn: wire.turn,
            kind: wire.kind,
            state: wire.state,
            sender: wire.sender,
            author: wire.author,
            observed_at: wire.observed_at,
            payload: wire.payload,
        };
        record.check_local()?;
        Ok(record)
    }
}
