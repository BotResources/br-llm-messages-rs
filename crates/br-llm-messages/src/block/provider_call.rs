use serde::{Deserialize, Deserializer, Serialize};

use super::ids::IdSink;
use crate::error::Error;
use crate::id::BlockId;
use crate::opaque_json::OpaqueJson;
use crate::value::{ExternalId, ProviderOperation, ReplayMaterial, opaque};

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderToolCall {
    id: BlockId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    external_id: Option<ExternalId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    operation: Option<ProviderOperation>,
    #[serde(
        default,
        deserialize_with = "present_value",
        skip_serializing_if = "Option::is_none"
    )]
    arguments: Option<OpaqueJson>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    replay: Option<ReplayMaterial>,
}

fn present_value<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<OpaqueJson>, D::Error> {
    opaque::deserialize(deserializer).map(Some)
}

impl ProviderToolCall {
    pub fn new(id: BlockId) -> Self {
        ProviderToolCall {
            id,
            external_id: None,
            operation: None,
            arguments: None,
            replay: None,
        }
    }

    pub fn with_external_id(mut self, external_id: ExternalId) -> Self {
        self.external_id = Some(external_id);
        self
    }

    pub fn with_operation(mut self, operation: ProviderOperation) -> Self {
        self.operation = Some(operation);
        self
    }

    pub fn with_arguments(mut self, arguments: OpaqueJson) -> Result<Self, Error> {
        opaque::check(&arguments).map_err(|error| error.at("arguments"))?;
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

    pub fn operation(&self) -> Option<&ProviderOperation> {
        self.operation.as_ref()
    }

    pub fn arguments(&self) -> Option<&OpaqueJson> {
        self.arguments.as_ref()
    }

    pub fn replay(&self) -> Option<&ReplayMaterial> {
        self.replay.as_ref()
    }

    pub(crate) fn collect_ids(&self, sink: &mut IdSink) -> Result<(), Error> {
        sink.insert(self.id.as_uuid())
    }
}
