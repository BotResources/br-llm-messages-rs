use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Map, Value};

use crate::block::ContentBlockKind;
use crate::entry::{ClosureReason, RequestOrigin, TurnEntryKind};
use crate::error::{Error, ErrorCode};
use crate::id::{ConversationId, EntryId, RequestId, TaskId};
use crate::stream::InterruptionReason;
use crate::value::{
    Citation, ExecutionStatus, ExternalId, ImageSource, ItemRef, ModelName, PlainText,
    ProviderOperation, ReplayMaterial, ResponseEnding, ToolName, Usage,
};

pub(super) type Check = fn(&Value) -> Result<(), Error>;

pub(super) struct Field {
    pub(super) key: &'static str,
    pub(super) required: bool,
    pub(super) check: Check,
}

const fn required(key: &'static str, check: Check) -> Field {
    Field {
        key,
        required: true,
        check,
    }
}

const fn optional(key: &'static str, check: Check) -> Field {
    Field {
        key,
        required: false,
        check,
    }
}

fn typed<T: DeserializeOwned + Serialize>(value: &Value) -> Result<(), Error> {
    let decoded = crate::json::decode_ref::<T>(value)?;
    if crate::json::encode_value(&decoded)? == *value {
        Ok(())
    } else {
        Err(Error::new(ErrorCode::InvalidContent))
    }
}

fn present_opaque_value(value: &Value) -> Result<(), Error> {
    crate::value::opaque::check_present_value(value)
}

fn opaque_value(value: &Value) -> Result<(), Error> {
    crate::value::opaque::check_value(value)
}

fn arguments_object(value: &Value) -> Result<(), Error> {
    if !value.is_object() {
        return Err(Error::new(ErrorCode::InvalidContent));
    }
    opaque_value(value)
}

fn queue_position(value: &Value) -> Result<(), Error> {
    match value.as_u64() {
        Some(position) if position >= 1 && u32::try_from(position).is_ok() => Ok(()),
        _ => Err(Error::new(ErrorCode::InvalidContent)),
    }
}

pub(super) fn entry_fields(kind: TurnEntryKind) -> Vec<Field> {
    match kind {
        TurnEntryKind::AgentRequest => vec![
            required("source_conversation", typed::<ConversationId>),
            required("origin", typed::<RequestOrigin>),
        ],
        TurnEntryKind::AgentMail => vec![
            required("source_conversation", typed::<ConversationId>),
            optional("in_reply_to", typed::<EntryId>),
        ],
        TurnEntryKind::RequestAccepted => vec![required("request", typed::<RequestId>)],
        TurnEntryKind::QueuePosition => vec![
            required("request", typed::<RequestId>),
            required("position", queue_position),
        ],
        TurnEntryKind::TaskStarted => vec![
            required("request", typed::<RequestId>),
            required("task", typed::<TaskId>),
            optional("conversation", typed::<ConversationId>),
        ],
        TurnEntryKind::TaskCompleted => vec![
            required("task", typed::<TaskId>),
            optional("summary", typed::<PlainText>),
        ],
        TurnEntryKind::TaskFailed => vec![
            required("task", typed::<TaskId>),
            required("reason", typed::<PlainText>),
        ],
        TurnEntryKind::ModelResponse => vec![
            required("ending", typed::<ResponseEnding>),
            optional("usage", typed::<Usage>),
            optional("model", typed::<ModelName>),
            optional("provider_response_id", typed::<ExternalId>),
        ],
        TurnEntryKind::TurnClosure => vec![required("reason", typed::<ClosureReason>)],
        TurnEntryKind::HumanMessage | TurnEntryKind::RuntimeToolResults => Vec::new(),
    }
}

pub(super) fn header_fields(interrupted: bool) -> Vec<Field> {
    let mut fields = vec![
        optional("usage", typed::<Usage>),
        optional("model", typed::<ModelName>),
        optional("provider_response_id", typed::<ExternalId>),
    ];
    if interrupted {
        fields.push(required("interruption", typed::<InterruptionReason>));
    }
    fields
}

pub(super) fn block_fields(kind: ContentBlockKind) -> Vec<Field> {
    match kind {
        ContentBlockKind::Text => vec![
            required("text", typed::<PlainText>),
            optional("citations", typed::<Vec<Citation>>),
            optional("item_ref", typed::<ItemRef>),
            optional("replay", typed::<ReplayMaterial>),
        ],
        ContentBlockKind::Image => vec![required("source", typed::<ImageSource>)],
        ContentBlockKind::StructuredContent => vec![required("value", opaque_value)],
        ContentBlockKind::Thinking => vec![
            required("text", typed::<String>),
            optional("replay", typed::<ReplayMaterial>),
        ],
        ContentBlockKind::RedactedThinking => vec![required("material", typed::<ReplayMaterial>)],
        ContentBlockKind::RuntimeToolCall => vec![
            optional("external_id", typed::<ExternalId>),
            optional("item_ref", typed::<ItemRef>),
            required("name", typed::<ToolName>),
            required("arguments", arguments_object),
            optional("replay", typed::<ReplayMaterial>),
        ],
        ContentBlockKind::RuntimeToolResult => vec![
            required("tool_name", typed::<ToolName>),
            required("is_error", typed::<bool>),
        ],
        ContentBlockKind::ProviderToolCall => vec![
            optional("external_id", typed::<ExternalId>),
            optional("item_ref", typed::<ItemRef>),
            optional("operation", typed::<ProviderOperation>),
            optional("arguments", present_opaque_value),
            optional("replay", typed::<ReplayMaterial>),
        ],
        ContentBlockKind::ProviderToolResult => vec![
            optional("external_id", typed::<ExternalId>),
            optional("replay", typed::<ReplayMaterial>),
        ],
        ContentBlockKind::ProviderToolExecutionRecord => vec![
            optional("external_id", typed::<ExternalId>),
            optional("operation", typed::<ProviderOperation>),
            optional("status", typed::<ExecutionStatus>),
            optional("replay", typed::<ReplayMaterial>),
        ],
    }
}

pub(super) fn check_object(object: &Map<String, Value>, fields: &[Field]) -> Result<(), Error> {
    for key in object.keys() {
        if !fields.iter().any(|field| field.key == key) {
            return Err(Error::new(ErrorCode::InvalidContent));
        }
    }
    for field in fields {
        match object.get(field.key) {
            None if field.required => {
                return Err(Error::new(ErrorCode::InvalidContent).at(field.key));
            }
            None => {}
            Some(value) => (field.check)(value).map_err(|error| error.at(field.key))?,
        }
    }
    Ok(())
}

pub(super) fn as_object(payload: &Value) -> Result<&Map<String, Value>, Error> {
    payload
        .as_object()
        .ok_or_else(|| Error::new(ErrorCode::InvalidContent).at("payload"))
}
