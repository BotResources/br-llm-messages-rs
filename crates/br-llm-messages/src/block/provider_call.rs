use serde::{Deserialize, Serialize};

use super::ids::IdSink;
use crate::error::Error;
use crate::id::BlockId;
use crate::opaque_json::OpaqueJson;
use crate::value::{ExternalId, ItemRef, ObservedAt, ProviderOperation, ReplayMaterial, opaque};

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderToolCall {
    id: BlockId,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::json::optional"
    )]
    external_id: Option<ExternalId>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::json::optional"
    )]
    item_ref: Option<ItemRef>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::json::optional"
    )]
    operation: Option<ProviderOperation>,
    #[serde(
        default,
        deserialize_with = "opaque::optional_arguments",
        skip_serializing_if = "Option::is_none"
    )]
    arguments: Option<OpaqueJson>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::json::optional"
    )]
    replay: Option<ReplayMaterial>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::json::optional"
    )]
    observed_at: Option<ObservedAt>,
}

impl ProviderToolCall {
    pub fn new(id: BlockId) -> Self {
        ProviderToolCall {
            id,
            external_id: None,
            item_ref: None,
            operation: None,
            arguments: None,
            replay: None,
            observed_at: None,
        }
    }

    pub fn with_observed_at(mut self, observed_at: ObservedAt) -> Self {
        self.observed_at = Some(observed_at);
        self
    }

    pub fn with_external_id(mut self, external_id: ExternalId) -> Self {
        self.external_id = Some(external_id);
        self
    }

    pub fn with_item_ref(mut self, item_ref: ItemRef) -> Self {
        self.item_ref = Some(item_ref);
        self
    }

    pub fn with_operation(mut self, operation: ProviderOperation) -> Self {
        self.operation = Some(operation);
        self
    }

    pub fn with_arguments(mut self, arguments: OpaqueJson) -> Result<Self, Error> {
        opaque::check_present(&arguments).map_err(|error| error.at("arguments"))?;
        self.arguments = Some(arguments);
        Ok(self)
    }

    pub fn with_replay(mut self, replay: ReplayMaterial) -> Self {
        self.replay = Some(replay);
        self
    }

    pub fn id(&self) -> BlockId {
        self.id
    }

    pub fn external_id(&self) -> Option<&ExternalId> {
        self.external_id.as_ref()
    }

    pub fn item_ref(&self) -> Option<&ItemRef> {
        self.item_ref.as_ref()
    }

    pub fn operation(&self) -> Option<&ProviderOperation> {
        self.operation.as_ref()
    }

    pub fn arguments(&self) -> Option<&OpaqueJson> {
        self.arguments.as_ref()
    }

    pub fn replay(&self) -> Option<&ReplayMaterial> {
        self.replay.as_ref()
    }

    pub fn observed_at(&self) -> Option<&ObservedAt> {
        self.observed_at.as_ref()
    }

    pub(crate) fn collect_ids(&self, sink: &mut IdSink) -> Result<(), Error> {
        sink.insert(self.id.as_uuid())
    }
}
