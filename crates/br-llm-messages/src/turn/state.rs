use std::collections::{HashMap, HashSet};

use uuid::Uuid;

use super::declared::{Declared, Place, turn_place};
use crate::block::ContentBlock;
use crate::entry::{ClosureReason, TurnEntry, TurnEntryKind};
use crate::id::{BlockId, TurnId};
use crate::value::{ExecutionStatus, ResponseEnding, ToolName};

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub struct TurnState {
    closure: Option<ClosureReason>,
    outstanding_runtime_calls: Vec<BlockId>,
    pending_provider_executions: Vec<BlockId>,
    last_ending: Option<ResponseEnding>,
}

impl TurnState {
    pub fn closure(&self) -> Option<&ClosureReason> {
        self.closure.as_ref()
    }

    pub fn is_closed(&self) -> bool {
        self.closure.is_some()
    }

    pub fn outstanding_runtime_calls(&self) -> &[BlockId] {
        &self.outstanding_runtime_calls
    }

    pub fn pending_provider_executions(&self) -> &[BlockId] {
        &self.pending_provider_executions
    }

    pub fn last_ending(&self) -> Option<&ResponseEnding> {
        self.last_ending.as_ref()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Tracker {
    pub(crate) ids: HashMap<Uuid, Place>,
    pub(crate) calls: HashMap<BlockId, ToolName>,
    pub(crate) outstanding: Vec<BlockId>,
    pub(crate) answered: HashSet<BlockId>,
    pub(crate) answered_provider: HashSet<BlockId>,
    pub(crate) closure: Option<ClosureReason>,
    pub(crate) last_ending: Option<ResponseEnding>,
    pub(crate) pending: Vec<BlockId>,
    pub(crate) last_entry: Option<TurnEntryKind>,
}

impl Tracker {
    pub(crate) fn new(turn: TurnId) -> Self {
        Tracker {
            ids: HashMap::from([(turn.as_uuid(), turn_place())]),
            calls: HashMap::new(),
            outstanding: Vec::new(),
            answered: HashSet::new(),
            answered_provider: HashSet::new(),
            closure: None,
            last_ending: None,
            pending: Vec::new(),
            last_entry: None,
        }
    }

    pub(crate) fn state(&self) -> TurnState {
        TurnState {
            closure: self.closure.clone(),
            outstanding_runtime_calls: self.outstanding.clone(),
            pending_provider_executions: self.pending.clone(),
            last_ending: self.last_ending.clone(),
        }
    }

    pub(crate) fn declared(&self) -> Declared {
        let mut all: Declared = self.ids.iter().map(|(id, place)| (*id, *place)).collect();
        all.sort_by_key(|(id, place)| (place.entry, place.root, place.child, *id));
        all
    }

    pub(crate) fn apply(&mut self, entry: &TurnEntry, declared: Declared) {
        self.ids.extend(declared);
        match entry {
            TurnEntry::ModelResponse(response) => {
                self.pending.clear();
                for block in response.blocks() {
                    match block {
                        ContentBlock::RuntimeToolCall(call) => {
                            self.calls.insert(call.id(), call.name().clone());
                            self.outstanding.push(call.id());
                        }
                        ContentBlock::ProviderToolResult(result) => {
                            self.answered_provider.insert(result.call());
                        }
                        ContentBlock::ProviderToolExecutionRecord(record)
                            if matches!(
                                record.status(),
                                Some(ExecutionStatus::Pending | ExecutionStatus::Running)
                            ) =>
                        {
                            self.pending.push(record.id());
                        }
                        _ => {}
                    }
                }
                self.last_ending = Some(response.ending().clone());
            }
            TurnEntry::RuntimeToolResults(results) => {
                for result in results.results() {
                    self.outstanding.retain(|call| *call != result.call());
                    self.answered.insert(result.call());
                }
            }
            TurnEntry::TurnClosure(closure) => {
                self.closure = Some(closure.reason().clone());
            }
            TurnEntry::HumanMessage(_)
            | TurnEntry::AgentRequest(_)
            | TurnEntry::AgentMail(_)
            | TurnEntry::RequestAccepted(_)
            | TurnEntry::QueuePosition(_)
            | TurnEntry::TaskStarted(_)
            | TurnEntry::TaskCompleted(_)
            | TurnEntry::TaskFailed(_)
            | TurnEntry::RuntimeNotice(_) => {}
        }
        self.last_entry = Some(entry.kind());
    }
}
