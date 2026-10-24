use crate::opaque_json::OpaqueJson;
use serde::{Deserialize, Serialize};

use crate::entry::ResponseMetadata;
use crate::error::Error;
use crate::stream::draft::DraftPhase;
use crate::stream::ops::InterruptionReason;
use crate::value::{ExternalId, ModelName, Usage};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Header {
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::json::optional"
    )]
    usage: Option<Usage>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::json::optional"
    )]
    model: Option<ModelName>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::json::optional"
    )]
    provider_response_id: Option<ExternalId>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::json::optional"
    )]
    interruption: Option<InterruptionReason>,
}

pub(super) fn write(metadata: &ResponseMetadata, phase: &DraftPhase) -> Result<OpaqueJson, Error> {
    let interruption = match phase {
        DraftPhase::Interrupted { reason } => Some(reason.clone()),
        _ => None,
    };
    let header = Header {
        usage: metadata.usage().cloned(),
        model: metadata.model().cloned(),
        provider_response_id: metadata.provider_response_id().cloned(),
        interruption,
    };
    crate::json::encode_value(&header).map(OpaqueJson::from_inner)
}

pub(super) fn read(
    payload: &OpaqueJson,
) -> Result<(ResponseMetadata, Option<InterruptionReason>), Error> {
    let header: Header =
        crate::json::decode_ref(payload.inner()).map_err(|error| error.at("payload"))?;
    let mut metadata = ResponseMetadata::new();
    if let Some(usage) = header.usage {
        metadata = metadata.with_usage(usage);
    }
    if let Some(model) = header.model {
        metadata = metadata.with_model(model);
    }
    if let Some(id) = header.provider_response_id {
        metadata = metadata.with_provider_response_id(id);
    }
    Ok((metadata, header.interruption))
}
