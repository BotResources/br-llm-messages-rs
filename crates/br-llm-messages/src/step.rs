use crate::block::{AssistantBlock, ToolCall};
use crate::error::MessageError;
use crate::stop_reason::StopReason;
use crate::usage::Usage;
use crate::value::{ModelId, Text};

use serde_json::Value;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(try_from = "RawStep")]
pub struct Step {
    content: Vec<AssistantBlock>,
    stop_reason: StopReason,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    usage: Option<Usage>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    model: Option<ModelId>,
}

#[derive(serde::Deserialize)]
struct RawStep {
    content: Vec<AssistantBlock>,
    stop_reason: StopReason,
    #[serde(default)]
    usage: Option<Usage>,
    #[serde(default)]
    model: Option<ModelId>,
}

impl TryFrom<RawStep> for Step {
    type Error = MessageError;

    fn try_from(raw: RawStep) -> Result<Self, Self::Error> {
        Step::new(raw.content, raw.stop_reason, raw.usage, raw.model)
    }
}

impl Step {
    pub fn new(
        content: Vec<AssistantBlock>,
        stop_reason: StopReason,
        usage: Option<Usage>,
        model: Option<ModelId>,
    ) -> Result<Self, MessageError> {
        Self::check(&content, &stop_reason)?;
        Ok(Self {
            content,
            stop_reason,
            usage,
            model,
        })
    }

    fn check(content: &[AssistantBlock], stop_reason: &StopReason) -> Result<(), MessageError> {
        if content.is_empty() {
            return Err(MessageError::EmptyStep);
        }

        let mut seen_tool_call = false;
        for block in content {
            if block.is_tool_call() {
                seen_tool_call = true;
            } else if seen_tool_call {
                return Err(MessageError::ToolCallNotAtTail);
            }
        }

        let has_tool_call = seen_tool_call;
        let awaiting = matches!(stop_reason, StopReason::AwaitingToolResults);
        match (awaiting, has_tool_call) {
            (true, false) => return Err(MessageError::AwaitingToolResultsWithoutCalls),
            (false, true) => return Err(MessageError::ToolCallsWithoutAwaiting),
            _ => {}
        }

        let mut seen = Vec::new();
        for call in content.iter().filter_map(AssistantBlock::as_tool_call) {
            if seen.contains(&&call.id) {
                return Err(MessageError::DuplicateToolCallId {
                    id: call.id.clone(),
                });
            }
            seen.push(&call.id);
        }

        Ok(())
    }

    pub fn content(&self) -> &[AssistantBlock] {
        &self.content
    }

    pub fn stop_reason(&self) -> &StopReason {
        &self.stop_reason
    }

    pub fn usage(&self) -> Option<&Usage> {
        self.usage.as_ref()
    }

    pub fn model(&self) -> Option<&ModelId> {
        self.model.as_ref()
    }

    pub fn text(&self) -> impl Iterator<Item = &Text> {
        self.content.iter().filter_map(AssistantBlock::as_text)
    }

    pub fn thinking(&self) -> impl Iterator<Item = &crate::block::Thinking> {
        self.content.iter().filter_map(AssistantBlock::as_thinking)
    }

    pub fn structured(&self) -> impl Iterator<Item = &Value> {
        self.content
            .iter()
            .filter_map(AssistantBlock::as_structured)
    }

    pub fn tool_calls(&self) -> impl Iterator<Item = &ToolCall> {
        self.content.iter().filter_map(AssistantBlock::as_tool_call)
    }
}

impl std::fmt::Display for Step {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "step[{}]", self.stop_reason)?;
        for block in &self.content {
            match block {
                AssistantBlock::Text { text } => write!(f, " text({text})")?,
                AssistantBlock::Thinking(_) => f.write_str(" thinking")?,
                AssistantBlock::RedactedThinking(_) => f.write_str(" redacted_thinking")?,
                AssistantBlock::Structured { .. } => f.write_str(" structured")?,
                AssistantBlock::ToolCall(call) => write!(f, " tool_call({})", call.name)?,
            }
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "step_tests.rs"]
mod tests;
