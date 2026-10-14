use crate::error::MessageError;
use crate::step::Step;
use crate::stop_reason::StopReason;
use crate::usage::Usage;
use crate::value::{ModelId, ToolCallId, ToolName};

use super::assemble::assemble;
use super::event::{BlockKind, StreamEvent};

/// A block as the stream fills it, before it is assembled.
pub(super) enum DraftBlock {
    Text {
        text: String,
    },
    Thinking {
        text: String,
        signature: Option<String>,
    },
    RedactedThinking {
        data: String,
    },
    Structured {
        json: String,
    },
    ToolCall {
        id: ToolCallId,
        name: ToolName,
        arguments: String,
    },
}

#[derive(Default)]
pub struct StepDraft {
    blocks: Vec<DraftBlock>,
    open: Option<usize>,
    finish: Option<(StopReason, Option<Usage>)>,
}

impl StepDraft {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn apply(&mut self, event: StreamEvent) -> Result<(), MessageError> {
        if self.finish.is_some() {
            return Err(MessageError::EventAfterFinish);
        }
        match event {
            StreamEvent::BlockStart { index, kind } => self.start(index, kind),
            StreamEvent::ToolCallStart { index, id, name } => self.start_tool_call(index, id, name),
            StreamEvent::TextDelta { index, text } => self.push_text(index, &text),
            StreamEvent::ThinkingDelta { index, text } => self.push_thinking(index, &text),
            StreamEvent::SignatureDelta { index, signature } => {
                self.push_signature(index, &signature)
            }
            StreamEvent::RedactedThinkingData { index, data } => self.push_redacted(index, &data),
            StreamEvent::StructuredDelta {
                index,
                json_fragment,
            } => self.push_structured(index, &json_fragment),
            StreamEvent::ToolCallArgumentsDelta {
                index,
                json_fragment,
            } => self.push_tool_arguments(index, &json_fragment),
            StreamEvent::BlockEnd { index } => self.end(index),
            StreamEvent::Finish { stop_reason, usage } => self.finish_event(stop_reason, usage),
        }
    }

    fn start(&mut self, index: usize, kind: BlockKind) -> Result<(), MessageError> {
        self.expect_next(index)?;
        let block = match kind {
            BlockKind::Text => DraftBlock::Text {
                text: String::new(),
            },
            BlockKind::Thinking => DraftBlock::Thinking {
                text: String::new(),
                signature: None,
            },
            BlockKind::RedactedThinking => DraftBlock::RedactedThinking {
                data: String::new(),
            },
            BlockKind::Structured => DraftBlock::Structured {
                json: String::new(),
            },
            BlockKind::ToolCall => return Err(MessageError::ToolCallNeedsStart),
        };
        self.blocks.push(block);
        self.open = Some(index);
        Ok(())
    }

    fn start_tool_call(
        &mut self,
        index: usize,
        id: ToolCallId,
        name: ToolName,
    ) -> Result<(), MessageError> {
        self.expect_next(index)?;
        self.blocks.push(DraftBlock::ToolCall {
            id,
            name,
            arguments: String::new(),
        });
        self.open = Some(index);
        Ok(())
    }

    fn expect_next(&self, index: usize) -> Result<(), MessageError> {
        if let Some(open) = self.open {
            return Err(MessageError::BlockStillOpen { index: open });
        }
        if index != self.blocks.len() {
            return Err(MessageError::UnexpectedBlockIndex {
                expected: self.blocks.len(),
                given: index,
            });
        }
        Ok(())
    }

    fn open_block(&mut self, index: usize) -> Result<&mut DraftBlock, MessageError> {
        match self.open {
            Some(open) if open == index => Ok(&mut self.blocks[open]),
            _ => Err(MessageError::NoOpenBlock { index }),
        }
    }

    fn push_text(&mut self, index: usize, fragment: &str) -> Result<(), MessageError> {
        match self.open_block(index)? {
            DraftBlock::Text { text } => {
                text.push_str(fragment);
                Ok(())
            }
            DraftBlock::Thinking { .. }
            | DraftBlock::RedactedThinking { .. }
            | DraftBlock::Structured { .. }
            | DraftBlock::ToolCall { .. } => Err(MessageError::BlockKindMismatch { index }),
        }
    }

