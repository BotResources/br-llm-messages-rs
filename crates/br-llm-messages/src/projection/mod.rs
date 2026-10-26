mod json;
mod perspective;
mod validate;

pub use json::PROJECTED_THREAD_SCHEMA;
pub use perspective::Perspective;

pub(crate) use perspective::accumulated_text;

use crate::id::{AgentId, ConversationId};
use crate::turn::Turn;

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub struct ProjectedThread {
    conversation: ConversationId,
    agent: AgentId,
    turns: Vec<Turn>,
}

impl ProjectedThread {
    pub(crate) fn unchecked(
        conversation: ConversationId,
        agent: AgentId,
        turns: Vec<Turn>,
    ) -> Self {
        ProjectedThread {
            conversation,
            agent,
            turns,
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

    pub fn len(&self) -> usize {
        self.turns.len()
    }

    pub fn is_empty(&self) -> bool {
        self.turns.is_empty()
    }
}
