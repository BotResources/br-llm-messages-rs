use std::fmt;
use std::sync::Arc;

use super::ProjectedThread;
use super::check::Checked;
use super::output::{Output, Owner, usable_block};
use super::pending::PendingResponse;
use super::source::{Foreign, Item, Source, Sources};
use crate::conversation::Conversation;
use crate::entry::{IncomingEntry, TurnEntry};
use crate::error::Error;
use crate::id::{AgentId, ConversationId};
use crate::turn::{AITurn, Class, Place, Turn, places_of_incoming, refuse_agent_output};

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

fn located(turn: usize, place: &Place) -> String {
    match place.class {
        Class::Turn => format!("turns[{turn}]"),
        _ => format!("turns[{turn}].entries[{}]", place.entry),
    }
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
        Checked::unique(self.sources(conversation.turns())).project(conversation.id(), self.agent)
    }

    pub fn project_turns(
        &self,
        conversation: ConversationId,
        turns: &[Turn],
    ) -> Result<ProjectedThread, Error> {
        self.project_pending(conversation, turns, &[])
    }

    pub fn project_pending(
        &self,
        conversation: ConversationId,
        turns: &[Turn],
        pending: &[PendingResponse<'_>],
    ) -> Result<ProjectedThread, Error> {
        for (position, turn) in turns.iter().enumerate() {
            if let Turn::InputTurn(input) = turn {
                refuse_agent_output(input.entries())
                    .map_err(|error| error.at(format!("turns[{position}]")))?;
            }
        }
        let checked = self.sources(turns).check(conversation, pending)?;
        Ok(checked.project(conversation, self.agent))
    }

    fn sources(&self, turns: &[Turn]) -> Sources {
        let mut sources = Sources::default();
        for (position, turn) in turns.iter().enumerate() {
            match turn {
                Turn::AITurn(ai) if ai.agent() != self.agent => {
                    let foreign = self.foreign(ai, position, &mut sources);
                    sources.push(Source::Foreign(foreign));
                }
                kept => {
                    for (id, place) in kept.declared() {
                        sources.declare(id, located(position, &place));
                    }
                    sources.push(Source::Kept(kept.clone()));
                }
            }
        }
        sources
    }

    fn foreign(&self, turn: &AITurn, position: usize, sources: &mut Sources) -> Foreign {
        sources.declare(turn.id().as_uuid(), format!("turns[{position}]"));
        let mut items = Vec::new();
        for (index, entry) in turn.entries().iter().enumerate() {
            let at = format!("turns[{position}].entries[{index}]");
            match entry {
                TurnEntry::ModelResponse(response) => {
                    sources.declare(response.id().as_uuid(), at.clone());
                    for block in response.blocks() {
                        if usable_block(block).is_some() {
                            sources.declare(block.id().as_uuid(), at.clone());
                        }
                    }
                    items.push(Item::Output(Output::of_response(response)));
                }
                recorded => {
                    let Some(incoming) = IncomingEntry::recorded(recorded) else {
                        continue;
                    };
                    if !self.keeps(&incoming) {
                        continue;
                    }
                    for (id, _) in places_of_incoming(&incoming, index) {
                        sources.declare(id, at.clone());
                    }
                    items.push(Item::Incoming(incoming));
                }
            }
        }
        Foreign {
            id: turn.id(),
            owner: Owner {
                agent: turn.agent(),
                author: turn.author().clone(),
            },
            items,
        }
    }
}