    fn push_thinking(&mut self, index: usize, fragment: &str) -> Result<(), MessageError> {
        match self.open_block(index)? {
            DraftBlock::Thinking { text, .. } => {
                text.push_str(fragment);
                Ok(())
            }
            DraftBlock::Text { .. }
            | DraftBlock::RedactedThinking { .. }
            | DraftBlock::Structured { .. }
            | DraftBlock::ToolCall { .. } => Err(MessageError::BlockKindMismatch { index }),
        }
    }

    fn push_signature(&mut self, index: usize, fragment: &str) -> Result<(), MessageError> {
        match self.open_block(index)? {
            DraftBlock::Thinking { signature, .. } => {
                signature.get_or_insert_with(String::new).push_str(fragment);
                Ok(())
            }
            DraftBlock::Text { .. }
            | DraftBlock::RedactedThinking { .. }
            | DraftBlock::Structured { .. }
            | DraftBlock::ToolCall { .. } => Err(MessageError::BlockKindMismatch { index }),
        }
    }

    fn push_redacted(&mut self, index: usize, fragment: &str) -> Result<(), MessageError> {
        match self.open_block(index)? {
            DraftBlock::RedactedThinking { data } => {
                data.push_str(fragment);
                Ok(())
            }
            DraftBlock::Text { .. }
            | DraftBlock::Thinking { .. }
            | DraftBlock::Structured { .. }
            | DraftBlock::ToolCall { .. } => Err(MessageError::BlockKindMismatch { index }),
        }
    }

    fn push_structured(&mut self, index: usize, fragment: &str) -> Result<(), MessageError> {
        match self.open_block(index)? {
            DraftBlock::Structured { json } => {
                json.push_str(fragment);
                Ok(())
            }
            DraftBlock::Text { .. }
            | DraftBlock::Thinking { .. }
            | DraftBlock::RedactedThinking { .. }
            | DraftBlock::ToolCall { .. } => Err(MessageError::BlockKindMismatch { index }),
        }
    }

    fn push_tool_arguments(&mut self, index: usize, fragment: &str) -> Result<(), MessageError> {
        match self.open_block(index)? {
            DraftBlock::ToolCall { arguments, .. } => {
                arguments.push_str(fragment);
                Ok(())
            }
            DraftBlock::Text { .. }
            | DraftBlock::Thinking { .. }
            | DraftBlock::RedactedThinking { .. }
            | DraftBlock::Structured { .. } => Err(MessageError::BlockKindMismatch { index }),
        }
    }

    fn end(&mut self, index: usize) -> Result<(), MessageError> {
        match self.open {
            Some(open) if open == index => {
                self.open = None;
                Ok(())
            }
            _ => Err(MessageError::NoOpenBlock { index }),
        }
    }

    fn finish_event(
        &mut self,
        stop_reason: StopReason,
        usage: Option<Usage>,
    ) -> Result<(), MessageError> {
        if let Some(open) = self.open {
            return Err(MessageError::BlockStillOpen { index: open });
        }
        self.finish = Some((stop_reason, usage));
        Ok(())
    }

    /// Builds the step: every blank text block is dropped (providers send
    /// them as normal output), the other blocks are assembled under the Step
    /// rules, and a reply left with no block is refused as `EmptyStep`.
    pub fn finish(self, model: Option<ModelId>) -> Result<Step, MessageError> {
        let Some((stop_reason, usage)) = self.finish else {
            return Err(MessageError::MissingFinish);
        };
        let mut content = Vec::with_capacity(self.blocks.len());
        for block in self.blocks {
            if let Some(block) = assemble(block)? {
                content.push(block);
            }
        }
        Step::new(content, stop_reason, usage, model)
    }
}

#[cfg(test)]
#[path = "draft_protocol_tests.rs"]
mod protocol_tests;

#[cfg(test)]
#[path = "draft_assembly_tests.rs"]
mod assembly_tests;

#[cfg(test)]
#[path = "draft_assembly_error_tests.rs"]
mod assembly_error_tests;

#[cfg(test)]
#[path = "draft_blank_text_tests.rs"]
mod blank_text_tests;
