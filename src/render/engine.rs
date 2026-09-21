use crate::conversation::{Conversation, Entry};
use crate::error::MessageError;
use crate::turn::{Turn, TurnItem, TurnState};
use crate::value::Author;

use super::frame::{render_agent_frame, render_user_frame};
use super::wire::{Perspective, WireMessage, WireUserBlock};

pub fn render(
    conversation: &Conversation,
    perspective: &Perspective,
) -> Result<Vec<WireMessage>, MessageError> {
    guard_last_own_turn(conversation, &perspective.agent)?;

    let mut messages = Vec::new();
    for entry in conversation.entries() {
        match entry {
            Entry::Turn(turn) if owns(turn, &perspective.agent) => {
                render_own_turn(turn, &mut messages);
            }
            Entry::Turn(turn) => messages.push(render_agent_frame(turn)?),
            Entry::UserInput(input) => messages.push(render_user_frame(input)?),
        }
    }

    let merged = merge_user_messages(messages);
    match merged.first() {
        None => Err(MessageError::EmptyRender),
        Some(WireMessage::Assistant { .. }) => Err(MessageError::RenderStartsWithAssistant),
        Some(WireMessage::User { .. }) => Ok(merged),
    }
}

fn owns(turn: &Turn, agent: &Author) -> bool {
    turn.author() == Some(agent)
}

fn guard_last_own_turn(conversation: &Conversation, agent: &Author) -> Result<(), MessageError> {
    let last_own = conversation
        .entries()
        .iter()
        .rev()
        .find_map(|entry| match entry {
            Entry::Turn(turn) if owns(turn, agent) => Some(turn),
            _ => None,
        });
    match last_own.map(Turn::state) {
        Some(TurnState::AwaitingToolResults { .. }) => Err(MessageError::TurnAwaitingResults),
        _ => Ok(()),
    }
}

fn render_own_turn(turn: &Turn, messages: &mut Vec<WireMessage>) {
    for item in turn.items() {
        match item {
            TurnItem::Step(step) => messages.push(WireMessage::Assistant {
                content: step.content().to_vec(),
            }),
            TurnItem::ToolResults(results) => messages.push(WireMessage::User {
                content: results
                    .results()
                    .iter()
                    .cloned()
                    .map(WireUserBlock::ToolResult)
                    .collect(),
            }),
        }
    }
}

fn merge_user_messages(messages: Vec<WireMessage>) -> Vec<WireMessage> {
    let mut merged: Vec<WireMessage> = Vec::new();
    for message in messages {
        match message {
            WireMessage::User {
                content: mut incoming,
            } => match merged.last_mut() {
                Some(WireMessage::User { content }) => content.append(&mut incoming),
                _ => merged.push(WireMessage::User { content: incoming }),
            },
            assistant => merged.push(assistant),
        }
    }

    for message in &mut merged {
        if let WireMessage::User { content } = message {
            let (results, rest): (Vec<_>, Vec<_>) = std::mem::take(content)
                .into_iter()
                .partition(|block| matches!(block, WireUserBlock::ToolResult(_)));
            content.extend(results);
            content.extend(rest);
        }
    }

    merged
}

#[cfg(test)]
#[path = "engine_tests.rs"]
mod tests;
