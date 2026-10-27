use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

use crate::error::{Error, ErrorCode};

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
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
    RuntimeNotice,
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
        TurnEntryKind::RuntimeNotice,
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
            TurnEntryKind::RuntimeNotice => "runtime_notice",
            TurnEntryKind::ModelResponse => "model_response",
            TurnEntryKind::RuntimeToolResults => "runtime_tool_results",
            TurnEntryKind::TurnClosure => "turn_closure",
        }
    }

    pub fn is_incoming(self) -> bool {
        match self {
            TurnEntryKind::HumanMessage
            | TurnEntryKind::AgentRequest
            | TurnEntryKind::AgentMail
            | TurnEntryKind::RequestAccepted
            | TurnEntryKind::QueuePosition
            | TurnEntryKind::TaskStarted
            | TurnEntryKind::TaskCompleted
            | TurnEntryKind::TaskFailed
            | TurnEntryKind::RuntimeNotice => true,
            TurnEntryKind::ModelResponse
            | TurnEntryKind::RuntimeToolResults
            | TurnEntryKind::TurnClosure => false,
        }
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

const PROJECTED: [&str; 2] = ["agent_output", "partial_agent_output"];

pub(crate) fn refuse_type(value: &Value) -> Result<(), Error> {
    let Some(name) = value.get("type").and_then(Value::as_str) else {
        return Ok(());
    };
    if !PROJECTED.contains(&name) {
        return Ok(());
    }
    let refused = Error::new(ErrorCode::DisallowedKind);
    Err(
        match value
            .get("id")
            .and_then(Value::as_str)
            .and_then(|id| uuid::Uuid::try_parse(id).ok())
        {
            Some(id) => refused.with_id(id),
            None => refused.at("type"),
        },
    )
}

impl<'de> Deserialize<'de> for TurnEntryKind {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let name = String::deserialize(deserializer)?;
        if PROJECTED.contains(&name.as_str()) {
            return Err(serde::de::Error::custom(Error::new(
                ErrorCode::DisallowedKind,
            )));
        }
        name.parse().map_err(serde::de::Error::custom)
    }
}

impl fmt::Display for TurnEntryKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
