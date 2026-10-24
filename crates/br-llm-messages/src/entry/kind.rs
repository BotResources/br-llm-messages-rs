use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::error::{Error, ErrorCode};

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TurnEntryKind {
    HumanMessage,
    AgentRequest,
    AgentMail,
    RequestAccepted,
    QueuePosition,
    TaskStarted,
    TaskCompleted,
    TaskFailed,
    ModelResponse,
    RuntimeToolResults,
    TurnClosure,
}

impl TurnEntryKind {
    pub const ALL: &'static [TurnEntryKind] = &[
        TurnEntryKind::HumanMessage,
        TurnEntryKind::AgentRequest,
        TurnEntryKind::AgentMail,
        TurnEntryKind::RequestAccepted,
        TurnEntryKind::QueuePosition,
        TurnEntryKind::TaskStarted,
        TurnEntryKind::TaskCompleted,
        TurnEntryKind::TaskFailed,
        TurnEntryKind::ModelResponse,
        TurnEntryKind::RuntimeToolResults,
        TurnEntryKind::TurnClosure,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            TurnEntryKind::HumanMessage => "human_message",
            TurnEntryKind::AgentRequest => "agent_request",
            TurnEntryKind::AgentMail => "agent_mail",
            TurnEntryKind::RequestAccepted => "request_accepted",
            TurnEntryKind::QueuePosition => "queue_position",
            TurnEntryKind::TaskStarted => "task_started",
            TurnEntryKind::TaskCompleted => "task_completed",
            TurnEntryKind::TaskFailed => "task_failed",
            TurnEntryKind::ModelResponse => "model_response",
            TurnEntryKind::RuntimeToolResults => "runtime_tool_results",
            TurnEntryKind::TurnClosure => "turn_closure",
        }
    }

    pub fn is_incoming(self) -> bool {
        matches!(
            self,
            TurnEntryKind::HumanMessage
                | TurnEntryKind::AgentRequest
                | TurnEntryKind::AgentMail
                | TurnEntryKind::RequestAccepted
                | TurnEntryKind::QueuePosition
                | TurnEntryKind::TaskStarted
                | TurnEntryKind::TaskCompleted
                | TurnEntryKind::TaskFailed
        )
    }
}

impl FromStr for TurnEntryKind {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self, Error> {
        TurnEntryKind::ALL
            .iter()
            .copied()
            .find(|kind| kind.as_str() == value)
            .ok_or_else(|| Error::new(ErrorCode::VariantMismatch))
    }
}

impl fmt::Display for TurnEntryKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
