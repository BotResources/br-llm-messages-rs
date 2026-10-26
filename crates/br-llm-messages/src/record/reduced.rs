use std::collections::{HashMap, HashSet};

use super::block::BlockRecord;
use super::entry::EntryRecord;
use super::kinds::RepresentationState;
use super::partial::{TextPartial, decode as decode_partial};
use super::reconstruct::{build_entry, build_turn, ensure_parents, ensure_unique, entries_by_turn};
use super::set::RecordSet;
use super::tree::BlockTree;
use super::turn::TurnRecord;
use crate::block::{ContentBlock, ContentBlockKind, PlainBlock};
use crate::entry::{AgentOutput, IncomingEntry, PartialAgentOutput, TurnEntryKind};
use crate::error::{Error, ErrorCode};
use crate::id::{AgentId, ConversationId, EntryId, TurnId};
use crate::projection::{ProjectedThread, accumulated_text, translated_turn, usable_block};
use crate::turn::{Turn, TurnKind};

type Owners = HashMap<EntryId, (TurnId, RepresentationState)>;

fn other_turns(set: &RecordSet, agent: AgentId) -> HashSet<TurnId> {
    set.turns()
        .iter()
        .filter(|row| {
            row.kind() == TurnKind::AITurn && row.agent().is_some_and(|owner| owner != agent)
        })
        .map(TurnRecord::id)
        .collect()
}

fn check_rows(set: &RecordSet, others: &HashSet<TurnId>) -> Result<(), Error> {
    for (index, row) in set.turns().iter().enumerate() {
        row.check_local()
            .map_err(|error| error.at(format!("turns[{index}]")))?;
    }
    for (index, row) in set.entries().iter().enumerate() {
        let checked = if others.contains(&row.turn()) {
            row.check_reduced()
        } else {
            row.check_local()
        };
        checked.map_err(|error| error.at(format!("entries[{index}]")))?;
    }
    for (index, row) in set.blocks().iter().enumerate() {
        row.check_local()
            .map_err(|error| error.at(format!("blocks[{index}]")))?;
    }
    Ok(())
}

fn refused(id: uuid::Uuid, location: String) -> Error {
    Error::new(ErrorCode::DisallowedKind)
        .with_id(id)
        .at(location)
}

fn check_states(set: &RecordSet, others: &HashSet<TurnId>, owners: &Owners) -> Result<(), Error> {
    for (index, row) in set.entries().iter().enumerate() {
        let foreign_response =
            others.contains(&row.turn()) && row.kind() == TurnEntryKind::ModelResponse;
        if !foreign_response && row.state() != RepresentationState::Complete {
            return Err(refused(
                row.id().as_uuid(),
                format!("entries[{index}].state"),
            ));
        }
    }
    for (index, row) in set.blocks().iter().enumerate() {
        let reduced = owners
            .get(&row.entry())
            .filter(|(turn, _)| others.contains(turn));
        let allowed = match reduced {
            Some((_, state)) => {
                row.state() == RepresentationState::Complete || row.state() == *state
            }
            None => row.state() == RepresentationState::Complete,
        };
        if !allowed {
            return Err(refused(
                row.id().as_uuid(),
                format!("blocks[{index}].state"),
            ));
        }
        let usable_root = row.kind().is_plain() && row.parent().is_none();
        if reduced.is_some() && !usable_root {
            return Err(refused(row.id().as_uuid(), format!("blocks[{index}]")));
        }
    }
    Ok(())
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
    Ok(translated_turn(row.id(), kept))
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

fn without_interrupted(set: &RecordSet) -> RecordSet {
    let dropped: HashSet<EntryId> = set
        .entries()
        .iter()
        .filter(|row| row.state() == RepresentationState::Interrupted)
        .map(EntryRecord::id)
        .collect();
    RecordSet::new()
        .with_turns(set.turns().to_vec())
        .with_entries(
            set.entries()
                .iter()
                .filter(|row| !dropped.contains(&row.id()))
                .cloned()
                .collect(),
        )
        .with_blocks(
            set.blocks()
                .iter()
                .filter(|row| {
                    row.state() != RepresentationState::Interrupted
                        && !dropped.contains(&row.entry())
                })
                .cloned()
                .collect(),
        )
}

pub(super) fn rebuild_thread(
    set: &RecordSet,
    conversation: ConversationId,
    agent: AgentId,
) -> Result<ProjectedThread, Error> {
    let set = &without_interrupted(set);
    let others = other_turns(set, agent);
    check_rows(set, &others)?;
    ensure_unique(set)?;
    ensure_parents(set)?;
    let owners: Owners = set
        .entries()
        .iter()
        .map(|row| (row.id(), (row.turn(), row.state())))
        .collect();
    check_states(set, &others, &owners)?;
    let tree = BlockTree::build(set.blocks())?;
    let mut by_turn = entries_by_turn(set);
    let mut turns = Vec::with_capacity(set.turns().len());
    for (index, row) in set.turns().iter().enumerate() {
        let entries = by_turn.remove(&row.id()).unwrap_or_default();
        let built = if others.contains(&row.id()) {
            translate(row, &entries, &tree)
        } else {
            build_turn(row, entries, &tree).map(Some)
        };
        if let Some(turn) = built.map_err(|error| error.at(format!("turns[{index}]")))? {
            turns.push(turn);
        }
    }
    ProjectedThread::assemble(conversation, agent, turns)
}
