use super::BlockStart;
use crate::error::{Error, ErrorCode};
use crate::id::BlockId;
use crate::opaque_json::OpaqueJson;
use crate::value::{ExecutionStatus, ExternalId, ItemRef, ObservedAt, ProviderOperation, ToolName};

impl BlockStart {
    pub fn text() -> Self {
        BlockStart::Text {
            item_ref: None,
            observed_at: None,
        }
    }

    pub fn thinking() -> Self {
        BlockStart::Thinking { observed_at: None }
    }

    pub fn redacted_thinking() -> Self {
        BlockStart::RedactedThinking { observed_at: None }
    }

    pub fn structured_content() -> Self {
        BlockStart::StructuredContent { observed_at: None }
    }

    pub fn runtime_tool_call(name: Option<ToolName>, external_id: Option<ExternalId>) -> Self {
        BlockStart::RuntimeToolCall {
            name,
            external_id,
            item_ref: None,
            arguments: None,
            observed_at: None,
        }
    }

    pub fn provider_tool_call(
        operation: Option<ProviderOperation>,
        external_id: Option<ExternalId>,
    ) -> Self {
        BlockStart::ProviderToolCall {
            operation,
            external_id,
            item_ref: None,
            arguments: None,
            observed_at: None,
        }
    }

    pub fn provider_tool_result(call: BlockId, external_id: Option<ExternalId>) -> Self {
        BlockStart::ProviderToolResult {
            call,
            external_id,
            observed_at: None,
        }
    }

    pub fn provider_tool_execution_record(
        operation: Option<ProviderOperation>,
        external_id: Option<ExternalId>,
        status: Option<ExecutionStatus>,
    ) -> Self {
        BlockStart::ProviderToolExecutionRecord {
            operation,
            external_id,
            status,
            observed_at: None,
        }
    }

    pub fn with_observed_at(mut self, value: ObservedAt) -> Self {
        match &mut self {
            BlockStart::Text { observed_at, .. }
            | BlockStart::Thinking { observed_at }
            | BlockStart::RedactedThinking { observed_at }
            | BlockStart::StructuredContent { observed_at }
            | BlockStart::RuntimeToolCall { observed_at, .. }
            | BlockStart::ProviderToolCall { observed_at, .. }
            | BlockStart::ProviderToolResult { observed_at, .. }
            | BlockStart::ProviderToolExecutionRecord { observed_at, .. } => {
                *observed_at = Some(value);
            }
        }
        self
    }

    pub fn with_item_ref(mut self, value: ItemRef) -> Result<Self, Error> {
        match &mut self {
            BlockStart::Text { item_ref, .. }
            | BlockStart::RuntimeToolCall { item_ref, .. }
            | BlockStart::ProviderToolCall { item_ref, .. } => *item_ref = Some(value),
            _ => return Err(Error::new(ErrorCode::DisallowedKind).at("item_ref")),
        }
        Ok(self)
    }

    pub fn with_arguments(mut self, value: OpaqueJson) -> Result<Self, Error> {
        match &mut self {
            BlockStart::RuntimeToolCall { arguments, .. } => {
                super::wire::check_arguments(&value, true)?;
                *arguments = Some(value);
            }
            BlockStart::ProviderToolCall { arguments, .. } => {
                super::wire::check_arguments(&value, false)?;
                *arguments = Some(value);
            }
            _ => return Err(Error::new(ErrorCode::DisallowedKind).at("arguments")),
        }
        Ok(self)
    }
}
