use crate::block::{ContentBlock, PlainBlock, TextBlock};
use crate::entry::{AgentOutput, IncomingEntry, ModelResponse, PartialAgentOutput};
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

pub(crate) fn accumulated_text(id: BlockId, text: &str) -> Option<TextBlock> {
    TextBlock::new(id, text).ok()
}

pub(crate) fn usable_block(block: &ContentBlock) -> Option<PlainBlock> {
    match block {
        ContentBlock::Text(text) => Some(PlainBlock::Text(text.clone())),
        ContentBlock::Image(image) => Some(PlainBlock::Image(image.clone())),
        ContentBlock::StructuredContent(structured) => {
            Some(PlainBlock::StructuredContent(structured.clone()))
        }
        _ => None,
    }
}
