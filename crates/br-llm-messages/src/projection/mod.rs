mod other;
mod own;

pub use other::{OtherItem, OtherTurn};
pub use own::OwnTurn;

use crate::conversation::Conversation;
use crate::id::{AgentId, TurnId};
use crate::turn::{InputTurn, Turn};

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub struct Perspective {
    agent: AgentId,
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub enum ProjectedTurn {
    Input(InputTurn),
    Own(OwnTurn),
    Other(OtherTurn),
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub struct Projection {
    turns: Vec<ProjectedTurn>,
}

impl Perspective {
    pub fn of(agent: AgentId) -> Self {
        Perspective { agent }
    }

    pub fn agent(&self) -> AgentId {
        self.agent
    }

    pub fn project(&self, conversation: &Conversation) -> Projection {
        self.project_turns(conversation.turns())
    }

    pub fn project_turns(&self, turns: &[Turn]) -> Projection {
        let turns = turns
            .iter()
            .map(|turn| match turn {
                Turn::InputTurn(input) => ProjectedTurn::Input(input.clone()),
                Turn::AITurn(ai) if ai.agent() == self.agent => ProjectedTurn::Own(OwnTurn::of(ai)),
                Turn::AITurn(ai) => ProjectedTurn::Other(OtherTurn::of(ai)),
            })
            .collect();
        Projection { turns }
    }
}

impl ProjectedTurn {
    pub fn id(&self) -> TurnId {
        match self {
            ProjectedTurn::Input(turn) => turn.id(),
            ProjectedTurn::Own(turn) => turn.id(),
            ProjectedTurn::Other(turn) => turn.id(),
        }
    }
}

impl Projection {
    pub fn turns(&self) -> &[ProjectedTurn] {
        &self.turns
    }

    pub fn into_turns(self) -> Vec<ProjectedTurn> {
        self.turns
    }

    pub fn len(&self) -> usize {
        self.turns.len()
    }

    pub fn is_empty(&self) -> bool {
        self.turns.is_empty()
    }
}
