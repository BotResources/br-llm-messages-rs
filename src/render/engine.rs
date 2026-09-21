use crate::block::Image;
use crate::conversation::{Conversation, Entry};
use crate::error::MessageError;
use crate::turn::{Turn, TurnItem, TurnState};
use crate::value::{Author, Text};

use super::frame::{agent_frame_text, render_agent_frame, render_user_frame, user_frame_parts};
use super::wire::{Perspective, WireMessage, WireUserBlock};

pub fn render(
    conversation: &Conversation,
    perspective: &Perspective,
) -> Result<Vec<WireMessage>, MessageError> {
    let agent = &perspective.agent;
    guard_own_turns(conversation, agent)?;

    let entries = conversation.entries();
    let (head, tail): (&[Entry], &[Entry]) = match relay_boundary(conversation, agent) {
        Some(start) => (&entries[..start], &entries[start..]),
        None => (entries, &[]),
    };

    let mut messages = Vec::new();
    for entry in head {
        match entry {
            Entry::Turn(turn) if owns(turn, agent) => render_own_turn(turn, &mut messages),
            Entry::Turn(turn) => messages.push(render_agent_frame(turn)?),
            Entry::UserInput(input) => messages.push(render_user_frame(input)?),
        }
    }

    let mut merged = merge_user_messages(messages);
    if !tail.is_empty() {
        let relay = build_relay(tail, &mut merged)?;
        merged.push(relay);
    }

    match merged.first() {
        None => Err(MessageError::EmptyRender),
        Some(WireMessage::User { .. }) => Ok(merged),
        Some(WireMessage::Assistant { .. } | WireMessage::Relay { .. }) => {
            Err(MessageError::RenderStartsWithAssistant)
        }
    }
}

fn owns(turn: &Turn, agent: &Author) -> bool {
    turn.author() == Some(agent)
}

fn guard_own_turns(conversation: &Conversation, agent: &Author) -> Result<(), MessageError> {
    for entry in conversation.entries() {
        if let Entry::Turn(turn) = entry
            && owns(turn, agent)
            && matches!(turn.state(), TurnState::AwaitingToolResults { .. })
        {
            return Err(MessageError::TurnAwaitingResults);
        }
    }
    Ok(())
}

fn relay_boundary(conversation: &Conversation, agent: &Author) -> Option<usize> {
    let entries = conversation.entries();
    let last_own = entries
        .iter()
        .enumerate()
        .rev()
        .find_map(|(index, entry)| match entry {
            Entry::Turn(turn) if owns(turn, agent) => Some((index, turn)),
            Entry::Turn(_) | Entry::UserInput(_) => None,
        })?;
    let (index, turn) = last_own;
    if matches!(turn.state(), TurnState::Finished { .. }) || index + 1 == entries.len() {
        None
    } else {
        Some(index + 1)
    }
}

fn build_relay(tail: &[Entry], merged: &mut [WireMessage]) -> Result<WireMessage, MessageError> {
    let mut texts: Vec<Text> = Vec::new();
    let mut images: Vec<Image> = Vec::new();
    for entry in tail {
        let framed = match entry {
            Entry::UserInput(input) => {
                let (framed, mut input_images) = user_frame_parts(input);
                images.append(&mut input_images);
                framed
            }
            Entry::Turn(turn) => agent_frame_text(turn),
        };
        texts.push(Text::new(framed)?);
    }

    if !images.is_empty()
        && let Some(WireMessage::User { content }) = merged
            .iter_mut()
            .rev()
            .find(|message| matches!(message, WireMessage::User { .. }))
    {
        for image in images {
            content.push(WireUserBlock::Image(image));
        }
    }

    Ok(WireMessage::Relay { content: texts })
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
            other @ (WireMessage::Assistant { .. } | WireMessage::Relay { .. }) => {
                merged.push(other)
            }
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
