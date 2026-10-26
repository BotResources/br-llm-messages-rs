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

pub(crate) fn block_rows(
    entry: EntryId,
    block: &ContentBlock,
    out: &mut Vec<BlockRecord>,
) -> Result<(), Error> {
    let mut row = BlockRecord::new(
        block.id(),
        entry,
        block.kind(),
        RepresentationState::Complete,
        block_payload(block)?,
    );
    if let Some(call) = call_of(block) {
        row = row.with_call(call);
    }
    if let Some(observed_at) = block.observed_at() {
        row = row.with_observed_at(observed_at.clone());
    }
    out.push(row);
    for child in block.children() {
        let nested = ContentBlock::from(child.clone());
        let mut row = BlockRecord::new(
            nested.id(),
            entry,
            nested.kind(),
            RepresentationState::Complete,
            block_payload(&nested)?,
        )
        .with_parent(block.id());
        if let Some(observed_at) = nested.observed_at() {
            row = row.with_observed_at(observed_at.clone());
        }
        out.push(row);
    }
    Ok(())
}

fn entry_rows(turn: TurnId, entry: &TurnEntry, set: &mut RecordSet) -> Result<(), Error> {
    let parts = split_entry(entry)?;
    let mut row = EntryRecord::new(
        entry.id(),
        turn,
        entry.kind(),
        RepresentationState::Complete,
        parts.payload,
    );
    if let Some(sender) = parts.sender {
        row = row.with_sender(sender);
    }
    if let Some(author) = parts.author {
        row = row.with_author(author);
    }
    if let Some(observed_at) = parts.observed_at {
        row = row.with_observed_at(observed_at);
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
    for block in &blocks {
        block_rows(entry.id(), block, &mut set.blocks)?;
    }
    Ok(())
}

pub(crate) fn turn_rows(turn: &Turn, set: &mut RecordSet) -> Result<(), Error> {
    match turn {
        Turn::InputTurn(input) => {
            let mut row = TurnRecord::new(input.id(), TurnKind::InputTurn);
            if let Some(observed_at) = input.observed_at() {
                row = row.with_observed_at(observed_at.clone());
            }
            set.turns.push(row);
            for entry in input.entries() {
                entry_rows(input.id(), &TurnEntry::from(entry.clone()), set)?;
            }
            Ok(())
        }
        Turn::AITurn(ai) => {
            let mut row = TurnRecord::new(ai.id(), TurnKind::AITurn)
                .with_agent(ai.agent())
                .with_author(ai.author().clone());
            if let Some(observed_at) = ai.observed_at() {
                row = row.with_observed_at(observed_at.clone());
            }
            set.turns.push(row);
            for entry in ai.entries() {
                entry_rows(ai.id(), entry, set)?;
            }
            Ok(())
        }
    }
}
