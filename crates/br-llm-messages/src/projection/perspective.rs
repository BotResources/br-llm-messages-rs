use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use super::ProjectedThread;
use super::output::output_of;
use super::pending::{Owner, PendingResponse, pending_entries};
use crate::block::{ContentBlock, PlainBlock, TextBlock};
use crate::conversation::Conversation;
use crate::entry::{IncomingEntry, TurnEntry};
use crate::error::Error;
use crate::id::{AgentId, BlockId, ConversationId, TurnId};
use crate::stream::ResponseDraft;
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
        let turns = self
            .translate(conversation.turns(), &HashMap::new())
            .expect("the identities of a conversation are unique, so a translation cannot clash");
        ProjectedThread::unchecked(conversation.id(), self.agent, turns)
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
        let pending: Vec<PendingResponse<'_>> = drafts.iter().map(PendingResponse::Draft).collect();
        self.project_pending(conversation, turns, &pending)
    }

    pub fn project_pending(
        &self,
        conversation: ConversationId,
        turns: &[Turn],
        pending: &[PendingResponse<'_>],
    ) -> Result<ProjectedThread, Error> {
        let owners = turns.iter().filter_map(|turn| match turn {
            Turn::AITurn(ai) => Some((ai.id(), (ai.agent(), ai.author().clone()))),
            Turn::InputTurn(_) => None,
        });
        let declared = turns
            .iter()
            .flat_map(|turn| turn.declared().into_iter().map(|(id, _)| id));
        let pending = pending_entries(self.agent, conversation, pending, owners, declared)?;
        let translated = self.translate(turns, &pending)?;
        ProjectedThread::assemble(conversation, self.agent, translated)
    }

    fn translated(
        &self,
        turn: &AITurn,
        pending: &HashMap<TurnId, Vec<IncomingEntry>>,
    ) -> Result<Option<Turn>, Error> {
        let owner: Owner = (turn.agent(), turn.author().clone());
        let mut entries: Vec<IncomingEntry> = turn
            .entries()
            .iter()
            .filter_map(|entry| match entry {
                TurnEntry::ModelResponse(response) => output_of(&owner, response),
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
    ) -> Result<Vec<Turn>, Error> {
        let mut translated = Vec::with_capacity(turns.len());
        for (position, turn) in turns.iter().enumerate() {
            match turn {
                Turn::AITurn(ai) if ai.agent() != self.agent => {
                    let built = self
                        .translated(ai, pending)
                        .map_err(|error| error.at(format!("turns[{position}]")))?;
                    translated.extend(built);
                }
                other => translated.push(other.clone()),
            }
        }
        Ok(translated)
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

pub(super) fn blocks_of(blocks: &[ContentBlock]) -> Vec<PlainBlock> {
    blocks.iter().filter_map(usable_block).collect()
}

pub(crate) fn translated_turn(
    id: TurnId,
    entries: Vec<IncomingEntry>,
) -> Result<Option<Turn>, Error> {
    if entries.is_empty() {
        return Ok(None);
    }
    InputTurn::projected(id, entries).map(|turn| Some(Turn::InputTurn(turn)))
}
