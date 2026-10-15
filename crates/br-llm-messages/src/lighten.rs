use crate::block::{AssistantBlock, Thinking};
use crate::conversation::{Conversation, Entry};
use crate::render::Perspective;
use crate::step::Step;
use crate::turn::{Turn, TurnItem};

/// Lightens a conversation for one perspective: the result renders for that
/// perspective exactly as the original does, or is refused the same way.
///
/// User inputs and the agent's own turns are kept as they are, ids included,
/// and the entries keep their order. Every other turn keeps its id and its
/// author and holds one finished step (`end_turn`) made of the texts and
/// structured values of its steps, in order, or of one empty thinking block
/// when it has none.
///
/// A lightened conversation is a view for one render: its steps were written
/// by no model, so it is never stored as the conversation. Store and grow the
/// original, and lighten it again for each render.
pub fn lighten(conversation: &Conversation, perspective: &Perspective) -> Conversation {
    let entries = conversation
        .entries()
        .iter()
        .map(|entry| match entry {
            Entry::Turn(turn) if !perspective.owns(turn) => Entry::Turn(lighten_turn(turn)),
            Entry::Turn(_) | Entry::UserInput(_) => entry.clone(),
        })
        .collect();
    Conversation::from_entries(entries)
}

fn lighten_turn(turn: &Turn) -> Turn {
    let mut kept: Vec<AssistantBlock> = Vec::new();
    for item in turn.items() {
        let TurnItem::Step(step) = item else {
            continue;
        };
        for block in step.content() {
            match block {
                AssistantBlock::Text { .. } | AssistantBlock::Structured { .. } => {
                    kept.push(block.clone())
                }
                AssistantBlock::Thinking(_)
                | AssistantBlock::RedactedThinking(_)
                | AssistantBlock::ToolCall(_) => {}
            }
        }
    }
    if kept.is_empty() {
        kept.push(AssistantBlock::Thinking(Thinking {
            text: String::new(),
            signature: None,
        }));
    }
    Turn::new(
        turn.id().clone(),
        turn.author().cloned(),
        Step::finished(kept),
    )
}

#[cfg(test)]
#[path = "lighten_test_helpers.rs"]
mod test_helpers;

#[cfg(test)]
#[path = "lighten_tests.rs"]
mod tests;
