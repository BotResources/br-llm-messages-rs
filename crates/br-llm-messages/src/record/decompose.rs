use super::block::BlockRecord;
use super::entry::EntryRecord;
use super::kinds::RepresentationState;
use super::payload::{block_payload, call_of, split_entry};
use super::set::RecordSet;
use super::turn::TurnRecord;
use crate::block::ContentBlock;
use crate::entry::TurnEntry;
use crate::error::Error;
use crate::id::{EntryId, TurnId};
use crate::turn::{Turn, TurnKind};

pub(crate) fn position(index: usize) -> u32 {
    u32::try_from(index).unwrap_or(u32::MAX)
}

pub(crate) fn block_rows(
    entry: EntryId,
    block: &ContentBlock,
    at: u32,
    out: &mut Vec<BlockRecord>,
) -> Result<(), Error> {
    let mut row = BlockRecord::new(
        block.id(),
        entry,
        block.kind(),
        at,
        RepresentationState::Complete,
        block_payload(block)?,
    );
    if let Some(call) = call_of(block) {
        row = row.with_call(call);
    }
    out.push(row);
    for (index, child) in block.children().iter().enumerate() {
        let nested = ContentBlock::from(child.clone());
        out.push(
            BlockRecord::new(
                nested.id(),
                entry,
                nested.kind(),
                position(index),
                RepresentationState::Complete,
                block_payload(&nested)?,
            )
            .with_parent(block.id()),
        );
    }
    Ok(())
}

fn entry_rows(turn: TurnId, entry: &TurnEntry, at: u32, set: &mut RecordSet) -> Result<(), Error> {
    let parts = split_entry(entry)?;
    let mut row = EntryRecord::new(
        entry.id(),
        turn,
        entry.kind(),
        at,
        RepresentationState::Complete,
        parts.payload,
    );
    if let Some(sender) = parts.sender {
        row = row.with_sender(sender);
    }
    if let Some(author) = parts.author {
        row = row.with_author(author);
    }
    set.entries.push(row);
    let blocks: Vec<ContentBlock> = match entry {
        TurnEntry::HumanMessage(message) => {
            message.content().iter().cloned().map(Into::into).collect()
        }
        TurnEntry::AgentRequest(request) => {
            request.content().iter().cloned().map(Into::into).collect()
        }
        TurnEntry::AgentMail(mail) => mail.content().iter().cloned().map(Into::into).collect(),
        TurnEntry::ModelResponse(response) => response.blocks().to_vec(),
        TurnEntry::RuntimeToolResults(results) => results
            .results()
            .iter()
            .cloned()
            .map(ContentBlock::RuntimeToolResult)
            .collect(),
        _ => Vec::new(),
    };
    for (index, block) in blocks.iter().enumerate() {
        block_rows(entry.id(), block, position(index), &mut set.blocks)?;
    }
    Ok(())
}

pub(crate) fn turn_rows(turn: &Turn, at: u32, set: &mut RecordSet) -> Result<(), Error> {
    match turn {
        Turn::InputTurn(input) => {
            set.turns.push(TurnRecord::new(
                input.id(),
                TurnKind::InputTurn,
                at,
                None,
                None,
            ));
            let entry = TurnEntry::from(input.entry().clone());
            entry_rows(input.id(), &entry, 0, set)
        }
        Turn::AITurn(ai) => {
            set.turns.push(TurnRecord::new(
                ai.id(),
                TurnKind::AITurn,
                at,
                Some(ai.agent()),
                Some(ai.author().clone()),
            ));
            for (index, entry) in ai.entries().iter().enumerate() {
                entry_rows(ai.id(), entry, position(index), set)?;
            }
            Ok(())
        }
    }
}
