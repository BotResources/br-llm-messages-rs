use serde::{Deserialize, Serialize};

use super::{locate, refuse_author};
use crate::error::Error;
use crate::id::{ConversationId, EntryId, RequestId, TaskId};
use crate::value::{ObservedAt, PlainText};

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "TaskStartedWire")]
pub struct TaskStarted {
    id: EntryId,
    request: RequestId,
    task: TaskId,
    #[serde(skip_serializing_if = "Option::is_none")]
    conversation: Option<ConversationId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    observed_at: Option<ObservedAt>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TaskStartedWire {
    id: EntryId,
    request: RequestId,
    task: TaskId,
    #[serde(default, deserialize_with = "crate::json::optional")]
    conversation: Option<ConversationId>,
    #[serde(default, deserialize_with = "super::present")]
    author: bool,
    #[serde(default, deserialize_with = "crate::json::optional")]
    observed_at: Option<ObservedAt>,
}

impl TaskStarted {
    pub fn new(id: EntryId, request: RequestId, task: TaskId) -> Self {
        TaskStarted {
            id,
            request,
            task,
            conversation: None,
            observed_at: None,
        }
    }

    pub fn with_observed_at(mut self, observed_at: ObservedAt) -> Self {
        self.observed_at = Some(observed_at);
        self
    }

    pub fn with_conversation(mut self, conversation: ConversationId) -> Self {
        self.conversation = Some(conversation);
        self
    }

    pub fn id(&self) -> EntryId {
        self.id
    }

    pub fn request(&self) -> RequestId {
        self.request
    }

    pub fn task(&self) -> TaskId {
        self.task
    }

    pub fn conversation(&self) -> Option<ConversationId> {
        self.conversation
    }

    pub fn observed_at(&self) -> Option<&ObservedAt> {
        self.observed_at.as_ref()
    }
}

impl TryFrom<TaskStartedWire> for TaskStarted {
    type Error = Error;

    fn try_from(wire: TaskStartedWire) -> Result<Self, Error> {
        refuse_author(wire.author, wire.id)?;
        let started = TaskStarted::new(wire.id, wire.request, wire.task);
        let started = match wire.conversation {
            Some(conversation) => started.with_conversation(conversation),
            None => started,
        };
        Ok(match wire.observed_at {
            Some(observed_at) => started.with_observed_at(observed_at),
            None => started,
        })
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "TaskCompletedWire")]
pub struct TaskCompleted {
    id: EntryId,
    task: TaskId,
    #[serde(skip_serializing_if = "Option::is_none")]
    summary: Option<PlainText>,
    #[serde(skip_serializing_if = "Option::is_none")]
    observed_at: Option<ObservedAt>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TaskCompletedWire {
    id: EntryId,
    task: TaskId,
    #[serde(default, deserialize_with = "crate::json::optional")]
    summary: Option<String>,
    #[serde(default, deserialize_with = "super::present")]
    author: bool,
    #[serde(default, deserialize_with = "crate::json::optional")]
    observed_at: Option<ObservedAt>,
}

impl TaskCompleted {
    pub fn new(id: EntryId, task: TaskId) -> Self {
        TaskCompleted {
            id,
            task,
            summary: None,
            observed_at: None,
        }
    }

    pub fn with_observed_at(mut self, observed_at: ObservedAt) -> Self {
        self.observed_at = Some(observed_at);
        self
    }

    pub fn with_summary(mut self, summary: impl Into<String>) -> Result<Self, Error> {
        let summary =
            PlainText::new(summary).map_err(|error| locate(error.at("summary"), self.id))?;
        self.summary = Some(summary);
        Ok(self)
    }

    pub fn id(&self) -> EntryId {
        self.id
    }

    pub fn task(&self) -> TaskId {
        self.task
    }

    pub fn summary(&self) -> Option<&str> {
        self.summary.as_ref().map(PlainText::as_str)
    }

    pub fn observed_at(&self) -> Option<&ObservedAt> {
        self.observed_at.as_ref()
    }
}

impl TryFrom<TaskCompletedWire> for TaskCompleted {
    type Error = Error;

    fn try_from(wire: TaskCompletedWire) -> Result<Self, Error> {
        refuse_author(wire.author, wire.id)?;
        let completed = TaskCompleted::new(wire.id, wire.task);
        let completed = match wire.summary {
            Some(summary) => completed.with_summary(summary)?,
            None => completed,
        };
        Ok(match wire.observed_at {
            Some(observed_at) => completed.with_observed_at(observed_at),
            None => completed,
        })
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "TaskFailedWire")]
pub struct TaskFailed {
    id: EntryId,
    task: TaskId,
    reason: PlainText,
    #[serde(skip_serializing_if = "Option::is_none")]
    observed_at: Option<ObservedAt>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TaskFailedWire {
    id: EntryId,
    task: TaskId,
    reason: String,
    #[serde(default, deserialize_with = "super::present")]
    author: bool,
    #[serde(default, deserialize_with = "crate::json::optional")]
    observed_at: Option<ObservedAt>,
}

impl TaskFailed {
    pub fn new(id: EntryId, task: TaskId, reason: impl Into<String>) -> Result<Self, Error> {
        let reason = PlainText::new(reason).map_err(|error| locate(error.at("reason"), id))?;
        Ok(TaskFailed {
            id,
            task,
            reason,
            observed_at: None,
        })
    }

    pub fn with_observed_at(mut self, observed_at: ObservedAt) -> Self {
        self.observed_at = Some(observed_at);
        self
    }

    pub fn id(&self) -> EntryId {
        self.id
    }

    pub fn task(&self) -> TaskId {
        self.task
    }

    pub fn reason(&self) -> &str {
        self.reason.as_str()
    }

    pub fn observed_at(&self) -> Option<&ObservedAt> {
        self.observed_at.as_ref()
    }
}

impl TryFrom<TaskFailedWire> for TaskFailed {
    type Error = Error;

    fn try_from(wire: TaskFailedWire) -> Result<Self, Error> {
        refuse_author(wire.author, wire.id)?;
        let failed = TaskFailed::new(wire.id, wire.task, wire.reason)?;
        Ok(match wire.observed_at {
            Some(observed_at) => failed.with_observed_at(observed_at),
            None => failed,
        })
    }
}
