use crate::block::{ContentBlock, PlainBlock};
use crate::entry::{IncomingEntry, ModelResponse, TurnEntry};
use crate::id::{AgentId, EntryId, TurnId};
use crate::turn::AITurn;
use crate::value::Author;

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicResponse {
    entry: EntryId,
    blocks: Vec<PlainBlock>,
}

impl PublicResponse {
    pub fn entry(&self) -> EntryId {
        self.entry
    }

    pub fn blocks(&self) -> &[PlainBlock] {
        &self.blocks
    }

    pub(crate) fn of(response: &ModelResponse) -> Option<PublicResponse> {
        let blocks: Vec<PlainBlock> = response
            .blocks()
            .iter()
            .filter_map(|block| match block {
                ContentBlock::Text(text) => Some(PlainBlock::Text(text.without_replay())),
                ContentBlock::Image(image) => Some(PlainBlock::Image(image.clone())),
                ContentBlock::StructuredContent(structured) => {
                    Some(PlainBlock::StructuredContent(structured.clone()))
                }
                _ => None,
            })
            .collect();
        if blocks.is_empty() {
            None
        } else {
            Some(PublicResponse {
                entry: response.id(),
                blocks,
            })
        }
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OtherItem {
    Incoming(IncomingEntry),
    Public(PublicResponse),
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OtherTurn {
    id: TurnId,
    agent: AgentId,
    author: Author,
    items: Vec<OtherItem>,
}

impl OtherTurn {
    pub fn id(&self) -> TurnId {
        self.id
    }

    pub fn agent(&self) -> AgentId {
        self.agent
    }

    pub fn author(&self) -> &Author {
        &self.author
    }

    pub fn items(&self) -> &[OtherItem] {
        &self.items
    }

    pub(crate) fn of(turn: &AITurn) -> OtherTurn {
        let items = turn
            .entries()
            .iter()
            .filter_map(|entry| match entry {
                TurnEntry::ModelResponse(response) => {
                    PublicResponse::of(response).map(OtherItem::Public)
                }
                other => IncomingEntry::try_from(other).ok().map(OtherItem::Incoming),
            })
            .collect();
        OtherTurn {
            id: turn.id(),
            agent: turn.agent(),
            author: turn.author().clone(),
            items,
        }
    }
}
