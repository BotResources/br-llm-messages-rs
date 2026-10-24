use serde::{Deserialize, Serialize};

use super::check_content;
use crate::block::PlainBlock;
use crate::error::Error;
use crate::id::{AgentId, ConversationId, EntryId};
use crate::value::{Author, ObservedAt};

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "AgentMailWire")]
pub struct AgentMail {
    id: EntryId,
    sender: AgentId,
    author: Author,
    source_conversation: ConversationId,
    #[serde(skip_serializing_if = "Option::is_none")]
    in_reply_to: Option<EntryId>,
    content: Vec<PlainBlock>,
    #[serde(skip_serializing_if = "Option::is_none")]
    observed_at: Option<ObservedAt>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AgentMailWire {
    id: EntryId,
    sender: AgentId,
    author: Author,
    source_conversation: ConversationId,
    in_reply_to: Option<EntryId>,
    content: Vec<PlainBlock>,
    observed_at: Option<ObservedAt>,
}

impl AgentMail {
    pub fn new(
        id: EntryId,
        sender: AgentId,
        author: Author,
        source_conversation: ConversationId,
        content: Vec<PlainBlock>,
    ) -> Result<Self, Error> {
        check_content(id, &content)?;
        Ok(AgentMail {
            id,
            sender,
            author,
            source_conversation,
            in_reply_to: None,
            content,
            observed_at: None,
        })
    }

    pub fn with_observed_at(mut self, observed_at: ObservedAt) -> Self {
        self.observed_at = Some(observed_at);
        self
    }

    pub fn with_in_reply_to(mut self, entry: EntryId) -> Self {
        self.in_reply_to = Some(entry);
        self
    }

    pub fn id(&self) -> EntryId {
        self.id
    }

    pub fn sender(&self) -> AgentId {
        self.sender
    }

    pub fn author(&self) -> &Author {
        &self.author
    }

    pub fn source_conversation(&self) -> ConversationId {
        self.source_conversation
    }

    pub fn in_reply_to(&self) -> Option<EntryId> {
        self.in_reply_to
    }

    pub fn content(&self) -> &[PlainBlock] {
        &self.content
    }

    pub fn observed_at(&self) -> Option<&ObservedAt> {
        self.observed_at.as_ref()
    }
}

impl TryFrom<AgentMailWire> for AgentMail {
    type Error = Error;

    fn try_from(wire: AgentMailWire) -> Result<Self, Error> {
        let mail = AgentMail::new(
            wire.id,
            wire.sender,
            wire.author,
            wire.source_conversation,
            wire.content,
        )?;
        let mail = match wire.in_reply_to {
            Some(entry) => mail.with_in_reply_to(entry),
            None => mail,
        };
        Ok(match wire.observed_at {
            Some(observed_at) => mail.with_observed_at(observed_at),
            None => mail,
        })
    }
}
