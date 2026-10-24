use serde::{Deserialize, Serialize};

use super::ids::IdSink;
use super::plain::PlainBlock;
use crate::error::{Error, ErrorCode};
use crate::id::BlockId;
use crate::value::{ExternalId, ObservedAt, ReplayMaterial};

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "ProviderToolResultWire")]
pub struct ProviderToolResult {
    id: BlockId,
    call: BlockId,
    #[serde(skip_serializing_if = "Option::is_none")]
    external_id: Option<ExternalId>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    content: Vec<PlainBlock>,
    #[serde(skip_serializing_if = "Option::is_none")]
    replay: Option<ReplayMaterial>,
    #[serde(skip_serializing_if = "Option::is_none")]
    observed_at: Option<ObservedAt>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProviderToolResultWire {
    id: BlockId,
    call: BlockId,
    #[serde(default, deserialize_with = "crate::json::optional")]
    external_id: Option<ExternalId>,
    #[serde(default)]
    content: Vec<PlainBlock>,
    #[serde(default, deserialize_with = "crate::json::optional")]
    replay: Option<ReplayMaterial>,
    #[serde(default, deserialize_with = "crate::json::optional")]
    observed_at: Option<ObservedAt>,
}

impl ProviderToolResult {
    pub fn new(id: BlockId, call: BlockId, content: Vec<PlainBlock>) -> Result<Self, Error> {
        if content.is_empty() {
            return Err(Error::new(ErrorCode::InvalidContent)
                .with_id(id.as_uuid())
                .at("content"));
        }
        ProviderToolResult::assemble(id, call, content, None)
    }

    pub fn replay_only(id: BlockId, call: BlockId, replay: ReplayMaterial) -> Result<Self, Error> {
        ProviderToolResult::assemble(id, call, Vec::new(), Some(replay))
    }

    pub fn with_external_id(mut self, external_id: ExternalId) -> Self {
        self.external_id = Some(external_id);
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

    fn assemble(
        id: BlockId,
        call: BlockId,
        content: Vec<PlainBlock>,
        replay: Option<ReplayMaterial>,
    ) -> Result<Self, Error> {
        let result = ProviderToolResult {
            id,
            call,
            external_id: None,
            content,
            replay,
            observed_at: None,
        };
        result.collect_ids(&mut IdSink::default())?;
        Ok(result)
    }

    pub fn id(&self) -> BlockId {
        self.id
    }

    pub fn call(&self) -> BlockId {
        self.call
    }

    pub fn external_id(&self) -> Option<&ExternalId> {
        self.external_id.as_ref()
    }

    pub fn content(&self) -> &[PlainBlock] {
        &self.content
    }

    pub fn replay(&self) -> Option<&ReplayMaterial> {
        self.replay.as_ref()
    }

    pub fn observed_at(&self) -> Option<&ObservedAt> {
        self.observed_at.as_ref()
    }

    pub(crate) fn collect_ids(&self, sink: &mut IdSink) -> Result<(), Error> {
        sink.insert(self.id.as_uuid())?;
        PlainBlock::collect_all(&self.content, sink).map_err(|error| error.at("content"))
    }
}

impl TryFrom<ProviderToolResultWire> for ProviderToolResult {
    type Error = Error;

    fn try_from(wire: ProviderToolResultWire) -> Result<Self, Error> {
        let mut result = match wire.replay {
            Some(replay) if wire.content.is_empty() => {
                ProviderToolResult::replay_only(wire.id, wire.call, replay)?
            }
            replay => {
                let mut result = ProviderToolResult::new(wire.id, wire.call, wire.content)?;
                result.replay = replay;
                result
            }
        };
        result.external_id = wire.external_id;
        result.observed_at = wire.observed_at;
        Ok(result)
    }
}
