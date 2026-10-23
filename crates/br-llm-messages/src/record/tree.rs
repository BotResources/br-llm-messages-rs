use std::collections::HashMap;

use serde_json::Value;
use uuid::Uuid;

use super::block::BlockRecord;
use super::kinds::RepresentationState;
use super::payload::block_value;
use crate::block::ContentBlockKind;
use crate::error::{Error, ErrorCode};
use crate::id::{BlockId, EntryId};

pub(crate) fn order<T>(
    mut rows: Vec<T>,
    position: impl Fn(&T) -> u32,
    id: impl Fn(&T) -> Uuid,
    contiguous: bool,
) -> Result<Vec<T>, Error> {
    rows.sort_by_key(|row| position(row));
    let mut previous = None;
    for (index, row) in rows.iter().enumerate() {
        let invalid = || Error::new(ErrorCode::InvalidPosition).with_id(id(row));
        let at = position(row);
        if previous == Some(at) {
            return Err(invalid());
        }
        if contiguous && usize::try_from(at) != Ok(index) {
            return Err(invalid());
        }
        previous = Some(at);
    }
    Ok(rows)
}

fn is_container(kind: ContentBlockKind) -> bool {
    matches!(
        kind,
        ContentBlockKind::RuntimeToolResult
            | ContentBlockKind::ProviderToolResult
            | ContentBlockKind::ProviderToolExecutionRecord
    )
}

pub(crate) struct BlockTree<'a> {
    roots: HashMap<EntryId, Vec<&'a BlockRecord>>,
    children: HashMap<BlockId, Vec<&'a BlockRecord>>,
}

impl<'a> BlockTree<'a> {
    pub(crate) fn build(blocks: &'a [BlockRecord]) -> Result<Self, Error> {
        let mut rows = HashMap::with_capacity(blocks.len());
        for row in blocks {
            if rows.insert(row.id(), row).is_some() {
                return Err(Error::new(ErrorCode::ReusedIdentity).with_id(row.id().as_uuid()));
            }
        }
        check_references(blocks, &rows)?;
        check_containment(blocks, &rows)?;
        let mut roots: HashMap<EntryId, Vec<&BlockRecord>> = HashMap::new();
        let mut children: HashMap<BlockId, Vec<&BlockRecord>> = HashMap::new();
        for row in blocks {
            match row.parent() {
                None => roots.entry(row.entry()).or_default().push(row),
                Some(parent) => children.entry(parent).or_default().push(row),
            }
        }
        let by = |row: &&BlockRecord| row.position();
        let identity = |row: &&BlockRecord| row.id().as_uuid();
        let roots = roots
            .into_iter()
            .map(|(entry, group)| Ok((entry, order(group, by, identity, false)?)))
            .collect::<Result<_, Error>>()?;
        let children = children
            .into_iter()
            .map(|(parent, group)| Ok((parent, order(group, by, identity, true)?)))
            .collect::<Result<_, Error>>()?;
        Ok(BlockTree { roots, children })
    }

    pub(crate) fn roots_of(&self, entry: EntryId) -> &[&'a BlockRecord] {
        self.roots.get(&entry).map_or(&[], Vec::as_slice)
    }

    pub(crate) fn children_of(&self, block: BlockId) -> &[&'a BlockRecord] {
        self.children.get(&block).map_or(&[], Vec::as_slice)
    }

    pub(crate) fn value_of(&self, row: &BlockRecord) -> Result<Value, Error> {
        let mut nested = Vec::new();
        for child in self.children_of(row.id()) {
            nested.push(block_value(child, Vec::new())?);
        }
        block_value(row, nested)
    }
}

fn check_references(
    blocks: &[BlockRecord],
    rows: &HashMap<BlockId, &BlockRecord>,
) -> Result<(), Error> {
    for row in blocks {
        let Some(parent) = row.parent() else { continue };
        match rows.get(&parent) {
            Some(found) if found.entry() == row.entry() => {}
            _ => {
                return Err(Error::new(ErrorCode::UnknownReference)
                    .with_id(row.id().as_uuid())
                    .at("parent"));
            }
        }
    }
    Ok(())
}

pub(super) fn check_containment(
    blocks: &[BlockRecord],
    rows: &HashMap<BlockId, &BlockRecord>,
) -> Result<(), Error> {
    for row in blocks {
        let Some(parent) = row.parent() else { continue };
        let unknown = || {
            Error::new(ErrorCode::UnknownReference)
                .with_id(row.id().as_uuid())
                .at("parent")
        };
        let parent = rows.get(&parent).ok_or_else(unknown)?;
        if !is_container(parent.kind()) {
            return Err(Error::new(ErrorCode::DisallowedBlockKind)
                .with_id(row.id().as_uuid())
                .at("parent"));
        }
        if parent.state() != RepresentationState::Complete {
            return Err(Error::new(ErrorCode::InvalidRecord)
                .with_id(row.id().as_uuid())
                .at("parent"));
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "tree_tests.rs"]
mod tests;
