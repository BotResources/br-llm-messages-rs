use crate::block::{ToolCall, ToolResult};
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

/// Computed from the turn's last step, never stored: it serializes for a
/// reader but is not loaded from JSON.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
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
                    if results.is_empty() {
                        return Err(MessageError::EmptyToolResults);
                    }
                    turn.push_results(results.into_results())?;
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
            Some(TurnItem::Step(_)) | None => None,
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

    /// Accepts a result while the turn awaits results, for a pending call
    /// id, once, and only when its tool name is the name of that call.
    pub fn push_result(&mut self, result: ToolResult) -> Result<(), MessageError> {
        let state = self.state();
        let TurnState::AwaitingToolResults { pending } = state else {
            return Err(MessageError::TurnNotAwaitingResults {
                state: state.label(),
            });
        };
        let id = &result.tool_call_id;
        let Some(call) = self.last_step().tool_calls().find(|call| &call.id == id) else {
            return Err(MessageError::ToolResultNotPending {
                given: result.tool_call_id,
                pending,
            });
        };
        if !pending.contains(id) {
            return Err(MessageError::DuplicateToolResultId {
                id: result.tool_call_id,
            });
        }
        if call.name != result.tool_name {
            return Err(MessageError::ToolNameMismatch {
                id: result.tool_call_id,
                call: call.name.clone(),
                result: result.tool_name,
            });
        }
        match self.items.last_mut() {
            Some(TurnItem::ToolResults(results)) => results.push(result),
            Some(TurnItem::Step(_)) | None => {
                let mut results = ToolResults::empty();
                results.push(result)?;
                self.items.push(TurnItem::ToolResults(results));
                Ok(())
            }
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

    /// Accepts a step while the turn awaits a step, when none of its tool
    /// call ids is already used by a step of the turn.
    pub fn push_step(&mut self, step: Step) -> Result<(), MessageError> {
        self.check_step(&step)?;
        self.items.push(TurnItem::Step(step));
        Ok(())
    }

    /// The turn-level rules a new step must meet, checked without changing
    /// the turn so that the conversation can add its own rule before the step
    /// is appended.
    pub(crate) fn check_step(&self, step: &Step) -> Result<(), MessageError> {
        let state = self.state();
        if state != TurnState::AwaitingStep {
            return Err(MessageError::TurnNotAwaitingStep {
                state: state.label(),
            });
        }
        if let Some(call) = step
            .tool_calls()
            .find(|call| self.tool_calls().any(|used| used.id == call.id))
        {
            return Err(MessageError::DuplicateToolCallIdInTurn {
                id: call.id.clone(),
            });
        }
        Ok(())
    }

    /// Every tool call of every step of the turn, in order.
    pub(crate) fn tool_calls(&self) -> impl Iterator<Item = &ToolCall> {
        self.items
            .iter()
            .filter_map(|item| match item {
                TurnItem::Step(step) => Some(step),
                TurnItem::ToolResults(_) => None,
            })
            .flat_map(Step::tool_calls)
    }
}

#[cfg(test)]
#[path = "turn_test_helpers.rs"]
mod test_helpers;

#[cfg(test)]
#[path = "turn_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "turn_rules_tests.rs"]
mod rules_tests;
