use std::collections::HashMap;

use super::ProjectedThread;
use crate::block::{ContentBlock, ContentBlockKind, TextBlock};
use crate::conversation::Conversation;
use crate::entry::{AgentOutput, IncomingEntry, ModelResponse, PartialAgentOutput, TurnEntry};
use crate::error::{Error, ErrorCode};
use crate::id::{AgentId, BlockId, ConversationId, TurnId};
use crate::stream::{DraftPhase, ResponseDraft};
use crate::turn::{AITurn, InputTurn, Turn};
use crate::value::Author;

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub struct Perspective {
    agent: AgentId,
}

impl Perspective {
    pub fn of(agent: AgentId) -> Self {
        Perspective { agent }
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

    fn translate(
        &self,
        turns: &[Turn],
        pending: &HashMap<TurnId, Vec<IncomingEntry>>,
    ) -> Vec<Turn> {
        turns
            .iter()
            .filter_map(|turn| match turn {
                Turn::AITurn(ai) if ai.agent() != self.agent => translated(ai, pending),
                other => Some(other.clone()),
            })
            .collect()
    }
}

pub(crate) fn accumulated_text(id: BlockId, text: &str) -> Option<TextBlock> {
    TextBlock::new(id, text).ok()
}

fn texts_of(blocks: &[ContentBlock]) -> Vec<TextBlock> {
    blocks
        .iter()
        .filter_map(|block| match block {
            ContentBlock::Text(text) => Some(text.text_only()),
            _ => None,
        })
        .collect()
}

fn output_of(turn: &AITurn, response: &ModelResponse) -> Option<IncomingEntry> {
    AgentOutput::from_texts(
        response.id(),
        turn.agent(),
        turn.author().clone(),
        texts_of(response.blocks()),
    )
    .ok()
    .map(IncomingEntry::AgentOutput)
}

fn draft_entry(draft: &ResponseDraft, turn: &AITurn) -> Option<IncomingEntry> {
    match draft.phase() {
        DraftPhase::Completed { .. } => draft
            .response()
            .and_then(|response| output_of(turn, response)),
        DraftPhase::Producing { .. } => {
            let texts: Vec<TextBlock> = draft
                .view()
                .slots()
                .iter()
                .filter(|slot| slot.kind() == ContentBlockKind::Text)
                .filter_map(|slot| accumulated_text(slot.block(), slot.text()?))
                .collect();
            let author: Author = turn.author().clone();
            PartialAgentOutput::from_texts(draft.entry(), turn.agent(), author, texts)
                .ok()
                .map(IncomingEntry::PartialAgentOutput)
        }
        _ => None,
    }
}

fn translated(turn: &AITurn, pending: &HashMap<TurnId, Vec<IncomingEntry>>) -> Option<Turn> {
    let mut entries: Vec<IncomingEntry> = turn
        .entries()
        .iter()
        .filter_map(|entry| match entry {
            TurnEntry::ModelResponse(response) => output_of(turn, response),
            _ => None,
        })
        .collect();
    entries.extend(pending.get(&turn.id()).cloned().unwrap_or_default());
    InputTurn::projected(turn.id(), entries)
        .ok()
        .map(Turn::InputTurn)
}
