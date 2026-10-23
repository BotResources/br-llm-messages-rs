use serde::{Deserialize, Serialize};

use crate::id::{BlockId, EntryId, RequestId};

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum RequestOrigin {
    #[non_exhaustive]
    Request { id: RequestId },
    #[non_exhaustive]
    ToolCall { block: BlockId },
}

impl RequestOrigin {
    pub fn request(id: RequestId) -> Self {
        RequestOrigin::Request { id }
    }

    pub fn tool_call(block: BlockId) -> Self {
        RequestOrigin::ToolCall { block }
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum MailCorrelation {
    #[non_exhaustive]
    Request { id: RequestId },
    #[non_exhaustive]
    Message { entry: EntryId },
}

impl MailCorrelation {
    pub fn request(id: RequestId) -> Self {
        MailCorrelation::Request { id }
    }

    pub fn message(entry: EntryId) -> Self {
        MailCorrelation::Message { entry }
    }
}
