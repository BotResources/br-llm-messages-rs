use serde::{Deserialize, Serialize};

use super::{ToolDefinition, refuse_author};
use crate::block::IdSink;
use crate::error::Error;
use crate::id::EntryId;

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "ToolMountedWire")]
pub struct ToolMounted {
    id: EntryId,
    definition: ToolDefinition,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ToolMountedWire {
    id: EntryId,
    definition: ToolDefinition,
    #[serde(default, deserialize_with = "super::present")]
    author: bool,
}

impl ToolMounted {
    pub fn new(id: EntryId, definition: ToolDefinition) -> Self {
        ToolMounted { id, definition }
    }

    pub fn id(&self) -> EntryId {
        self.id
    }

    pub fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    pub(crate) fn collect_ids(&self, sink: &mut IdSink) -> Result<(), Error> {
        sink.insert(self.id.as_uuid())
    }
}

impl TryFrom<ToolMountedWire> for ToolMounted {
    type Error = Error;

    fn try_from(wire: ToolMountedWire) -> Result<Self, Error> {
        refuse_author(wire.author, wire.id)?;
        Ok(ToolMounted::new(wire.id, wire.definition))
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "ToolUnmountedWire")]
pub struct ToolUnmounted {
    id: EntryId,
    mounting: EntryId,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ToolUnmountedWire {
    id: EntryId,
    mounting: EntryId,
    #[serde(default, deserialize_with = "super::present")]
    author: bool,
}

impl ToolUnmounted {
    pub fn new(id: EntryId, mounting: EntryId) -> Self {
        ToolUnmounted { id, mounting }
    }

    pub fn id(&self) -> EntryId {
        self.id
    }

    pub fn mounting(&self) -> EntryId {
        self.mounting
    }

    pub(crate) fn collect_ids(&self, sink: &mut IdSink) -> Result<(), Error> {
        sink.insert(self.id.as_uuid())
    }
}

impl TryFrom<ToolUnmountedWire> for ToolUnmounted {
    type Error = Error;

    fn try_from(wire: ToolUnmountedWire) -> Result<Self, Error> {
        refuse_author(wire.author, wire.id)?;
        Ok(ToolUnmounted::new(wire.id, wire.mounting))
    }
}
