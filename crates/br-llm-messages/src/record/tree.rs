use std::collections::{HashMap, HashSet};

use super::block::BlockRecord;
use super::kinds::RepresentationState;
use super::payload::block_value;
use crate::block::ContentBlockKind;
use crate::error::{Error, ErrorCode};
use crate::id::{BlockId, EntryId};
use serde_json::Value;

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
        check_acyclic(blocks, &rows)?;
        check_containment(blocks, &rows)?;
        let mut roots: HashMap<EntryId, Vec<&BlockRecord>> = HashMap::new();
        let mut children: HashMap<BlockId, Vec<&BlockRecord>> = HashMap::new();
        for row in blocks {
            match row.parent() {
                None => roots.entry(row.entry()).or_default().push(row),
                Some(parent) => children.entry(parent).or_default().push(row),
            }
        }
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
                return Err(structure(row));
            }
        }
    }
    Ok(())
}

fn structure(row: &BlockRecord) -> Error {
    Error::new(ErrorCode::InvalidRecordStructure)
        .with_id(row.id().as_uuid())
        .at("parent")
}

fn check_acyclic(
    blocks: &[BlockRecord],
    rows: &HashMap<BlockId, &BlockRecord>,
) -> Result<(), Error> {
    let mut settled: HashSet<BlockId> = HashSet::new();
    for row in blocks {
        let mut path: HashSet<BlockId> = HashSet::new();
        let mut cursor = Some(row.id());
        while let Some(current) = cursor {
            if settled.contains(&current) {
                break;
            }
            if !path.insert(current) {
                return Err(structure(row));
            }
            cursor = rows.get(&current).and_then(|found| found.parent());
        }
        settled.extend(path);
    }
    Ok(())
}

pub(super) fn check_containment(
    blocks: &[BlockRecord],
    rows: &HashMap<BlockId, &BlockRecord>,
) -> Result<(), Error> {
    for row in blocks {
        let Some(parent) = row.parent() else { continue };
        let parent = rows.get(&parent).ok_or_else(|| structure(row))?;
        if !is_container(parent.kind()) || parent.state() != RepresentationState::Complete {
            return Err(structure(row));
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "tree_tests.rs"]
mod tests;
