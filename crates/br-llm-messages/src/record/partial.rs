use crate::opaque_json::OpaqueJson;
use serde::{Deserialize, Serialize};

use crate::block::ContentBlockKind;
use crate::error::{Error, ErrorCode};
use crate::value::{
    Citation, ExecutionStatus, ExternalId, ItemRef, ProviderOperation, ReplayMaterial, ToolName,
    opaque,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TextPartial {
    pub(crate) text: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) citations: Vec<Citation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) item_ref: Option<ItemRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) replay: Option<ReplayMaterial>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ThinkingPartial {
    pub(crate) text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) replay: Option<ReplayMaterial>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RedactedPartial {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) replay: Option<ReplayMaterial>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct StructuredPartial {
    pub(crate) raw_json: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RuntimeCallPartial {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) name: Option<ToolName>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) external_id: Option<ExternalId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) item_ref: Option<ItemRef>,
    #[serde(
        default,
        deserialize_with = "opaque::optional_provisional_arguments",
        skip_serializing_if = "Option::is_none"
    )]
    pub(crate) provisional_arguments: Option<OpaqueJson>,
    pub(crate) raw_arguments: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) replay: Option<ReplayMaterial>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProviderCallPartial {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) operation: Option<ProviderOperation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) external_id: Option<ExternalId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) item_ref: Option<ItemRef>,
    #[serde(
        default,
        deserialize_with = "opaque::optional_provisional_arguments",
        skip_serializing_if = "Option::is_none"
    )]
    pub(crate) provisional_arguments: Option<OpaqueJson>,
    pub(crate) raw_arguments: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) replay: Option<ReplayMaterial>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProviderResultPartial {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) external_id: Option<ExternalId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) replay: Option<ReplayMaterial>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RecordPartial {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) operation: Option<ProviderOperation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) external_id: Option<ExternalId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) status: Option<ExecutionStatus>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) replay: Option<ReplayMaterial>,
}

pub(crate) fn decode<T: serde::de::DeserializeOwned>(payload: &OpaqueJson) -> Result<T, Error> {
    payload.deserialize_into()
}

pub(crate) fn encode<T: Serialize>(partial: &T) -> Result<OpaqueJson, Error> {
    OpaqueJson::from_serialize(partial)
}

fn runtime_call(payload: &OpaqueJson) -> Result<(), Error> {
    let partial: RuntimeCallPartial = decode(payload)?;
    match partial.provisional_arguments {
        Some(value) if !value.inner().is_object() => {
            Err(Error::new(ErrorCode::InvalidContent).at("provisional_arguments"))
        }
        _ => Ok(()),
    }
}

pub(crate) fn check(kind: ContentBlockKind, payload: &OpaqueJson) -> Result<(), Error> {
    if !payload.inner().is_object() {
        return Err(Error::new(ErrorCode::InvalidContent).at("payload"));
    }
    match kind {
        ContentBlockKind::Text => decode::<TextPartial>(payload).map(drop),
        ContentBlockKind::Thinking => decode::<ThinkingPartial>(payload).map(drop),
        ContentBlockKind::RedactedThinking => decode::<RedactedPartial>(payload).map(drop),
        ContentBlockKind::StructuredContent => decode::<StructuredPartial>(payload).map(drop),
        ContentBlockKind::RuntimeToolCall => runtime_call(payload),
        ContentBlockKind::ProviderToolCall => decode::<ProviderCallPartial>(payload).map(drop),
        ContentBlockKind::ProviderToolResult => decode::<ProviderResultPartial>(payload).map(drop),
        ContentBlockKind::ProviderToolExecutionRecord => decode::<RecordPartial>(payload).map(drop),
        _ => Err(Error::new(ErrorCode::DisallowedKind).at("kind")),
    }
    .map_err(|error| error.at("payload"))
}
