use serde::{Deserialize, Serialize};

use super::{RequestOrigin, check_content, collect_content};
use crate::block::{IdSink, PlainBlock};
use crate::error::Error;
use crate::id::{AgentId, ConversationId, EntryId};
use crate::value::Author;

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "AgentRequestWire")]
pub struct AgentRequest {
    id: EntryId,
    requester: AgentId,
    author: Author,
    source_conversation: ConversationId,
    origin: RequestOrigin,
    content: Vec<PlainBlock>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AgentRequestWire {
    id: EntryId,
    requester: AgentId,
    author: Author,
    source_conversation: ConversationId,
    origin: RequestOrigin,
    content: Vec<PlainBlock>,
}

impl AgentRequest {
    pub fn new(
        id: EntryId,
        requester: AgentId,
        author: Author,
        source_conversation: ConversationId,
        origin: RequestOrigin,
        content: Vec<PlainBlock>,
    ) -> Result<Self, Error> {
        check_content(id, &content)?;
        Ok(AgentRequest {
            id,
            requester,
            author,
            source_conversation,
            origin,
            content,
        })
    }

    pub fn id(&self) -> EntryId {
        self.id
    }

    pub fn requester(&self) -> AgentId {
        self.requester
    }

    pub fn author(&self) -> &Author {
        &self.author
    }

    pub fn source_conversation(&self) -> ConversationId {
        self.source_conversation
    }

    pub fn origin(&self) -> &RequestOrigin {
        &self.origin
    }

    pub fn content(&self) -> &[PlainBlock] {
        &self.content
    }

    pub(crate) fn collect_ids(&self, sink: &mut IdSink) -> Result<(), Error> {
        collect_content(self.id, &self.content, sink)
    }
}

impl TryFrom<AgentRequestWire> for AgentRequest {
    type Error = Error;

    fn try_from(wire: AgentRequestWire) -> Result<Self, Error> {
        AgentRequest::new(
            wire.id,
            wire.requester,
            wire.author,
            wire.source_conversation,
            wire.origin,
            wire.content,
        )
    }
}
