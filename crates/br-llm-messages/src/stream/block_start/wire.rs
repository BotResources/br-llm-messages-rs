use super::BlockStart;
use crate::error::{Error, ErrorCode};
use crate::id::BlockId;
use crate::opaque_json::OpaqueJson;
use crate::value::{
    ExecutionStatus, ExternalId, ItemRef, ObservedAt, ProviderOperation, ToolName, opaque,
};

pub(super) fn check_arguments(value: &OpaqueJson, object_required: bool) -> Result<(), Error> {
    opaque::check(value).map_err(|error| error.at("arguments"))?;
    if object_required && !value.inner().is_object() {
        return Err(Error::new(ErrorCode::InvalidContent).at("arguments"));
    }
    Ok(())
}

tagged_enum! {
    pub(super) enum BlockStartWire, BlockStartRaw, remote = "BlockStartWire", [tag = "type", rename_all = "snake_case", deny_unknown_fields] {
    Text {
        item_ref: Option<ItemRef>,
        observed_at: Option<ObservedAt>,
    },
    Thinking {
        observed_at: Option<ObservedAt>,
    },
    RedactedThinking {
        observed_at: Option<ObservedAt>,
    },
    StructuredContent {
        observed_at: Option<ObservedAt>,
    },
    RuntimeToolCall {
        name: Option<ToolName>,
        external_id: Option<ExternalId>,
        item_ref: Option<ItemRef>,
        #[serde(default, deserialize_with = "opaque::optional_arguments")]
        arguments: Option<OpaqueJson>,
        observed_at: Option<ObservedAt>,
    },
    ProviderToolCall {
        operation: Option<ProviderOperation>,
        external_id: Option<ExternalId>,
        item_ref: Option<ItemRef>,
        #[serde(default, deserialize_with = "opaque::optional_arguments")]
        arguments: Option<OpaqueJson>,
        observed_at: Option<ObservedAt>,
    },
    ProviderToolResult {
        call: BlockId,
        external_id: Option<ExternalId>,
        observed_at: Option<ObservedAt>,
    },
    ProviderToolExecutionRecord {
        operation: Option<ProviderOperation>,
        external_id: Option<ExternalId>,
        status: Option<ExecutionStatus>,
        observed_at: Option<ObservedAt>,
    },
    }
}

impl TryFrom<BlockStartWire> for BlockStart {
    type Error = Error;

    fn try_from(wire: BlockStartWire) -> Result<Self, Error> {
        Ok(match wire {
            BlockStartWire::Text {
                item_ref,
                observed_at,
            } => BlockStart::Text {
                item_ref,
                observed_at,
            },
            BlockStartWire::Thinking { observed_at } => BlockStart::Thinking { observed_at },
            BlockStartWire::RedactedThinking { observed_at } => {
                BlockStart::RedactedThinking { observed_at }
            }
            BlockStartWire::StructuredContent { observed_at } => {
                BlockStart::StructuredContent { observed_at }
            }
            BlockStartWire::RuntimeToolCall {
                name,
                external_id,
                item_ref,
                arguments,
                observed_at,
            } => {
                if let Some(value) = &arguments {
                    check_arguments(value, true)?;
                }
                BlockStart::RuntimeToolCall {
                    name,
                    external_id,
                    item_ref,
                    arguments,
                    observed_at,
                }
            }
            BlockStartWire::ProviderToolCall {
                operation,
                external_id,
                item_ref,
                arguments,
                observed_at,
            } => BlockStart::ProviderToolCall {
                operation,
                external_id,
                item_ref,
                arguments,
                observed_at,
            },
            BlockStartWire::ProviderToolResult {
                call,
                external_id,
                observed_at,
            } => BlockStart::ProviderToolResult {
                call,
                external_id,
                observed_at,
            },
            BlockStartWire::ProviderToolExecutionRecord {
                operation,
                external_id,
                status,
                observed_at,
            } => BlockStart::ProviderToolExecutionRecord {
                operation,
                external_id,
                status,
                observed_at,
            },
        })
    }
}
