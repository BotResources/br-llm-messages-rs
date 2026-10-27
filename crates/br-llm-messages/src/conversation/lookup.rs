use uuid::Uuid;

use super::Conversation;
use super::index::{Index, Slot};
use super::view::EntryView;
use crate::block::{
    ContentBlock, ContentBlockKind, PlainBlock, RuntimeToolCall, RuntimeToolResult,
};
use crate::entry::{IncomingEntry, TurnEntry, TurnEntryKind};
use crate::error::{Error, ErrorCode};
use crate::id::{BlockId, EntryId, TurnId};
use crate::turn::{Body, Class, Turn, body_of_entry, body_of_incoming};

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub enum EntryRef<'a> {
    #[non_exhaustive]
    Incoming {
        entry: &'a IncomingEntry,
        kind: TurnEntryKind,
    },
    Turn(&'a TurnEntry),
}

impl EntryRef<'_> {
    pub fn id(&self) -> EntryId {
        match self {
            EntryRef::Incoming { entry, .. } => entry.id(),
            EntryRef::Turn(entry) => entry.id(),
        }
    }

    pub fn kind(&self) -> TurnEntryKind {
        match self {
            EntryRef::Incoming { kind, .. } => *kind,
            EntryRef::Turn(entry) => entry.kind(),
        }
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub enum BlockRef<'a> {
    Plain(&'a PlainBlock),
    Content(&'a ContentBlock),
    RuntimeResult(&'a RuntimeToolResult),
}

impl BlockRef<'_> {
    pub fn id(&self) -> BlockId {
        match self {
            BlockRef::Plain(block) => block.id(),
            BlockRef::Content(block) => block.id(),
            BlockRef::RuntimeResult(block) => block.id(),
        }
    }

    pub fn kind(&self) -> ContentBlockKind {
        match self {
            BlockRef::Plain(block) => block.kind(),
            BlockRef::Content(block) => block.kind(),
            BlockRef::RuntimeResult(_) => ContentBlockKind::RuntimeToolResult,
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct Lookup<'a> {
    turns: &'a [Turn],
    index: &'a Index,
}

impl<'a> Lookup<'a> {
    pub(crate) fn new(turns: &'a [Turn], index: &'a Index) -> Self {
        Lookup { turns, index }
    }

    fn resolve(&self, id: Uuid, accepts: impl Fn(Class) -> bool) -> Result<&'a Slot, Error> {
        match self.index.get(id) {
            Some(slot) if accepts(slot.place.class) => Ok(slot),
            _ => Err(Error::new(ErrorCode::UnknownReference).with_id(id)),
        }
    }

    fn turn_at(&self, slot: &Slot, id: Uuid) -> Result<&'a Turn, Error> {
        self.turns
            .get(slot.turn)
            .ok_or_else(|| Error::new(ErrorCode::UnknownReference).with_id(id))
    }

    pub(crate) fn turn(&self, id: &TurnId) -> Result<&'a Turn, Error> {
        let slot = self.resolve(id.as_uuid(), |class| class == Class::Turn)?;
        self.turn_at(slot, id.as_uuid())
    }

    pub(crate) fn entry(&self, id: &EntryId) -> Result<EntryView<'a>, Error> {
        let slot = self.resolve(id.as_uuid(), |class| {
            matches!(class, Class::Entry(_) | Class::Output)
        })?;
        let found = match self.turn_at(slot, id.as_uuid())? {
            Turn::InputTurn(turn) => turn
                .entries()
                .get(slot.place.entry)
                .map(EntryView::Incoming),
            Turn::AITurn(turn) => turn.entries().get(slot.place.entry).map(EntryView::Turn),
        };
        found.ok_or_else(|| Error::new(ErrorCode::UnknownReference).with_id(id.as_uuid()))
    }

    pub(crate) fn block(&self, id: &BlockId) -> Result<BlockRef<'a>, Error> {
        let slot = self.resolve(id.as_uuid(), |class| matches!(class, Class::Block(_)))?;
        let body = match self.turn_at(slot, id.as_uuid())? {
            Turn::InputTurn(turn) => turn.entries().get(slot.place.entry).map(body_of_incoming),
            Turn::AITurn(turn) => turn.entries().get(slot.place.entry).map(body_of_entry),
        };
        let (root, child) = (slot.place.root, slot.place.child);
        let found = match (body, child) {
            (Some(Body::Content(blocks)), None) => blocks.get(root).map(BlockRef::Plain),
            (Some(Body::Response(blocks)), None) => blocks.get(root).map(BlockRef::Content),
            (Some(Body::Response(blocks)), Some(child)) => blocks
                .get(root)
                .and_then(|block| block.children().get(child))
                .map(BlockRef::Plain),
            (Some(Body::Results(results)), None) => results.get(root).map(BlockRef::RuntimeResult),
            (Some(Body::Results(results)), Some(child)) => results
                .get(root)
                .and_then(|result| result.content().get(child))
                .map(BlockRef::Plain),
            _ => None,
        };
        found.ok_or_else(|| Error::new(ErrorCode::UnknownReference).with_id(id.as_uuid()))
    }
}

impl Conversation {
    fn lookup(&self) -> Lookup<'_> {
        Lookup::new(&self.turns, &self.index)
    }

    pub fn turn(&self, id: &TurnId) -> Result<&Turn, Error> {
        self.lookup().turn(id)
    }

    pub fn entry(&self, id: &EntryId) -> Result<EntryRef<'_>, Error> {
        match self.lookup().entry(id)? {
            EntryView::Incoming(entry) => match entry.kind() {
                Some(kind) => Ok(EntryRef::Incoming { entry, kind }),
                None => Err(Error::new(ErrorCode::UnknownReference).with_id(id.as_uuid())),
            },
            EntryView::Turn(entry) => Ok(EntryRef::Turn(entry)),
        }
    }

    pub fn block(&self, id: &BlockId) -> Result<BlockRef<'_>, Error> {
        self.lookup().block(id)
    }

    pub fn runtime_call(&self, id: &BlockId) -> Result<&RuntimeToolCall, Error> {
        match self.block(id)? {
            BlockRef::Content(ContentBlock::RuntimeToolCall(call)) => Ok(call),
            _ => Err(Error::new(ErrorCode::UnknownReference).with_id(id.as_uuid())),
        }
    }
}
