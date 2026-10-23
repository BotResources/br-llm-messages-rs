use serde::{Deserialize, Serialize};

use super::{locate, refuse_author};
use crate::block::IdSink;
use crate::error::Error;
use crate::id::{ConversationId, EntryId, RequestId, TaskId};
use crate::value::{FailureDetail, PlainText};

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "TaskStartedWire")]
pub struct TaskStarted {
    id: EntryId,
    request: RequestId,
    task: TaskId,
    #[serde(skip_serializing_if = "Option::is_none")]
    conversation: Option<ConversationId>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TaskStartedWire {
    id: EntryId,
    request: RequestId,
    task: TaskId,
    conversation: Option<ConversationId>,
    #[serde(default, deserialize_with = "super::present")]
    author: bool,
}

impl TaskStarted {
    pub fn new(id: EntryId, request: RequestId, task: TaskId) -> Self {
        TaskStarted {
            id,
            request,
            task,
            conversation: None,
        }
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

    pub(crate) fn collect_ids(&self, sink: &mut IdSink) -> Result<(), Error> {
        sink.insert(self.id.as_uuid())
    }
}

impl TryFrom<TaskStartedWire> for TaskStarted {
    type Error = Error;

    fn try_from(wire: TaskStartedWire) -> Result<Self, Error> {
        refuse_author(wire.author, wire.id)?;
        let started = TaskStarted::new(wire.id, wire.request, wire.task);
        Ok(match wire.conversation {
            Some(conversation) => started.with_conversation(conversation),
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
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TaskCompletedWire {
    id: EntryId,
    task: TaskId,
    summary: Option<String>,
    #[serde(default, deserialize_with = "super::present")]
    author: bool,
}

impl TaskCompleted {
    pub fn new(id: EntryId, task: TaskId) -> Self {
        TaskCompleted {
            id,
            task,
            summary: None,
        }
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

    pub(crate) fn collect_ids(&self, sink: &mut IdSink) -> Result<(), Error> {
        sink.insert(self.id.as_uuid())
    }
}

impl TryFrom<TaskCompletedWire> for TaskCompleted {
    type Error = Error;

    fn try_from(wire: TaskCompletedWire) -> Result<Self, Error> {
        refuse_author(wire.author, wire.id)?;
        let completed = TaskCompleted::new(wire.id, wire.task);
        match wire.summary {
            Some(summary) => completed.with_summary(summary),
            None => Ok(completed),
        }
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "TaskFailedWire")]
pub struct TaskFailed {
    id: EntryId,
    task: TaskId,
    failure: FailureDetail,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TaskFailedWire {
    id: EntryId,
    task: TaskId,
    failure: FailureDetail,
    #[serde(default, deserialize_with = "super::present")]
    author: bool,
}

impl TaskFailed {
    pub fn new(id: EntryId, task: TaskId, failure: FailureDetail) -> Self {
        TaskFailed { id, task, failure }
    }

    pub fn id(&self) -> EntryId {
        self.id
    }

    pub fn task(&self) -> TaskId {
        self.task
    }

    pub fn failure(&self) -> &FailureDetail {
        &self.failure
    }

    pub(crate) fn collect_ids(&self, sink: &mut IdSink) -> Result<(), Error> {
        sink.insert(self.id.as_uuid())
    }
}

impl TryFrom<TaskFailedWire> for TaskFailed {
    type Error = Error;

    fn try_from(wire: TaskFailedWire) -> Result<Self, Error> {
        refuse_author(wire.author, wire.id)?;
        Ok(TaskFailed::new(wire.id, wire.task, wire.failure))
    }
}
