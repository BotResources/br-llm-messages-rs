use serde::{Deserialize, Serialize};

use crate::value::{ExternalId, ModelName, ObservedAt, Usage};

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResponseMetadata {
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::json::optional"
    )]
    pub(super) usage: Option<Usage>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::json::optional"
    )]
    pub(super) model: Option<ModelName>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::json::optional"
    )]
    pub(super) provider_response_id: Option<ExternalId>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::json::optional"
    )]
    pub(super) observed_at: Option<ObservedAt>,
}

impl ResponseMetadata {
    #[allow(clippy::new_without_default)] // a Default derive would freeze the field set
    pub fn new() -> Self {
        ResponseMetadata {
            usage: None,
            model: None,
            provider_response_id: None,
            observed_at: None,
        }
    }

    pub(crate) fn from_header(
        usage: Option<Usage>,
        model: Option<ModelName>,
        provider_response_id: Option<ExternalId>,
    ) -> Self {
        ResponseMetadata {
            usage,
            model,
            provider_response_id,
            observed_at: None,
        }
    }

    pub(crate) fn overlaid(
        self,
        usage: Option<Usage>,
        model: Option<ModelName>,
        provider_response_id: Option<ExternalId>,
    ) -> Self {
        let ResponseMetadata {
            usage: kept_usage,
            model: kept_model,
            provider_response_id: kept_provider_response_id,
            observed_at,
        } = self;
        ResponseMetadata {
            usage: usage.or(kept_usage),
            model: model.or(kept_model),
            provider_response_id: provider_response_id.or(kept_provider_response_id),
            observed_at,
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

    pub fn with_observed_at(mut self, observed_at: ObservedAt) -> Self {
        self.observed_at = Some(observed_at);
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

    pub fn observed_at(&self) -> Option<&ObservedAt> {
        self.observed_at.as_ref()
    }
}
