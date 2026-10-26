use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use super::ProjectedThread;
use crate::block::{ContentBlock, ContentBlockKind, PlainBlock, TextBlock};
use crate::conversation::Conversation;
use crate::entry::{AgentOutput, IncomingEntry, ModelResponse, PartialAgentOutput, TurnEntry};
use crate::error::{Error, ErrorCode};
use crate::id::{AgentId, BlockId, ConversationId, TurnId};
use crate::stream::{DraftPhase, ResponseDraft};
use crate::turn::{AITurn, InputTurn, Turn};

#[derive(Clone)]
struct Selection(Arc<dyn Fn(&IncomingEntry) -> bool + Send + Sync>);

impl fmt::Debug for Selection {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Selection")
    }
}

impl PartialEq for Selection {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub struct Perspective {
    agent: AgentId,
    selection: Option<Selection>,
}

impl Perspective {
    pub fn of(agent: AgentId) -> Self {
        Perspective {
            agent,
            selection: None,
        }
    }

    pub fn keeping(
        mut self,
        selection: impl Fn(&IncomingEntry) -> bool + Send + Sync + 'static,
    ) -> Self {
        self.selection = Some(Selection(Arc::new(selection)));
        self
    }

    fn keeps(&self, entry: &IncomingEntry) -> bool {
        self.selection
            .as_ref()
            .is_none_or(|selection| (selection.0)(entry))
    }

    pub fn agent(&self) -> AgentId {
        self.agent
    }

    pub fn project(&self, conversation: &Conversation) -> ProjectedThread {
        ProjectedThread::unchecked(
            conversation.id(),
            self.agent,
            self.translate(conversation.turns(), &HashMap::new()),
        )
    }

    pub fn project_turns(
        &self,
        conversation: ConversationId,
        turns: &[Turn],
    ) -> Result<ProjectedThread, Error> {
        self.project_live(conversation, turns, &[])
    }

    pub fn project_live(
        &self,
        conversation: ConversationId,
        turns: &[Turn],
        drafts: &[ResponseDraft],
    ) -> Result<ProjectedThread, Error> {
        let pending = self.pending(turns, drafts)?;
        ProjectedThread::assemble(conversation, self.agent, self.translate(turns, &pending))
    }

    fn pending(
        &self,
        turns: &[Turn],
        drafts: &[ResponseDraft],
    ) -> Result<HashMap<TurnId, Vec<IncomingEntry>>, Error> {
        let mut pending: HashMap<TurnId, Vec<IncomingEntry>> = HashMap::new();
        for (index, draft) in drafts.iter().enumerate() {
            let located = |code| {
                Error::new(code)
                    .with_id(draft.turn().as_uuid())
                    .at(format!("drafts[{index}]"))
            };
            let owner = turns.iter().find_map(|turn| match turn {
                Turn::AITurn(ai) if ai.id() == draft.turn() => Some(ai),
                _ => None,
            });
            let Some(owner) = owner else {
                return Err(located(ErrorCode::UnknownReference));
            };
            if owner.agent() == self.agent {
                return Err(located(ErrorCode::DisallowedKind));
            }
            if let Some(entry) = draft_entry(draft, owner) {
                pending.entry(owner.id()).or_default().push(entry);
            }
        }
        Ok(pending)
    }

    fn translated(
        &self,
        turn: &AITurn,
        pending: &HashMap<TurnId, Vec<IncomingEntry>>,
    ) -> Option<Turn> {
        let mut entries: Vec<IncomingEntry> = turn
            .entries()
            .iter()
            .filter_map(|entry| match entry {
                TurnEntry::ModelResponse(response) => output_of(turn, response),
                other => IncomingEntry::try_from(other)
                    .ok()
                    .filter(|incoming| self.keeps(incoming)),
            })
            .collect();
        entries.extend(pending.get(&turn.id()).cloned().unwrap_or_default());
        translated_turn(turn.id(), entries)
    }

    fn translate(
        &self,
        turns: &[Turn],
        pending: &HashMap<TurnId, Vec<IncomingEntry>>,
    ) -> Vec<Turn> {
        turns
            .iter()
            .filter_map(|turn| match turn {
                Turn::AITurn(ai) if ai.agent() != self.agent => self.translated(ai, pending),
                other => Some(other.clone()),
            })
            .collect()
    }
}

pub(crate) fn accumulated_text(id: BlockId, text: &str) -> Option<TextBlock> {
    TextBlock::new(id, text).ok()
}

pub(crate) fn usable_block(block: &ContentBlock) -> Option<PlainBlock> {
    let plain = match block {
        ContentBlock::Text(text) => PlainBlock::Text(text.clone()),
        ContentBlock::Image(image) => PlainBlock::Image(image.clone()),
        ContentBlock::StructuredContent(structured) => {
            PlainBlock::StructuredContent(structured.clone())
        }
        _ => return None,
    };
    Some(plain.bare())
}

fn blocks_of(blocks: &[ContentBlock]) -> Vec<PlainBlock> {
    blocks.iter().filter_map(usable_block).collect()
}

fn built<T>(
    blocks: Vec<PlainBlock>,
    build: impl FnOnce(Vec<PlainBlock>) -> Result<T, Error>,
) -> Option<T> {
    if blocks.is_empty() {
        return None;
    }
    build(blocks).ok()
}

fn output_of(turn: &AITurn, response: &ModelResponse) -> Option<IncomingEntry> {
    built(blocks_of(response.blocks()), |blocks| {
        AgentOutput::from_content(response.id(), turn.agent(), turn.author().clone(), blocks)
    })
    .map(IncomingEntry::AgentOutput)
}

fn producing_blocks(draft: &ResponseDraft) -> Vec<PlainBlock> {
    draft
        .view()
        .slots()
        .iter()
        .filter_map(|slot| match slot.finalized() {
            Some(block) => usable_block(block),
            None if slot.kind() == ContentBlockKind::Text => {
                accumulated_text(slot.block(), slot.text()?).map(PlainBlock::Text)
            }
            None => None,
        })
        .collect()
}

fn draft_entry(draft: &ResponseDraft, turn: &AITurn) -> Option<IncomingEntry> {
    match draft.phase() {
        DraftPhase::Completed { .. } => draft
            .response()
            .and_then(|response| output_of(turn, response)),
        DraftPhase::Producing { .. } => built(producing_blocks(draft), |blocks| {
            PartialAgentOutput::from_content(
                draft.entry(),
                turn.agent(),
                turn.author().clone(),
                blocks,
            )
        })
        .map(IncomingEntry::PartialAgentOutput),
        _ => None,
    }
}

pub(crate) fn translated_turn(id: TurnId, entries: Vec<IncomingEntry>) -> Option<Turn> {
    if entries.is_empty() {
        return None;
    }
    InputTurn::projected(id, entries).ok().map(Turn::InputTurn)
}
