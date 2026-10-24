use serde::{Deserialize, Serialize};

use super::ids::IdSink;
use crate::error::{Error, ErrorCode};
use crate::id::BlockId;
use crate::value::{ObservedAt, ReplayMaterial};

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "ThinkingBlockWire")]
pub struct ThinkingBlock {
    id: BlockId,
    text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    replay: Option<ReplayMaterial>,
    #[serde(skip_serializing_if = "Option::is_none")]
    observed_at: Option<ObservedAt>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ThinkingBlockWire {
    id: BlockId,
    text: String,
    #[serde(default, deserialize_with = "crate::json::optional")]
    replay: Option<ReplayMaterial>,
    #[serde(default, deserialize_with = "crate::json::optional")]
    observed_at: Option<ObservedAt>,
}

impl ThinkingBlock {
    pub fn new(id: BlockId, text: impl Into<String>) -> Result<Self, Error> {
        ThinkingBlock::assemble(id, text.into(), None)
    }

    pub fn replay_only(id: BlockId, replay: ReplayMaterial) -> Self {
        ThinkingBlock {
            id,
            text: String::new(),
            replay: Some(replay),
            observed_at: None,
        }
    }

    pub fn with_observed_at(mut self, observed_at: ObservedAt) -> Self {
        self.observed_at = Some(observed_at);
        self
    }

    pub fn with_replay(mut self, replay: ReplayMaterial) -> Self {
        self.replay = Some(replay);
        self
    }

    fn assemble(id: BlockId, text: String, replay: Option<ReplayMaterial>) -> Result<Self, Error> {
        if text.is_empty() && replay.is_none() {
            return Err(Error::new(ErrorCode::InvalidContent)
                .with_id(id.as_uuid())
                .at("text"));
        }
        Ok(ThinkingBlock {
            id,
            text,
            replay,
            observed_at: None,
        })
    }

    pub fn id(&self) -> BlockId {
        self.id
    }

    pub fn text(&self) -> &str {
        &self.text
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

impl TryFrom<ThinkingBlockWire> for ThinkingBlock {
    type Error = Error;

    fn try_from(wire: ThinkingBlockWire) -> Result<Self, Error> {
        let mut block = ThinkingBlock::assemble(wire.id, wire.text, wire.replay)?;
        block.observed_at = wire.observed_at;
        Ok(block)
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RedactedThinkingBlock {
    id: BlockId,
    material: ReplayMaterial,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::json::optional"
    )]
    observed_at: Option<ObservedAt>,
}

impl RedactedThinkingBlock {
    pub fn new(id: BlockId, material: ReplayMaterial) -> Self {
        RedactedThinkingBlock {
            id,
            material,
            observed_at: None,
        }
    }

    pub fn with_observed_at(mut self, observed_at: ObservedAt) -> Self {
        self.observed_at = Some(observed_at);
        self
    }

    pub fn id(&self) -> BlockId {
        self.id
    }

    pub fn material(&self) -> &ReplayMaterial {
        &self.material
    }

    pub fn observed_at(&self) -> Option<&ObservedAt> {
        self.observed_at.as_ref()
    }

    pub(crate) fn collect_ids(&self, sink: &mut IdSink) -> Result<(), Error> {
        sink.insert(self.id.as_uuid())
    }
}
