use crate::block::{ContentBlock, TextBlock};
use crate::entry::{ModelResponse, TurnEntry};
use crate::id::{AgentId, EntryId, TurnId};
use crate::turn::AITurn;
use crate::value::Author;

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub struct OtherItem {
    entry: EntryId,
    agent: AgentId,
    author: Author,
    texts: Vec<TextBlock>,
}

impl OtherItem {
    pub fn entry(&self) -> EntryId {
        self.entry
    }

    pub fn agent(&self) -> AgentId {
        self.agent
    }

    pub fn author(&self) -> &Author {
        &self.author
    }

    pub fn texts(&self) -> &[TextBlock] {
        &self.texts
    }

    fn of(turn: &AITurn, response: &ModelResponse) -> Option<OtherItem> {
        let texts: Vec<TextBlock> = response
            .blocks()
            .iter()
            .filter_map(|block| match block {
                ContentBlock::Text(text) => Some(text.text_only()),
                _ => None,
            })
            .collect();
        if texts.is_empty() {
            return None;
        }
        Some(OtherItem {
            entry: response.id(),
            agent: turn.agent(),
            author: turn.author().clone(),
            texts,
        })
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub struct OtherTurn {
    id: TurnId,
    items: Vec<OtherItem>,
}

impl OtherTurn {
    pub fn id(&self) -> TurnId {
        self.id
    }

    pub fn items(&self) -> &[OtherItem] {
        &self.items
    }

    pub(crate) fn of(turn: &AITurn) -> OtherTurn {
        let items = turn
            .entries()
            .iter()
            .filter_map(|entry| match entry {
                TurnEntry::ModelResponse(response) => OtherItem::of(turn, response),
                _ => None,
            })
            .collect();
        OtherTurn {
            id: turn.id(),
            items,
        }
    }
}
