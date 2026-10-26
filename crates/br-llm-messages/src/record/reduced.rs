use std::collections::HashSet;

use super::block::BlockRecord;
use super::entry::EntryRecord;
use super::kept::Kept;
use super::kinds::RepresentationState;
use super::partial::{TextPartial, decode as decode_partial};
use super::reconstruct::{
    build_entry, build_turn, ensure_parents, ensure_unique, entries_by_turn, ids_of,
};
use super::reduced_check::{
    Owners, check_rows, check_states, interrupted_foreign_responses, other_turns,
};
use super::set::RecordSet;
use super::tree::BlockTree;
use super::turn::TurnRecord;
use crate::block::{ContentBlock, ContentBlockKind, PlainBlock};
use crate::entry::{IncomingEntry, TurnEntryKind};
use crate::error::{Error, ErrorCode};
use crate::id::{AgentId, ConversationId, EntryId};
use crate::projection::{
    Owner, PendingResponse, ProjectedThread, accumulated_text, output_entry, pending_entries,
    translated_turn, usable_block,
};
use crate::turn::{Turn, TurnKind};

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

fn translate_entry(
    entry: &EntryRecord,
    owner: &Owner,
    tree: &BlockTree<'_>,
) -> Result<Option<IncomingEntry>, Error> {
    let mut blocks = Vec::new();
    for block in tree.roots_of(entry.id()) {
        blocks.extend(block_of(block, tree)?);
    }
    output_entry(
        entry.id(),
        owner,
        entry.state() == RepresentationState::Complete,
        blocks,
    )
}

fn translate(
    row: &TurnRecord,
    entries: &[&EntryRecord],
    dropped: &HashSet<EntryId>,
    tree: &BlockTree<'_>,
    pending: Vec<IncomingEntry>,
) -> Result<Option<Turn>, Error> {
    let Some(owner) = owner_of(row) else {
        return Err(Error::new(ErrorCode::InvalidContent).with_id(row.id().as_uuid()));
    };
    let mut kept = Vec::new();
    for (index, entry) in entries.iter().enumerate() {
        if dropped.contains(&entry.id()) {
            continue;
        }
        let built = translate_row(entry, &owner, tree)
            .map_err(|error| error.at(format!("entries[{index}]")))?;
        kept.extend(built);
    }
    kept.extend(pending);
    translated_turn(row.id(), kept)
}

fn translate_row(
    entry: &EntryRecord,
    owner: &Owner,
    tree: &BlockTree<'_>,
) -> Result<Option<IncomingEntry>, Error> {
    if entry.kind() == TurnEntryKind::ModelResponse {
        return translate_entry(entry, owner, tree);
    }
    IncomingEntry::try_from(build_entry(entry, tree)?).map(Some)
}

fn owner_of(row: &TurnRecord) -> Option<Owner> {
    Some((row.agent()?, row.author()?.clone()))
}

pub(super) fn rebuild_thread(
    set: &RecordSet,
    conversation: ConversationId,
    agent: AgentId,
    pending: &[PendingResponse<'_>],
) -> Result<ProjectedThread, Error> {
    let others = other_turns(set, agent);
    let dropped = interrupted_foreign_responses(set, &others);
    let kept = Kept::without(set, &dropped);
    check_rows(&kept, &others)?;
    ensure_unique(&kept)?;
    ensure_parents(&kept)?;
    let owners: Owners = kept
        .entries()
        .map(|(_, row)| (row.id(), (row.turn(), row.state())))
        .collect();
    check_states(&kept, &others, &owners)?;
    let turn_owners = set
        .turns()
        .iter()
        .filter(|row| row.kind() == TurnKind::AITurn)
        .filter_map(|row| Some((row.id(), owner_of(row)?)));
    let mut pending = pending_entries(agent, conversation, pending, turn_owners, ids_of(&kept))?;
    let tree = BlockTree::build(kept.blocks().map(|(_, row)| row))?;
    let mut by_turn = entries_by_turn(set);
    let mut turns = Vec::with_capacity(set.turns().len());
    for (index, row) in set.turns().iter().enumerate() {
        let entries = by_turn.remove(&row.id()).unwrap_or_default();
        let built = if others.contains(&row.id()) {
            translate(
                row,
                &entries,
                &dropped,
                &tree,
                pending.remove(&row.id()).unwrap_or_default(),
            )
        } else {
            build_turn(row, entries, &tree).map(Some)
        };
        if let Some(turn) = built.map_err(|error| error.at(format!("turns[{index}]")))? {
            turns.push(turn);
        }
    }
    ProjectedThread::assemble(conversation, agent, turns)
}
