use std::collections::{HashMap, HashSet};

use uuid::Uuid;

use super::block::BlockRecord;
use super::entry::EntryRecord;
use super::kinds::RepresentationState;
use super::partial::{TextPartial, decode as decode_partial};
use super::reconstruct::{
    build_entry, build_turn, ensure_parents, ensure_unique, entries_by_turn, ids_of,
};
use super::reduced_check::{Owners, check_rows, check_states, other_turns, without_interrupted};
use super::set::RecordSet;
use super::tree::BlockTree;
use super::turn::TurnRecord;
use crate::block::{ContentBlock, ContentBlockKind, PlainBlock};
use crate::entry::{AgentOutput, IncomingEntry, PartialAgentOutput, TurnEntryKind};
use crate::error::{Error, ErrorCode};
use crate::id::{AgentId, ConversationId, TurnId};
use crate::projection::{
    Owner, PendingResponse, ProjectedThread, accumulated_text, pending_entries, translated_turn,
    usable_block,
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
    agent: AgentId,
    author: &crate::value::Author,
    tree: &BlockTree<'_>,
) -> Result<Option<IncomingEntry>, Error> {
    let mut blocks = Vec::new();
    for block in tree.roots_of(entry.id()) {
        blocks.extend(block_of(block, tree)?);
    }
    if blocks.is_empty() {
        return Ok(None);
    }
    let author = author.clone();
    if entry.state() == RepresentationState::Complete {
        AgentOutput::from_content(entry.id(), agent, author, blocks)
            .map(|output| Some(IncomingEntry::AgentOutput(output)))
    } else {
        PartialAgentOutput::from_content(entry.id(), agent, author, blocks)
            .map(|output| Some(IncomingEntry::PartialAgentOutput(output)))
    }
}

fn translate(
    row: &TurnRecord,
    entries: &[&EntryRecord],
    tree: &BlockTree<'_>,
    pending: Vec<IncomingEntry>,
) -> Result<Option<Turn>, Error> {
    let (Some(agent), Some(author)) = (row.agent(), row.author()) else {
        return Err(Error::new(ErrorCode::InvalidContent).with_id(row.id().as_uuid()));
    };
    let mut kept = Vec::new();
    for (index, entry) in entries.iter().enumerate() {
        let built = translate_row(entry, agent, author, tree)
            .map_err(|error| error.at(format!("entries[{index}]")))?;
        kept.extend(built);
    }
    kept.extend(pending);
    translated_turn(row.id(), kept)
}

fn translate_row(
    entry: &EntryRecord,
    agent: AgentId,
    author: &crate::value::Author,
    tree: &BlockTree<'_>,
) -> Result<Option<IncomingEntry>, Error> {
    if entry.kind() == TurnEntryKind::ModelResponse {
        return translate_entry(entry, agent, author, tree);
    }
    IncomingEntry::try_from(build_entry(entry, tree)?).map(Some)
}

fn owners_of(set: &RecordSet) -> HashMap<TurnId, Owner> {
    set.turns()
        .iter()
        .filter(|row| row.kind() == TurnKind::AITurn)
        .filter_map(|row| Some((row.id(), (row.agent()?, row.author()?.clone()))))
        .collect()
}

pub(super) fn rebuild_thread(
    set: &RecordSet,
    conversation: ConversationId,
    agent: AgentId,
    pending: &[PendingResponse<'_>],
) -> Result<ProjectedThread, Error> {
    let others = other_turns(set, agent);
    let set = &without_interrupted(set, &others);
    check_rows(set, &others)?;
    ensure_unique(set)?;
    ensure_parents(set)?;
    let owners: Owners = set
        .entries()
        .iter()
        .map(|row| (row.id(), (row.turn(), row.state())))
        .collect();
    check_states(set, &others, &owners)?;
    let mut taken: HashSet<Uuid> = ids_of(set).collect();
    taken.insert(conversation.as_uuid());
    let turn_owners = owners_of(set);
    let mut pending = pending_entries(
        agent,
        pending,
        |turn| turn_owners.get(&turn).cloned(),
        &taken,
    )?;
    let tree = BlockTree::build(set.blocks())?;
    let mut by_turn = entries_by_turn(set);
    let mut turns = Vec::with_capacity(set.turns().len());
    for (index, row) in set.turns().iter().enumerate() {
        let entries = by_turn.remove(&row.id()).unwrap_or_default();
        let built = if others.contains(&row.id()) {
            translate(
                row,
                &entries,
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
