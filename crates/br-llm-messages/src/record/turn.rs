use serde::{Deserialize, Serialize};

use crate::error::{Error, ErrorCode};
use crate::id::{AgentId, TurnId};
use crate::turn::TurnKind;
use crate::value::{Author, ObservedAt};

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "TurnRecordWire")]
pub struct TurnRecord {
    id: TurnId,
    kind: TurnKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    agent: Option<AgentId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    author: Option<Author>,
    #[serde(skip_serializing_if = "Option::is_none")]
    observed_at: Option<ObservedAt>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TurnRecordWire {
    id: TurnId,
    kind: TurnKind,
    agent: Option<AgentId>,
    author: Option<Author>,
    observed_at: Option<ObservedAt>,
}

impl TurnRecord {
    pub fn new(id: TurnId, kind: TurnKind) -> Self {
        TurnRecord {
            id,
            kind,
            agent: None,
            author: None,
            observed_at: None,
        }
    }

    pub fn with_agent(mut self, agent: AgentId) -> Self {
        self.agent = Some(agent);
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

    pub fn id(&self) -> TurnId {
        self.id
    }

    pub fn kind(&self) -> TurnKind {
        self.kind
    }

    pub fn agent(&self) -> Option<AgentId> {
        self.agent
    }

    pub fn author(&self) -> Option<&Author> {
        self.author.as_ref()
    }

    pub fn observed_at(&self) -> Option<&ObservedAt> {
        self.observed_at.as_ref()
    }

    pub fn check_local(&self) -> Result<(), Error> {
        let located = |code| Error::new(code).with_id(self.id.as_uuid());
        match self.kind {
            TurnKind::AITurn => {
                if self.agent.is_none() {
                    return Err(located(ErrorCode::InvalidContent).at("agent"));
                }
                if self.author.is_none() {
                    return Err(located(ErrorCode::InvalidContent).at("author"));
                }
            }
            _ => {
                if self.author.is_some() {
                    return Err(located(ErrorCode::AuthorForbidden).at("author"));
                }
                if self.agent.is_some() {
                    return Err(located(ErrorCode::DisallowedKind).at("agent"));
                }
            }
        }
        Ok(())
    }
}

impl TryFrom<TurnRecordWire> for TurnRecord {
    type Error = Error;

    fn try_from(wire: TurnRecordWire) -> Result<Self, Error> {
        let mut record = TurnRecord::new(wire.id, wire.kind);
        record.agent = wire.agent;
        record.author = wire.author;
        record.observed_at = wire.observed_at;
        record.check_local()?;
        Ok(record)
    }
}
