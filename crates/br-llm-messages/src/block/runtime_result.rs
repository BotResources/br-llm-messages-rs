use serde::{Deserialize, Serialize};

use super::ids::IdSink;
use super::plain::PlainBlock;
use crate::error::Error;
use crate::id::BlockId;
use crate::value::{ObservedAt, ToolName};

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "RuntimeToolResultWire")]
pub struct RuntimeToolResult {
    id: BlockId,
    call: BlockId,
    tool_name: ToolName,
    is_error: bool,
    content: Vec<PlainBlock>,
    #[serde(skip_serializing_if = "Option::is_none")]
    observed_at: Option<ObservedAt>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RuntimeToolResultWire {
    id: BlockId,
    call: BlockId,
    tool_name: ToolName,
    is_error: bool,
    content: Vec<PlainBlock>,
    observed_at: Option<ObservedAt>,
}

impl RuntimeToolResult {
    pub fn new(
        id: BlockId,
        call: BlockId,
        tool_name: ToolName,
        is_error: bool,
        content: Vec<PlainBlock>,
    ) -> Result<Self, Error> {
        let result = RuntimeToolResult {
            id,
            call,
            tool_name,
            is_error,
            content,
            observed_at: None,
        };
        result.collect_ids(&mut IdSink::default())?;
        Ok(result)
    }

    pub fn with_observed_at(mut self, observed_at: ObservedAt) -> Self {
        self.observed_at = Some(observed_at);
        self
    }

    pub fn id(&self) -> BlockId {
        self.id
    }

    pub fn call(&self) -> BlockId {
        self.call
    }

    pub fn tool_name(&self) -> &ToolName {
        &self.tool_name
    }

    pub fn is_error(&self) -> bool {
        self.is_error
    }

    pub fn content(&self) -> &[PlainBlock] {
        &self.content
    }

    pub fn observed_at(&self) -> Option<&ObservedAt> {
        self.observed_at.as_ref()
    }

    pub(crate) fn collect_ids(&self, sink: &mut IdSink) -> Result<(), Error> {
        sink.insert(self.id.as_uuid())?;
        PlainBlock::collect_all(&self.content, sink).map_err(|error| error.at("content"))
    }
}

impl TryFrom<RuntimeToolResultWire> for RuntimeToolResult {
    type Error = Error;

    fn try_from(wire: RuntimeToolResultWire) -> Result<Self, Error> {
        let mut result = RuntimeToolResult::new(
            wire.id,
            wire.call,
            wire.tool_name,
            wire.is_error,
            wire.content,
        )?;
        result.observed_at = wire.observed_at;
        Ok(result)
    }
}
