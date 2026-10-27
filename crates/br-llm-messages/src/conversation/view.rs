use super::Conversation;
use crate::entry::{IncomingEntry, TurnEntry, TurnEntryKind};
use crate::id::{AgentId, EntryId, TurnId};
use crate::projection::{Perspective, ProjectedThread};
use crate::turn::Turn;

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub enum EntryView<'a> {
    Incoming(&'a IncomingEntry),
    Turn(&'a TurnEntry),
}

impl EntryView<'_> {
    pub fn id(&self) -> EntryId {
        match self {
            EntryView::Incoming(entry) => entry.id(),
            EntryView::Turn(entry) => entry.id(),
        }
    }

    pub fn kind(&self) -> Option<TurnEntryKind> {
        match self {
            EntryView::Incoming(entry) => entry.kind(),
            EntryView::Turn(entry) => Some(entry.kind()),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlacedEntry<'a> {
    turn: TurnId,
    entry: EntryView<'a>,
}

impl<'a> PlacedEntry<'a> {
    pub fn turn(&self) -> TurnId {
        self.turn
    }

    pub fn entry(&self) -> &EntryView<'a> {
        &self.entry
    }

    pub fn into_entry(self) -> EntryView<'a> {
        self.entry
    }
}

pub(crate) fn placed_entries(turns: &[Turn]) -> impl Iterator<Item = PlacedEntry<'_>> {
    turns.iter().flat_map(|turn| {
        let id = turn.id();
        let entries: Box<dyn Iterator<Item = EntryView<'_>> + '_> = match turn {
            Turn::InputTurn(input) => Box::new(input.entries().iter().map(EntryView::Incoming)),
            Turn::AITurn(ai) => Box::new(ai.entries().iter().map(EntryView::Turn)),
        };
        entries.map(move |entry| PlacedEntry { turn: id, entry })
    })
}

impl Conversation {
    pub fn entries(&self) -> impl Iterator<Item = PlacedEntry<'_>> {
        placed_entries(&self.turns)
    }

    pub fn project_for(&self, agent: AgentId) -> ProjectedThread {
        Perspective::of(agent).project(self)
    }
}
