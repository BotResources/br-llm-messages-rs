use serde::{Deserialize, Serialize};

use super::{MailCorrelation, check_content, collect_content};
use crate::block::{IdSink, PlainBlock};
use crate::error::Error;
use crate::id::{AgentId, ConversationId, EntryId};
use crate::value::Author;

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "AgentMailWire")]
pub struct AgentMail {
    id: EntryId,
    sender: AgentId,
    author: Author,
    source_conversation: ConversationId,
    #[serde(skip_serializing_if = "Option::is_none")]
    in_reply_to: Option<MailCorrelation>,
    content: Vec<PlainBlock>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AgentMailWire {
    id: EntryId,
    sender: AgentId,
    author: Author,
    source_conversation: ConversationId,
    in_reply_to: Option<MailCorrelation>,
    content: Vec<PlainBlock>,
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
        })
    }

    pub fn with_in_reply_to(mut self, correlation: MailCorrelation) -> Self {
        self.in_reply_to = Some(correlation);
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

    pub fn in_reply_to(&self) -> Option<&MailCorrelation> {
        self.in_reply_to.as_ref()
    }

    pub fn content(&self) -> &[PlainBlock] {
        &self.content
    }

    pub(crate) fn collect_ids(&self, sink: &mut IdSink) -> Result<(), Error> {
        collect_content(self.id, &self.content, sink)
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
        Ok(match wire.in_reply_to {
            Some(correlation) => mail.with_in_reply_to(correlation),
            None => mail,
        })
    }
}
