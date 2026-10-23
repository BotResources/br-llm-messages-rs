use serde::{Deserialize, Serialize};

use super::ids::IdSink;
use crate::error::Error;
use crate::id::BlockId;
use crate::value::ReplayMaterial;

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThinkingBlock {
    id: BlockId,
    text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    replay: Option<ReplayMaterial>,
}

impl ThinkingBlock {
    pub fn new(id: BlockId, text: impl Into<String>, replay: Option<ReplayMaterial>) -> Self {
        ThinkingBlock {
            id,
            text: text.into(),
            replay,
        }
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

    pub(crate) fn collect_ids(&self, sink: &mut IdSink) -> Result<(), Error> {
        sink.insert(self.id.as_uuid())
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RedactedThinkingBlock {
    id: BlockId,
    material: ReplayMaterial,
}

impl RedactedThinkingBlock {
    pub fn new(id: BlockId, material: ReplayMaterial) -> Self {
        RedactedThinkingBlock { id, material }
    }

    pub fn id(&self) -> BlockId {
        self.id
    }

    pub fn material(&self) -> &ReplayMaterial {
        &self.material
    }

    pub(crate) fn collect_ids(&self, sink: &mut IdSink) -> Result<(), Error> {
        sink.insert(self.id.as_uuid())
    }
}
