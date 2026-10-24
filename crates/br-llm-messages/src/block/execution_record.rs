use serde::{Deserialize, Serialize};

use super::exposure::ResultExposure;
use super::ids::IdSink;
use super::plain::PlainBlock;
use crate::error::Error;
use crate::id::BlockId;
use crate::value::{ExecutionStatus, ExternalId, ObservedAt, ProviderOperation, ReplayMaterial};

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "ProviderToolExecutionRecordWire")]
pub struct ProviderToolExecutionRecord {
    id: BlockId,
    #[serde(skip_serializing_if = "Option::is_none")]
    external_id: Option<ExternalId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    operation: Option<ProviderOperation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    status: Option<ExecutionStatus>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    content: Vec<PlainBlock>,
    #[serde(skip_serializing_if = "Option::is_none")]
    replay: Option<ReplayMaterial>,
    #[serde(skip_serializing_if = "Option::is_none")]
    observed_at: Option<ObservedAt>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProviderToolExecutionRecordWire {
    id: BlockId,
    #[serde(default, deserialize_with = "crate::json::optional")]
    external_id: Option<ExternalId>,
    #[serde(default, deserialize_with = "crate::json::optional")]
    operation: Option<ProviderOperation>,
    #[serde(default, deserialize_with = "crate::json::optional")]
    status: Option<ExecutionStatus>,
    #[serde(default)]
    content: Vec<PlainBlock>,
    #[serde(default, deserialize_with = "crate::json::optional")]
    replay: Option<ReplayMaterial>,
    #[serde(default, deserialize_with = "crate::json::optional")]
    observed_at: Option<ObservedAt>,
}

impl ProviderToolExecutionRecord {
    pub fn new(id: BlockId) -> Self {
        ProviderToolExecutionRecord {
            id,
            external_id: None,
            operation: None,
            status: None,
            content: Vec::new(),
            replay: None,
            observed_at: None,
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

    pub fn with_status(mut self, status: ExecutionStatus) -> Self {
        self.status = Some(status);
        self
    }

    pub fn with_content(mut self, content: Vec<PlainBlock>) -> Result<Self, Error> {
        self.content = content;
        self.collect_ids(&mut IdSink::default())?;
        Ok(self)
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

    pub fn operation(&self) -> Option<&ProviderOperation> {
        self.operation.as_ref()
    }

    pub fn status(&self) -> Option<ExecutionStatus> {
        self.status.clone()
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

    pub fn exposure(&self) -> ResultExposure {
        ResultExposure::of(!self.content.is_empty(), self.replay.is_some())
    }

    pub(crate) fn collect_ids(&self, sink: &mut IdSink) -> Result<(), Error> {
        sink.insert(self.id.as_uuid())?;
        PlainBlock::collect_all(&self.content, sink).map_err(|error| error.at("content"))
    }
}

impl TryFrom<ProviderToolExecutionRecordWire> for ProviderToolExecutionRecord {
    type Error = Error;

    fn try_from(wire: ProviderToolExecutionRecordWire) -> Result<Self, Error> {
        let mut record = ProviderToolExecutionRecord::new(wire.id).with_content(wire.content)?;
        record.external_id = wire.external_id;
        record.operation = wire.operation;
        record.status = wire.status;
        record.replay = wire.replay;
        record.observed_at = wire.observed_at;
        Ok(record)
    }
}
