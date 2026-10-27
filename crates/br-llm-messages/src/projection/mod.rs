mod check;
mod grow;
mod json;
mod lookup;
mod output;
mod pending;
mod perspective;
mod source;
mod validate;

pub use json::PROJECTED_THREAD_SCHEMA;
pub use pending::PendingResponse;
pub use perspective::Perspective;

pub(crate) use output::{Output, Owner, Progress, accumulated_text, usable};
pub(crate) use source::{Foreign, Item, Source, Sources};

use crate::conversation::Index;
use crate::id::{AgentId, ConversationId};
use crate::turn::Turn;

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub struct ProjectedThread {
    conversation: ConversationId,
    agent: AgentId,
    turns: Vec<Turn>,
    index: Index,
}

impl ProjectedThread {
    pub(crate) fn unchecked(
        conversation: ConversationId,
        agent: AgentId,
        turns: Vec<Turn>,
    ) -> Self {
        let mut index = Index::seeded(conversation);
        for (position, turn) in turns.iter().enumerate() {
            index.insert(position, turn.declared());
        }
        ProjectedThread {
            conversation,
            agent,
            turns,
            index,
        }
    }

    pub fn conversation(&self) -> ConversationId {
        self.conversation
    }

    pub fn agent(&self) -> AgentId {
        self.agent
    }

    pub fn turns(&self) -> &[Turn] {
        &self.turns
    }

    pub fn into_turns(self) -> Vec<Turn> {
        self.turns
    }
}
