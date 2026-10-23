use serde::{Deserialize, Serialize};

use super::ids::IdSink;
use super::plain::PlainBlock;
use crate::error::Error;
use crate::id::BlockId;
use crate::value::{ResultStatus, ToolName};

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "RuntimeToolResultWire")]
pub struct RuntimeToolResult {
    id: BlockId,
    call: BlockId,
    tool_name: ToolName,
    status: ResultStatus,
    content: Vec<PlainBlock>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RuntimeToolResultWire {
    id: BlockId,
    call: BlockId,
    tool_name: ToolName,
    status: ResultStatus,
    content: Vec<PlainBlock>,
}

impl RuntimeToolResult {
    pub fn new(
        id: BlockId,
        call: BlockId,
        tool_name: ToolName,
        status: ResultStatus,
        content: Vec<PlainBlock>,
    ) -> Result<Self, Error> {
        let result = RuntimeToolResult {
            id,
            call,
            tool_name,
            status,
            content,
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

    pub fn tool_name(&self) -> &ToolName {
        &self.tool_name
    }

    pub fn status(&self) -> ResultStatus {
        self.status
    }

    pub fn content(&self) -> &[PlainBlock] {
        &self.content
    }

    pub(crate) fn collect_ids(&self, sink: &mut IdSink) -> Result<(), Error> {
        sink.insert(self.id.as_uuid())?;
        PlainBlock::collect_all(&self.content, sink).map_err(|error| error.at("content"))
    }
}

impl TryFrom<RuntimeToolResultWire> for RuntimeToolResult {
    type Error = Error;

    fn try_from(wire: RuntimeToolResultWire) -> Result<Self, Error> {
        RuntimeToolResult::new(
            wire.id,
            wire.call,
            wire.tool_name,
            wire.status,
            wire.content,
        )
    }
}
