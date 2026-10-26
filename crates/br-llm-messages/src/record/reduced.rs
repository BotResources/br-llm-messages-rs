use std::collections::HashMap;

use super::block::BlockRecord;
use super::entry::EntryRecord;
use super::kinds::RepresentationState;
use super::partial::{TextPartial, decode as decode_partial};
use super::reconstruct::{build_entry, build_turn};
use super::reduced_rows::Rows;
use super::set::RecordSet;
use super::tree::BlockTree;
use super::turn::TurnRecord;
use crate::block::{ContentBlock, ContentBlockKind, PlainBlock};
use crate::entry::{IncomingEntry, TurnEntryKind};
use crate::error::{Error, ErrorCode};
use crate::id::{AgentId, ConversationId, TurnId};
use crate::projection::{
    Foreign, Item, Output, Owner, PendingResponse, Progress, ProjectedThread, Source, Sources,
    accumulated_text, usable_block,
};
use crate::turn::{Class, Declared, places_of_incoming};

struct Door<'a> {
    rows: Rows<'a>,
    tree: BlockTree<'a>,
    sources: Sources,
}

fn block_of(row: &BlockRecord, tree: &BlockTree<'_>) -> Result<Option<PlainBlock>, Error> {
    let located = |error: Error| error.with_id(row.id().as_uuid());
    if row.state() != RepresentationState::Complete {
        return match row.kind() {
            ContentBlockKind::Text => {
                let partial: TextPartial = decode_partial(row.payload()).map_err(located)?;
                Ok(accumulated_text(row.id(), &partial.text).map(PlainBlock::Text))
            }
            _ => Ok(None),
        };
    }
    let block: ContentBlock = crate::json::decode_value(tree.value_of(row)?).map_err(located)?;
    usable_block(&block)
        .map(Some)
        .ok_or_else(|| located(Error::new(ErrorCode::DisallowedKind)))
}

impl Door<'_> {
    fn block_at(&self, block: &BlockRecord) -> String {
        self.rows
            .block_rows
            .get(&block.id().as_uuid())
            .map_or_else(|| "blocks".to_owned(), |row| format!("blocks[{row}]"))
    }

    fn declare_rows(&mut self, declared: Declared, turn: usize) {
        let mut block = format!("turns[{turn}]");
        for (id, place) in declared {
            let at = match place.class {
                Class::Turn => format!("turns[{turn}]"),
                Class::Block(_) => {
                    if let Some(row) = self.rows.block_rows.get(&id) {
                        block = format!("blocks[{row}]");
                    }
                    block.clone()
                }
                Class::Citation => block.clone(),
                _ => match self.rows.entry_rows.get(&id) {
                    Some(row) => format!("entries[{row}]"),
                    None => format!("turns[{turn}]"),
                },
            };
            self.sources.declare(id, at);
        }
    }

    fn output(
        &mut self,
        index: usize,
        entry: &EntryRecord,
        progress: Progress,
    ) -> Result<Output, Error> {
        self.sources
            .declare(entry.id().as_uuid(), format!("entries[{index}]"));
        let mut blocks = Vec::new();
        for root in self.tree.roots_of(entry.id()) {
            let at = self.block_at(root);
            self.sources.declare(root.id().as_uuid(), at.clone());
            blocks.extend(block_of(root, &self.tree).map_err(|error| error.at(&at))?);
        }
        Ok(Output::new(entry.id(), progress, blocks))
    }

    fn ignored(&mut self, index: usize, entry: &EntryRecord) {
        self.sources
            .declare(entry.id().as_uuid(), format!("entries[{index}]"));
        let blocks: Vec<(usize, &BlockRecord)> = self.rows.ignored_blocks(entry.id()).collect();
        for (row, block) in blocks {
            self.sources
                .declare(block.id().as_uuid(), format!("blocks[{row}]"));
        }
    }

    fn foreign(
        &mut self,
        position: usize,
        turn: &TurnRecord,
        entries: &[(usize, &EntryRecord)],
    ) -> Result<Foreign, Error> {
        let owner = match (turn.agent(), turn.author()) {
            (Some(agent), Some(author)) => Owner {
                agent,
                author: author.clone(),
            },
            _ => {
                return Err(Error::new(ErrorCode::InvalidContent)
                    .with_id(turn.id().as_uuid())
                    .at(format!("turns[{position}]")));
            }
        };
        let mut items = Vec::new();
        for (ordinal, (index, entry)) in entries.iter().enumerate() {
            let progress = match (entry.kind(), entry.state()) {
                (TurnEntryKind::ModelResponse, RepresentationState::Interrupted) => None,
                (TurnEntryKind::ModelResponse, RepresentationState::Complete) => {
                    Some(Progress::Completed)
                }
                (TurnEntryKind::ModelResponse, RepresentationState::Producing) => {
                    Some(Progress::Producing)
                }
                _ => {
                    items.push(Item::Incoming(
                        self.incoming(position, ordinal, *index, entry)?,
                    ));
                    continue;
                }
            };
            match progress {
                None => self.ignored(*index, entry),
                Some(progress) => items.push(Item::Output(self.output(*index, entry, progress)?)),
            }
        }
        Ok(Foreign {
            id: turn.id(),
            owner,
            items,
        })
    }

    fn incoming(
        &mut self,
        position: usize,
        ordinal: usize,
        index: usize,
        entry: &EntryRecord,
    ) -> Result<IncomingEntry, Error> {
        let located = |error: Error| error.at(format!("turns[{position}].entries[{ordinal}]"));
        let built = build_entry(entry, &self.tree).map_err(located)?;
        let incoming = IncomingEntry::recorded(&built).ok_or_else(|| {
            Error::new(ErrorCode::DisallowedKind)
                .with_id(entry.id().as_uuid())
                .at(format!("entries[{index}].kind"))
        })?;
        self.declare_rows(places_of_incoming(&incoming, ordinal), position);
        Ok(incoming)
    }
}

pub(super) fn rebuild_thread(
    set: &RecordSet,
    conversation: ConversationId,
    agent: AgentId,
    pending: &[PendingResponse<'_>],
) -> Result<ProjectedThread, Error> {
    let rows = Rows::read(set, agent)?;
    let tree = BlockTree::build(rows.blocks().map(|(_, row)| row)).map_err(|error| match error
        .object_id()
        .and_then(|id| rows.block_rows.get(&id))
    {
        Some(row) => error.at(format!("blocks[{row}]")),
        None => error,
    })?;
    let mut door = Door {
        rows,
        tree,
        sources: Sources::default(),
    };
    let mut by_turn: HashMap<TurnId, Vec<(usize, &EntryRecord)>> = HashMap::new();
    for (index, row) in set.entries().iter().enumerate() {
        by_turn.entry(row.turn()).or_default().push((index, row));
    }
    for (position, turn) in set.turns().iter().enumerate() {
        let entries = by_turn.remove(&turn.id()).unwrap_or_default();
        if door.rows.foreign.contains(&turn.id()) {
            door.sources
                .declare(turn.id().as_uuid(), format!("turns[{position}]"));
            let foreign = door.foreign(position, turn, &entries)?;
            door.sources.push(Source::Foreign(foreign));
        } else {
            let rows = entries.iter().map(|(_, row)| *row).collect();
            let rebuilt = build_turn(turn, rows, &door.tree)
                .map_err(|error| error.at(format!("turns[{position}]")))?;
            door.declare_rows(rebuilt.declared(), position);
            door.sources.push(Source::Kept(rebuilt));
        }
    }
    Ok(door
        .sources
        .check(conversation, pending)?
        .project(conversation, agent))
}
