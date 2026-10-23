use serde::{Deserialize, Serialize};

use super::ids::IdSink;
use crate::error::Error;
use crate::id::BlockId;
use crate::value::ImageSource;

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImageBlock {
    id: BlockId,
    source: ImageSource,
}

impl ImageBlock {
    pub fn new(id: BlockId, source: ImageSource) -> Self {
        ImageBlock { id, source }
    }

    pub fn id(&self) -> BlockId {
        self.id
    }

    pub fn source(&self) -> &ImageSource {
        &self.source
    }

    pub(crate) fn collect_ids(&self, sink: &mut IdSink) -> Result<(), Error> {
        sink.insert(self.id.as_uuid())
    }
}
