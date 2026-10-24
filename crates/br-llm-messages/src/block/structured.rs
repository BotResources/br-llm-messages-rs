use serde::{Deserialize, Serialize};

use super::ids::IdSink;
use crate::error::Error;
use crate::id::BlockId;
use crate::opaque_json::OpaqueJson;
use crate::value::{ObservedAt, opaque};

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StructuredBlock {
    id: BlockId,
    #[serde(deserialize_with = "crate::value::opaque::value")]
    value: OpaqueJson,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    observed_at: Option<ObservedAt>,
}

impl StructuredBlock {
    pub fn new(id: BlockId, value: OpaqueJson) -> Result<Self, Error> {
        opaque::check(&value).map_err(|error| error.at("value"))?;
        Ok(StructuredBlock {
            id,
            value,
            observed_at: None,
        })
    }

    pub fn with_observed_at(mut self, observed_at: ObservedAt) -> Self {
        self.observed_at = Some(observed_at);
        self
    }

    pub fn id(&self) -> BlockId {
        self.id
    }

    pub fn value(&self) -> &OpaqueJson {
        &self.value
    }

    pub fn observed_at(&self) -> Option<&ObservedAt> {
        self.observed_at.as_ref()
    }

    pub(crate) fn collect_ids(&self, sink: &mut IdSink) -> Result<(), Error> {
        sink.insert(self.id.as_uuid())
    }
}
