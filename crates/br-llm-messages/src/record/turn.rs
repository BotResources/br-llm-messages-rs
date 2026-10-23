use serde::{Deserialize, Serialize};

use crate::error::{Error, ErrorCode};
use crate::id::{AgentId, TurnId};
use crate::turn::TurnKind;
use crate::value::Author;

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "TurnRecordWire")]
pub struct TurnRecord {
    id: TurnId,
    kind: TurnKind,
    position: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    agent: Option<AgentId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    author: Option<Author>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TurnRecordWire {
    id: TurnId,
    kind: TurnKind,
    position: u32,
    agent: Option<AgentId>,
    author: Option<Author>,
}

impl TurnRecord {
    pub fn new(
        id: TurnId,
        kind: TurnKind,
        position: u32,
        agent: Option<AgentId>,
        author: Option<Author>,
    ) -> Self {
        TurnRecord {
            id,
            kind,
            position,
            agent,
            author,
        }
    }

    pub fn id(&self) -> TurnId {
        self.id
    }

    pub fn kind(&self) -> TurnKind {
        self.kind
    }

    pub fn position(&self) -> u32 {
        self.position
    }

    pub fn agent(&self) -> Option<AgentId> {
        self.agent
    }

    pub fn author(&self) -> Option<&Author> {
        self.author.as_ref()
    }

    pub fn check_local(&self) -> Result<(), Error> {
        let located = |code| Error::new(code).with_id(self.id.as_uuid());
        match self.kind {
            TurnKind::AITurn => {
                if self.agent.is_none() {
                    return Err(located(ErrorCode::MissingContent).at("agent"));
                }
                if self.author.is_none() {
                    return Err(located(ErrorCode::MissingContent).at("author"));
                }
            }
            _ => {
                if self.author.is_some() {
                    return Err(located(ErrorCode::AuthorForbidden).at("author"));
                }
                if self.agent.is_some() {
                    return Err(located(ErrorCode::InvalidRecord).at("agent"));
                }
            }
        }
        Ok(())
    }
}

impl TryFrom<TurnRecordWire> for TurnRecord {
    type Error = Error;

    fn try_from(wire: TurnRecordWire) -> Result<Self, Error> {
        let record = TurnRecord::new(wire.id, wire.kind, wire.position, wire.agent, wire.author);
        record.check_local()?;
        Ok(record)
    }
}
