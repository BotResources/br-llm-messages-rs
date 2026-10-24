use crate::id::{BlockId, RequestId};

tagged_enum! {
    #[non_exhaustive]
    #[derive(Debug, Clone, PartialEq)]
    pub enum RequestOrigin, RequestOriginWire, remote = "RequestOrigin", [tag = "type", rename_all = "snake_case", deny_unknown_fields] {
        #[non_exhaustive]
        Request { id: RequestId },
        #[non_exhaustive]
        ToolCall { block: BlockId },
    }
}

impl RequestOrigin {
    pub fn request(id: RequestId) -> Self {
        RequestOrigin::Request { id }
    }

    pub fn tool_call(block: BlockId) -> Self {
        RequestOrigin::ToolCall { block }
    }
}
