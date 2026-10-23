use uuid::Uuid;

use super::Conversation;
use super::index::Slot;
use crate::block::{
    ContentBlock, ContentBlockKind, PlainBlock, RuntimeToolCall, RuntimeToolResult,
};
use crate::entry::{IncomingEntry, TurnEntry, TurnEntryKind};
use crate::error::{Error, ErrorCode};
use crate::id::{BlockId, EntryId, TurnId};
use crate::turn::{Body, Class, Turn, body_of_entry, body_of_incoming};

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryRef<'a> {
    Incoming(&'a IncomingEntry),
    Turn(&'a TurnEntry),
}

impl EntryRef<'_> {
    pub fn id(&self) -> EntryId {
        match self {
            EntryRef::Incoming(entry) => entry.id(),
            EntryRef::Turn(entry) => entry.id(),
        }
    }

    pub fn kind(&self) -> TurnEntryKind {
        match self {
            EntryRef::Incoming(entry) => entry.kind(),
            EntryRef::Turn(entry) => entry.kind(),
        }
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

impl Conversation {
    fn resolve(&self, id: Uuid, accepts: impl Fn(Class) -> bool) -> Result<&Slot, Error> {
        match self.index.get(id) {
            None => Err(Error::new(ErrorCode::UnknownReference).with_id(id)),
            Some(slot) if accepts(slot.place.class) => Ok(slot),
            Some(_) => Err(Error::new(ErrorCode::WrongKindReference).with_id(id)),
        }
    }

    pub fn turn(&self, id: &TurnId) -> Result<&Turn, Error> {
        let slot = self.resolve(id.as_uuid(), |class| class == Class::Turn)?;
        Ok(&self.turns[slot.turn])
    }

    pub fn entry(&self, id: &EntryId) -> Result<EntryRef<'_>, Error> {
        let slot = self.resolve(id.as_uuid(), |class| matches!(class, Class::Entry(_)))?;
        let found = match &self.turns[slot.turn] {
            Turn::InputTurn(turn) => Some(EntryRef::Incoming(turn.entry())),
            Turn::AITurn(turn) => turn.entries().get(slot.place.entry).map(EntryRef::Turn),
        };
        found.ok_or_else(|| Error::new(ErrorCode::UnknownReference).with_id(id.as_uuid()))
    }

    pub fn block(&self, id: &BlockId) -> Result<BlockRef<'_>, Error> {
        let slot = self.resolve(id.as_uuid(), |class| matches!(class, Class::Block(_)))?;
        let body = match &self.turns[slot.turn] {
            Turn::InputTurn(turn) => Some(body_of_incoming(turn.entry())),
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

    pub fn runtime_call(&self, id: &BlockId) -> Result<&RuntimeToolCall, Error> {
        match self.block(id)? {
            BlockRef::Content(ContentBlock::RuntimeToolCall(call)) => Ok(call),
            _ => Err(Error::new(ErrorCode::WrongKindReference).with_id(id.as_uuid())),
        }
    }
}
