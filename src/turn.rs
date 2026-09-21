use crate::block::ToolResult;
use crate::error::{MessageError, TurnStateLabel};
use crate::step::Step;
use crate::stop_reason::StopReason;
use crate::tool_results::ToolResults;
use crate::value::{Author, ToolCallId, TurnId};

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TurnItem {
    Step(Step),
    ToolResults(ToolResults),
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TurnState {
    AwaitingToolResults { pending: Vec<ToolCallId> },
    AwaitingStep,
    Finished { stop_reason: StopReason },
}

impl TurnState {
    pub fn label(&self) -> TurnStateLabel {
        match self {
            TurnState::AwaitingToolResults { .. } => TurnStateLabel::AwaitingToolResults,
            TurnState::AwaitingStep => TurnStateLabel::AwaitingStep,
            TurnState::Finished { .. } => TurnStateLabel::Finished,
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(try_from = "RawTurn")]
pub struct Turn {
    id: TurnId,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    author: Option<Author>,
    items: Vec<TurnItem>,
}

#[derive(serde::Deserialize)]
struct RawTurn {
    id: TurnId,
    #[serde(default)]
    author: Option<Author>,
    items: Vec<TurnItem>,
}

impl TryFrom<RawTurn> for Turn {
    type Error = MessageError;

    fn try_from(raw: RawTurn) -> Result<Self, Self::Error> {
        let mut items = raw.items.into_iter();
        let Some(TurnItem::Step(first)) = items.next() else {
            return Err(MessageError::TurnMustStartWithStep);
        };
        let mut turn = Turn::new(raw.id, raw.author, first);
        for item in items {
            match item {
                TurnItem::Step(step) => turn.push_step(step)?,
                TurnItem::ToolResults(results) => {
                    for result in results.results() {
                        turn.push_result(result.clone())?;
                    }
                }
            }
        }
        Ok(turn)
    }
}

impl Turn {
    pub fn new(id: TurnId, author: Option<Author>, first_step: Step) -> Self {
        Self {
            id,
            author,
            items: vec![TurnItem::Step(first_step)],
        }
    }

    pub fn id(&self) -> &TurnId {
        &self.id
    }

    pub fn author(&self) -> Option<&Author> {
        self.author.as_ref()
    }

    pub fn items(&self) -> &[TurnItem] {
        &self.items
    }

    fn last_step(&self) -> &Step {
        for item in self.items.iter().rev() {
            if let TurnItem::Step(step) = item {
                return step;
            }
        }
        unreachable!("a turn always holds at least its first step")
    }

    fn trailing_results(&self) -> Option<&ToolResults> {
        match self.items.last() {
            Some(TurnItem::ToolResults(results)) => Some(results),
            _ => None,
        }
    }

    pub fn state(&self) -> TurnState {
        let step = self.last_step();
        if !matches!(step.stop_reason(), StopReason::AwaitingToolResults) {
            return TurnState::Finished {
                stop_reason: step.stop_reason().clone(),
            };
        }
        let collected = self.trailing_results();
        let pending: Vec<ToolCallId> = step
            .tool_calls()
            .map(|call| call.id.clone())
            .filter(|id| collected.map(|c| !c.contains(id)).unwrap_or(true))
            .collect();
        if pending.is_empty() {
            TurnState::AwaitingStep
        } else {
            TurnState::AwaitingToolResults { pending }
        }
    }

    pub fn push_result(&mut self, result: ToolResult) -> Result<(), MessageError> {
        let state = self.state();
        let TurnState::AwaitingToolResults { pending } = state else {
            return Err(MessageError::TurnNotAwaitingResults {
                state: state.label(),
            });
        };
        let id = &result.tool_call_id;
        if pending.contains(id) {
            match self.items.last_mut() {
                Some(TurnItem::ToolResults(results)) => results.push(result),
                _ => {
                    let mut results = ToolResults::empty();
                    results.push(result)?;
                    self.items.push(TurnItem::ToolResults(results));
                    Ok(())
                }
            }
        } else if self.last_step().tool_calls().any(|call| &call.id == id) {
            Err(MessageError::DuplicateToolResultId {
                id: result.tool_call_id,
            })
        } else {
            Err(MessageError::ToolResultNotPending {
                given: result.tool_call_id,
                pending,
            })
        }
    }

    pub fn push_results(
        &mut self,
        results: impl IntoIterator<Item = ToolResult>,
    ) -> Result<(), MessageError> {
        for result in results {
            self.push_result(result)?;
        }
        Ok(())
    }

    pub fn push_step(&mut self, step: Step) -> Result<(), MessageError> {
        match self.state() {
            TurnState::AwaitingStep => {
                self.items.push(TurnItem::Step(step));
                Ok(())
            }
            other => Err(MessageError::TurnNotAwaitingStep {
                state: other.label(),
            }),
        }
    }
}

impl std::fmt::Display for Turn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.author {
            Some(author) => write!(f, "turn {} by {author}", self.id)?,
            None => write!(f, "turn {}", self.id)?,
        }
        write!(f, " ({} items, {})", self.items.len(), self.state().label())
    }
}

#[cfg(test)]
#[path = "turn_tests.rs"]
mod tests;
