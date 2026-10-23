use serde::{Deserialize, Serialize};

use super::ids::IdSink;
use crate::error::Error;
use crate::id::BlockId;
use crate::opaque_json::OpaqueJson;
use crate::value::{ExternalId, ReplayMaterial, ToolName, opaque};

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeToolCall {
    id: BlockId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    external_id: Option<ExternalId>,
    name: ToolName,
    #[serde(deserialize_with = "crate::value::opaque::deserialize")]
    arguments: OpaqueJson,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    replay: Option<ReplayMaterial>,
}

impl RuntimeToolCall {
    pub fn new(id: BlockId, name: ToolName, arguments: OpaqueJson) -> Result<Self, Error> {
        opaque::check(&arguments).map_err(|error| error.at("arguments"))?;
        Ok(RuntimeToolCall {
            id,
            external_id: None,
            name,
            arguments,
            replay: None,
        })
    }

    pub fn with_external_id(mut self, external_id: ExternalId) -> Self {
        self.external_id = Some(external_id);
        self
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

    pub fn name(&self) -> &ToolName {
        &self.name
    }

    pub fn arguments(&self) -> &OpaqueJson {
        &self.arguments
    }

    pub fn replay(&self) -> Option<&ReplayMaterial> {
        self.replay.as_ref()
    }

    pub(crate) fn collect_ids(&self, sink: &mut IdSink) -> Result<(), Error> {
        sink.insert(self.id.as_uuid())
    }
}
