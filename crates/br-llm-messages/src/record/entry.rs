use serde::{Deserialize, Serialize};

use super::kinds::{ActorRef, RepresentationState};
use super::schema::{as_object, check_object, entry_fields, header_fields};
use crate::entry::TurnEntryKind;
use crate::error::{Error, ErrorCode};
use crate::id::{EntryId, TurnId};
use crate::opaque_json::OpaqueJson;
use crate::value::{Author, ObservedAt};

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
    sender: Option<ActorRef>,
    author: Option<Author>,
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
        self.check_inner().map_err(|error| match error.object_id() {
            Some(_) => error,
            None => error.with_id(self.id.as_uuid()),
        })
    }

    fn check_inner(&self) -> Result<(), Error> {
        self.check_columns()?;
        let object = as_object(self.payload.inner())?;
        let checked = match (self.kind, self.state) {
            (_, RepresentationState::Complete) => check_object(object, &entry_fields(self.kind)),
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

    fn check_columns(&self) -> Result<(), Error> {
        let sender_ok = match (self.kind, &self.sender) {
            (TurnEntryKind::HumanMessage, Some(ActorRef::Human { .. })) => true,
            (
                TurnEntryKind::AgentRequest | TurnEntryKind::AgentMail,
                Some(ActorRef::Agent { .. }),
            ) => true,
            (
                TurnEntryKind::HumanMessage
                | TurnEntryKind::AgentRequest
                | TurnEntryKind::AgentMail,
                None,
            ) => return Err(Error::new(ErrorCode::InvalidContent).at("sender")),
            (_, None) => true,
            _ => false,
        };
        if !sender_ok {
            return Err(Error::new(ErrorCode::DisallowedKind).at("sender"));
        }
        let authored = matches!(
            self.kind,
            TurnEntryKind::AgentRequest | TurnEntryKind::AgentMail
        );
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
