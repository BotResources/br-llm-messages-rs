use serde::{Deserialize, Serialize};

use super::ids::IdSink;
use crate::error::Error;
use crate::id::BlockId;
use crate::value::{ImageSource, ObservedAt};

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImageBlock {
    id: BlockId,
    source: ImageSource,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::json::optional"
    )]
    observed_at: Option<ObservedAt>,
}

impl ImageBlock {
    pub fn new(id: BlockId, source: ImageSource) -> Self {
        ImageBlock {
            id,
            source,
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

    pub fn source(&self) -> &ImageSource {
        &self.source
    }

    pub fn observed_at(&self) -> Option<&ObservedAt> {
        self.observed_at.as_ref()
    }

    pub(crate) fn collect_ids(&self, sink: &mut IdSink) -> Result<(), Error> {
        sink.insert(self.id.as_uuid())
    }
}
