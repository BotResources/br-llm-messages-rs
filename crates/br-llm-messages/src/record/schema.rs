use serde::de::DeserializeOwned;
use serde_json::{Map, Value};

use crate::block::ContentBlockKind;
use crate::entry::{ClosureReason, MailCorrelation, RequestOrigin, ToolDefinition, TurnEntryKind};
use crate::error::{Error, ErrorCode};
use crate::id::{ConversationId, EntryId, RequestId, TaskId};
use crate::stream::InterruptionReason;
use crate::value::{
    Citation, ExecutionStatus, ExternalId, FailureDetail, ImageSource, ModelName, PlainText,
    ProviderOperation, ReplayMaterial, ResponseEnding, ResultStatus, ToolName, Usage,
};

pub(super) type Check = fn(&Value) -> Result<(), Error>;

pub(super) struct Field {
    pub(super) key: &'static str,
    pub(super) required: bool,
    pub(super) nullable: bool,
    pub(super) check: Check,
}

const fn required(key: &'static str, check: Check) -> Field {
    Field {
        key,
        required: true,
        nullable: false,
        check,
    }
}

const fn optional(key: &'static str, check: Check) -> Field {
    Field {
        key,
        required: false,
        nullable: true,
        check,
    }
}

const fn non_null(key: &'static str, check: Check) -> Field {
    Field {
        key,
        required: false,
        nullable: false,
        check,
    }
}

fn typed<T: DeserializeOwned>(value: &Value) -> Result<(), Error> {
    crate::json::decode_ref::<T>(value).map(|_| ())
}

fn any(_: &Value) -> Result<(), Error> {
    Ok(())
}

fn queue_position(value: &Value) -> Result<(), Error> {
    match value.as_u64() {
        Some(position) if position >= 1 && u32::try_from(position).is_ok() => Ok(()),
        _ => Err(Error::new(ErrorCode::InvalidContent)),
    }
}

fn exposure(value: &Value) -> Result<(), Error> {
    let object = value
        .as_object()
        .ok_or_else(|| Error::new(ErrorCode::InvalidRecord))?;
    if object.contains_key("content") {
        return Err(Error::new(ErrorCode::InvalidRecord).at("content"));
    }
    let kind = object
        .get("type")
        .and_then(Value::as_str)
        .ok_or_else(|| Error::new(ErrorCode::MissingContent).at("type"))?;
    let (material_required, material_allowed) = match kind {
        "readable" | "not_exposed" => (false, false),
        "opaque" | "mixed" => (true, true),
        _ => return Err(Error::new(ErrorCode::UnsupportedVariant).at("type")),
    };
    for key in object.keys() {
        if key != "type" && key != "material" {
            return Err(Error::new(ErrorCode::UnsupportedField).at(key.as_str()));
        }
    }
    match (object.get("material"), material_required, material_allowed) {
        (None, true, _) => Err(Error::new(ErrorCode::MissingContent).at("material")),
        (Some(_), _, false) => Err(Error::new(ErrorCode::InvalidRecord).at("material")),
        (Some(material), _, true) => {
            typed::<ReplayMaterial>(material).map_err(|e| e.at("material"))
        }
        (None, false, _) => Ok(()),
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
            optional("in_reply_to", typed::<MailCorrelation>),
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
            required("failure", typed::<FailureDetail>),
        ],
        TurnEntryKind::ModelResponse => vec![
            required("ending", typed::<ResponseEnding>),
            optional("usage", typed::<Usage>),
            optional("model", typed::<ModelName>),
            optional("provider_response_id", typed::<ExternalId>),
        ],
        TurnEntryKind::ToolMounted => vec![required("definition", typed::<ToolDefinition>)],
        TurnEntryKind::ToolUnmounted => vec![required("mounting", typed::<EntryId>)],
        TurnEntryKind::TurnClosure => vec![required("reason", typed::<ClosureReason>)],
        _ => Vec::new(),
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
            non_null("citations", typed::<Vec<Citation>>),
            optional("replay", typed::<ReplayMaterial>),
        ],
        ContentBlockKind::Image => vec![required("source", typed::<ImageSource>)],
        ContentBlockKind::StructuredContent => vec![required("value", any)],
        ContentBlockKind::Thinking => vec![
            required("text", typed::<String>),
            optional("replay", typed::<ReplayMaterial>),
        ],
        ContentBlockKind::RedactedThinking => vec![required("material", typed::<ReplayMaterial>)],
        ContentBlockKind::RuntimeToolCall => vec![
            optional("external_id", typed::<ExternalId>),
            required("name", typed::<ToolName>),
            required("arguments", any),
            optional("replay", typed::<ReplayMaterial>),
        ],
        ContentBlockKind::RuntimeToolResult => vec![
            required("tool_name", typed::<ToolName>),
            required("status", typed::<ResultStatus>),
        ],
        ContentBlockKind::ProviderToolCall => vec![
            optional("external_id", typed::<ExternalId>),
            optional("operation", typed::<ProviderOperation>),
            optional("arguments", any),
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
            required("exposure", exposure),
        ],
    }
}

pub(super) fn check_object(object: &Map<String, Value>, fields: &[Field]) -> Result<(), Error> {
    for key in object.keys() {
        if !fields.iter().any(|field| field.key == key) {
            return Err(Error::new(ErrorCode::UnsupportedField).at(key.as_str()));
        }
    }
    for field in fields {
        match object.get(field.key) {
            None if field.required => {
                return Err(Error::new(ErrorCode::MissingContent).at(field.key));
            }
            None => {}
            Some(Value::Null) if field.nullable => {}
            Some(value) => (field.check)(value).map_err(|error| error.at(field.key))?,
        }
    }
    Ok(())
}

pub(super) fn as_object(payload: &Value) -> Result<&Map<String, Value>, Error> {
    payload
        .as_object()
        .ok_or_else(|| Error::new(ErrorCode::InvalidRecord).at("payload"))
}
