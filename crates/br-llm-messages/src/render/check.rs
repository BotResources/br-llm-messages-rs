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
/// index of the block in that message); the messages are scanned in order and
/// the first fault found is reported.
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
                    check_answered_once(messages.get(index + 1), index, block, call)?;
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

/// The call at (`index`, `block`) is answered exactly once, by id, in `next`.
fn check_answered_once(
    next: Option<&WireMessage>,
    index: usize,
    block: usize,
    call: &ToolCall,
) -> Result<(), MessageError> {
    let answers: Vec<usize> = match next {
        Some(WireMessage::User { content }) => content
            .iter()
            .enumerate()
            .filter(|(_, item)| answers_call(item, &call.id))
            .map(|(position, _)| position)
            .collect(),
        Some(WireMessage::Assistant { .. } | WireMessage::Relay { .. }) | None => Vec::new(),
    };
    match answers.as_slice() {
        [] => Err(MessageError::WireToolCallUnanswered {
            message: index,
            block,
            id: call.id.clone(),
        }),
        [_] => Ok(()),
        [_, second, ..] => Err(MessageError::WireToolCallAnsweredTwice {
            message: index + 1,
            block: *second,
            id: call.id.clone(),
        }),
    }
}

/// Every tool result of the `User` message at `index` answers a tool call of
/// `previous` when it is an `Assistant` message.
fn check_results_answer(
    previous: Option<&WireMessage>,
    index: usize,
    content: &[WireUserBlock],
) -> Result<(), MessageError> {
    let calls: &[AssistantBlock] = match previous {
        Some(WireMessage::Assistant { content }) => content,
        Some(WireMessage::User { .. } | WireMessage::Relay { .. }) | None => &[],
    };
    for (block, item) in content.iter().enumerate() {
        let WireUserBlock::ToolResult(result) = item else {
            continue;
        };
        let answers = calls
            .iter()
            .filter_map(AssistantBlock::as_tool_call)
            .any(|call| call.id == result.tool_call_id);
        if !answers {
            return Err(MessageError::WireToolResultUnmatched {
                message: index,
                block,
                id: result.tool_call_id.clone(),
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
