use crate::block::{ContentBlock, PlainBlock, TextBlock};
use crate::entry::{AgentOutput, IncomingEntry, ModelResponse, PartialAgentOutput};
use crate::error::{Error, ErrorCode};
use crate::id::{AgentId, BlockId, EntryId};
use crate::value::Author;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Owner {
    pub(crate) agent: AgentId,
    pub(crate) author: Author,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Progress {
    Completed,
    Producing,
}

pub(crate) struct Output {
    entry: EntryId,
    progress: Progress,
    blocks: Vec<PlainBlock>,
}

impl Output {
    pub(crate) fn new(entry: EntryId, progress: Progress, blocks: Vec<PlainBlock>) -> Self {
        Output {
            entry,
            progress,
            blocks: blocks.iter().map(PlainBlock::bare).collect(),
        }
    }

    pub(crate) fn of_response(response: &ModelResponse) -> Self {
        Output::new(
            response.id(),
            Progress::Completed,
            response.blocks().iter().filter_map(usable_block).collect(),
        )
    }

    pub(crate) fn entry_for(self, owner: &Owner) -> Option<IncomingEntry> {
        if self.blocks.is_empty() {
            return None;
        }
        let (agent, author) = (owner.agent, owner.author.clone());
        Some(match self.progress {
            Progress::Completed => {
                AgentOutput::from_usable(self.entry, agent, author, self.blocks).into()
            }
            Progress::Producing => {
                PartialAgentOutput::from_usable(self.entry, agent, author, self.blocks).into()
            }
        })
    }
}

pub(crate) fn accumulated_text(id: BlockId, text: &str) -> Result<Option<TextBlock>, Error> {
    if text.chars().all(char::is_whitespace) {
        return Ok(None);
    }
    TextBlock::new(id, text).map(Some)
}

pub(crate) fn usable(block: &ContentBlock) -> Result<PlainBlock, Error> {
    match block {
        ContentBlock::Text(text) => Ok(PlainBlock::Text(text.clone())),
        ContentBlock::Image(image) => Ok(PlainBlock::Image(image.clone())),
        ContentBlock::StructuredContent(structured) => {
            Ok(PlainBlock::StructuredContent(structured.clone()))
        }
        other => Err(Error::new(ErrorCode::DisallowedKind).with_id(other.id().as_uuid())),
    }
}

pub(crate) fn usable_block(block: &ContentBlock) -> Option<PlainBlock> {
    usable(block).ok()
}
