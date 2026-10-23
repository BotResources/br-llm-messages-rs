use serde::{Deserialize, Serialize};

use super::{RequestOrigin, check_content, collect_content};
use crate::block::{IdSink, PlainBlock};
use crate::error::Error;
use crate::id::{AgentId, ConversationId, EntryId};
use crate::value::{Author, ObservedAt};

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
    #[serde(skip_serializing_if = "Option::is_none")]
    observed_at: Option<ObservedAt>,
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
    observed_at: Option<ObservedAt>,
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
            observed_at: None,
        })
    }

    pub fn with_observed_at(mut self, observed_at: ObservedAt) -> Self {
        self.observed_at = Some(observed_at);
        self
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

    pub fn observed_at(&self) -> Option<&ObservedAt> {
        self.observed_at.as_ref()
    }

    pub(crate) fn collect_ids(&self, sink: &mut IdSink) -> Result<(), Error> {
        collect_content(self.id, &self.content, sink)
    }
}

impl TryFrom<AgentRequestWire> for AgentRequest {
    type Error = Error;

    fn try_from(wire: AgentRequestWire) -> Result<Self, Error> {
        let mut request = AgentRequest::new(
            wire.id,
            wire.requester,
            wire.author,
            wire.source_conversation,
            wire.origin,
            wire.content,
        )?;
        request.observed_at = wire.observed_at;
        Ok(request)
    }
}
