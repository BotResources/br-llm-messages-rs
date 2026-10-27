use serde::{Deserialize, Serialize};

use crate::entry::ResponseMetadata;
use crate::value::{ExternalId, ModelName, Usage};

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResponseMetadataUpdate {
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
}

impl ResponseMetadataUpdate {
    #[allow(clippy::new_without_default)] // a Default derive would freeze the field set
    pub fn new() -> Self {
        ResponseMetadataUpdate {
            usage: None,
            model: None,
            provider_response_id: None,
        }
    }

    pub fn with_usage(mut self, usage: Usage) -> Self {
        self.usage = Some(usage);
        self
    }

    pub fn with_model(mut self, model: ModelName) -> Self {
        self.model = Some(model);
        self
    }

    pub fn with_provider_response_id(mut self, id: ExternalId) -> Self {
        self.provider_response_id = Some(id);
        self
    }

    pub fn usage(&self) -> Option<&Usage> {
        self.usage.as_ref()
    }

    pub fn model(&self) -> Option<&ModelName> {
        self.model.as_ref()
    }

    pub fn provider_response_id(&self) -> Option<&ExternalId> {
        self.provider_response_id.as_ref()
    }

    pub(crate) fn replacing(&self, base: &ResponseMetadata) -> ResponseMetadata {
        base.clone().overlaid(
            self.usage.clone(),
            self.model.clone(),
            self.provider_response_id.clone(),
        )
    }
}
