use std::collections::HashSet;

use crate::block::{AssistantBlock, ToolCall};
use crate::error::MessageError;
use crate::value::ToolCallId;

use super::wire::{WireMessage, WireUserBlock};

/// Checks a list of wire messages from anywhere, by choice: loading a list
/// from JSON checks no rule of the list, and a list yielded by `render`
/// always passes.
///
/// The list is refused when it is empty, when it does not start with a `User`
/// message, when a tool call of an `Assistant` message is not answered
/// exactly once, by id, in the next `User` message, when a tool result does
/// not answer a tool call of the `Assistant` message just before it, or when
/// a tool call id appears more than once. The refusal names the rule and the
/// position (the index of the message and, for a rule about a block, the
/// index of the block in that message). The first fault in message order,
/// then in block order, is the one reported: a call left unanswered is a
/// fault of its `Assistant` message, a result that answers no call or answers
/// one twice is a fault of its `User` message.
pub fn check_wire_list(messages: &[WireMessage]) -> Result<(), MessageError> {
    match messages.first() {
        None => return Err(MessageError::WireListEmpty),
        Some(WireMessage::User { .. }) => {}
        Some(WireMessage::Assistant { .. } | WireMessage::Relay { .. }) => {
            return Err(MessageError::WireListNotStartingWithUser { message: 0 });
        }
    }

    let mut used_ids: HashSet<&ToolCallId> = HashSet::new();
    for (index, message) in messages.iter().enumerate() {
        match message {
            WireMessage::Assistant { content } => {
                let calls = content
                    .iter()
                    .enumerate()
                    .filter_map(|(block, item)| item.as_tool_call().map(|call| (block, call)));
                for (block, call) in calls {
                    if !used_ids.insert(&call.id) {
                        return Err(MessageError::WireToolCallIdRepeated {
                            message: index,
                            block,
                            id: call.id.clone(),
                        });
                    }
                    check_answered(messages.get(index + 1), index, block, call)?;
                }
            }
            WireMessage::User { content } => {
                let previous = index.checked_sub(1).and_then(|before| messages.get(before));
                check_results_answer(previous, index, content)?;
            }
            WireMessage::Relay { .. } => {}
        }
    }
    Ok(())
}

/// The call at (`index`, `block`) is answered, by id, in `next`. A second
/// answer is a fault of `next`, reported when `next` is checked.
fn check_answered(
    next: Option<&WireMessage>,
    index: usize,
    block: usize,
    call: &ToolCall,
) -> Result<(), MessageError> {
    let answered = match next {
        Some(WireMessage::User { content }) => {
            content.iter().any(|item| answers_call(item, &call.id))
        }
        Some(WireMessage::Assistant { .. } | WireMessage::Relay { .. }) | None => false,
    };
    if answered {
        Ok(())
    } else {
        Err(MessageError::WireToolCallUnanswered {
            message: index,
            block,
            id: call.id.clone(),
        })
    }
}

/// Every tool result of the `User` message at `index` answers a tool call of
/// `previous` when it is an `Assistant` message, and answers it once.
fn check_results_answer(
    previous: Option<&WireMessage>,
    index: usize,
    content: &[WireUserBlock],
) -> Result<(), MessageError> {
    let calls: &[AssistantBlock] = match previous {
        Some(WireMessage::Assistant { content }) => content,
        Some(WireMessage::User { .. } | WireMessage::Relay { .. }) | None => &[],
    };
    let mut answered: HashSet<&ToolCallId> = HashSet::new();
    for (block, item) in content.iter().enumerate() {
        let WireUserBlock::ToolResult(result) = item else {
            continue;
        };
        let id = &result.tool_call_id;
        let answers = calls
            .iter()
            .filter_map(AssistantBlock::as_tool_call)
            .any(|call| &call.id == id);
        if !answers {
            return Err(MessageError::WireToolResultUnmatched {
                message: index,
                block,
                id: id.clone(),
            });
        }
        if !answered.insert(id) {
            return Err(MessageError::WireToolCallAnsweredTwice {
                message: index,
                block,
                id: id.clone(),
            });
        }
    }
    Ok(())
}

fn answers_call(block: &WireUserBlock, id: &ToolCallId) -> bool {
    match block {
        WireUserBlock::ToolResult(result) => &result.tool_call_id == id,
        WireUserBlock::Text { .. } | WireUserBlock::Image(_) => false,
    }
}

#[cfg(test)]
#[path = "check_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "check_render_tests.rs"]
mod render_tests;
