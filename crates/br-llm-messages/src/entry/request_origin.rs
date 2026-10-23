use serde::{Deserialize, Serialize};

use crate::id::{BlockId, RequestId};

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
