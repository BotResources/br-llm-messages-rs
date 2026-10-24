use crate::id::{BlockId, TurnId};
use crate::turn::AITurn;

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub struct OwnTurn {
    turn: AITurn,
    unresolved: Vec<BlockId>,
}

impl OwnTurn {
    pub fn id(&self) -> TurnId {
        self.turn.id()
    }

    pub fn turn(&self) -> &AITurn {
        &self.turn
    }

    pub fn into_turn(self) -> AITurn {
        self.turn
    }

    pub fn unresolved_calls(&self) -> &[BlockId] {
        &self.unresolved
    }

    pub(crate) fn of(turn: &AITurn) -> OwnTurn {
        let state = turn.state();
        let unresolved = if state.is_closed() {
            state.outstanding_runtime_calls().to_vec()
        } else {
            Vec::new()
        };
        OwnTurn {
            turn: turn.clone(),
            unresolved,
        }
    }
}
