use serde::{Deserialize, Serialize};

use super::exposure::ResultExposure;
use super::ids::IdSink;
use crate::error::Error;
use crate::id::BlockId;
use crate::value::{ExecutionStatus, ExternalId, ProviderOperation};

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
    exposure: ResultExposure,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProviderToolExecutionRecordWire {
    id: BlockId,
    external_id: Option<ExternalId>,
    operation: Option<ProviderOperation>,
    status: Option<ExecutionStatus>,
    exposure: ResultExposure,
}

impl ProviderToolExecutionRecord {
    pub fn new(id: BlockId, exposure: ResultExposure) -> Result<Self, Error> {
        let record = ProviderToolExecutionRecord {
            id,
            external_id: None,
            operation: None,
            status: None,
            exposure,
        };
        record.collect_ids(&mut IdSink::default())?;
        Ok(record)
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

    pub fn exposure(&self) -> &ResultExposure {
        &self.exposure
    }

    pub(crate) fn collect_ids(&self, sink: &mut IdSink) -> Result<(), Error> {
        sink.insert(self.id.as_uuid())?;
        self.exposure
            .collect_ids(sink)
            .map_err(|error| error.at("exposure"))
    }
}

impl TryFrom<ProviderToolExecutionRecordWire> for ProviderToolExecutionRecord {
    type Error = Error;

    fn try_from(wire: ProviderToolExecutionRecordWire) -> Result<Self, Error> {
        let mut record = ProviderToolExecutionRecord::new(wire.id, wire.exposure)?;
        record.external_id = wire.external_id;
        record.operation = wire.operation;
        record.status = wire.status;
        Ok(record)
    }
}
