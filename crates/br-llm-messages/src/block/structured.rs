use serde::{Deserialize, Serialize};

use super::ids::IdSink;
use crate::error::Error;
use crate::id::BlockId;
use crate::opaque_json::OpaqueJson;
use crate::value::opaque;

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StructuredBlock {
    id: BlockId,
    #[serde(deserialize_with = "crate::value::opaque::deserialize")]
    value: OpaqueJson,
}

impl StructuredBlock {
    pub fn new(id: BlockId, value: OpaqueJson) -> Result<Self, Error> {
        opaque::check(&value).map_err(|error| error.at("value"))?;
        Ok(StructuredBlock { id, value })
    }

    pub fn id(&self) -> BlockId {
        self.id
    }

    pub fn value(&self) -> &OpaqueJson {
        &self.value
    }

    pub(crate) fn collect_ids(&self, sink: &mut IdSink) -> Result<(), Error> {
        sink.insert(self.id.as_uuid())
    }
}
