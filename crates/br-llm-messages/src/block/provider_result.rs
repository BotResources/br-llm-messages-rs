use serde::{Deserialize, Serialize};

use super::ids::IdSink;
use super::plain::PlainBlock;
use crate::error::{Error, ErrorCode};
use crate::id::BlockId;
use crate::value::{ExternalId, ReplayMaterial};

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProviderToolResultWire {
    id: BlockId,
    call: BlockId,
    external_id: Option<ExternalId>,
    #[serde(default)]
    content: Vec<PlainBlock>,
    replay: Option<ReplayMaterial>,
}

impl ProviderToolResult {
    pub fn new(
        id: BlockId,
        call: BlockId,
        external_id: Option<ExternalId>,
        content: Vec<PlainBlock>,
        replay: Option<ReplayMaterial>,
    ) -> Result<Self, Error> {
        if content.is_empty() && replay.is_none() {
            return Err(Error::new(ErrorCode::MissingContent)
                .with_id(id.as_uuid())
                .at("content"));
        }
        let result = ProviderToolResult {
            id,
            call,
            external_id,
            content,
            replay,
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

    pub(crate) fn collect_ids(&self, sink: &mut IdSink) -> Result<(), Error> {
        sink.insert(self.id.as_uuid())?;
        PlainBlock::collect_all(&self.content, sink).map_err(|error| error.at("content"))
    }
}

impl TryFrom<ProviderToolResultWire> for ProviderToolResult {
    type Error = Error;

    fn try_from(wire: ProviderToolResultWire) -> Result<Self, Error> {
        ProviderToolResult::new(
            wire.id,
            wire.call,
            wire.external_id,
            wire.content,
            wire.replay,
        )
    }
}
