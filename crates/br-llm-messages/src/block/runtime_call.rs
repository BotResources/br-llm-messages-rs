use serde::{Deserialize, Serialize};

use super::ids::IdSink;
use crate::error::{Error, ErrorCode};
use crate::id::BlockId;
use crate::opaque_json::OpaqueJson;
use crate::value::{ExternalId, ItemRef, ObservedAt, ReplayMaterial, ToolName, opaque};

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "RuntimeToolCallWire")]
pub struct RuntimeToolCall {
    id: BlockId,
    #[serde(skip_serializing_if = "Option::is_none")]
    external_id: Option<ExternalId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    item_ref: Option<ItemRef>,
    name: ToolName,
    arguments: OpaqueJson,
    #[serde(skip_serializing_if = "Option::is_none")]
    replay: Option<ReplayMaterial>,
    #[serde(skip_serializing_if = "Option::is_none")]
    observed_at: Option<ObservedAt>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RuntimeToolCallWire {
    id: BlockId,
    #[serde(default, deserialize_with = "crate::json::optional")]
    external_id: Option<ExternalId>,
    #[serde(default, deserialize_with = "crate::json::optional")]
    item_ref: Option<ItemRef>,
    name: ToolName,
    #[serde(deserialize_with = "crate::value::opaque::arguments")]
    arguments: OpaqueJson,
    #[serde(default, deserialize_with = "crate::json::optional")]
    replay: Option<ReplayMaterial>,
    #[serde(default, deserialize_with = "crate::json::optional")]
    observed_at: Option<ObservedAt>,
}

impl RuntimeToolCall {
    pub fn new(id: BlockId, name: ToolName, arguments: OpaqueJson) -> Result<Self, Error> {
        opaque::check(&arguments).map_err(|error| error.with_id(id.as_uuid()).at("arguments"))?;
        if !arguments.inner().is_object() {
            return Err(Error::new(ErrorCode::InvalidContent)
                .with_id(id.as_uuid())
                .at("arguments"));
        }
        Ok(RuntimeToolCall {
            id,
            external_id: None,
            item_ref: None,
            name,
            arguments,
            replay: None,
            observed_at: None,
        })
    }

    pub fn with_external_id(mut self, external_id: ExternalId) -> Self {
        self.external_id = Some(external_id);
        self
    }

    pub fn with_item_ref(mut self, item_ref: ItemRef) -> Self {
        self.item_ref = Some(item_ref);
        self
    }

    pub fn with_replay(mut self, replay: ReplayMaterial) -> Self {
        self.replay = Some(replay);
        self
    }

    pub fn with_observed_at(mut self, observed_at: ObservedAt) -> Self {
        self.observed_at = Some(observed_at);
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

    pub fn name(&self) -> &ToolName {
        &self.name
    }

    pub fn arguments(&self) -> &OpaqueJson {
        &self.arguments
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

impl TryFrom<RuntimeToolCallWire> for RuntimeToolCall {
    type Error = Error;

    fn try_from(wire: RuntimeToolCallWire) -> Result<Self, Error> {
        let mut call = RuntimeToolCall::new(wire.id, wire.name, wire.arguments)?;
        call.external_id = wire.external_id;
        call.item_ref = wire.item_ref;
        call.replay = wire.replay;
        call.observed_at = wire.observed_at;
        Ok(call)
    }
}
