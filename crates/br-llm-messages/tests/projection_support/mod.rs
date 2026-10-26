#![allow(
    dead_code,
    reason = "each integration test crate uses a different subset of these helpers"
)]

use br_llm_messages::{
    AITurn, BlockStart, ContentBlockKind, Conversation, DeltaOp, Finalization, IncomingEntry,
    InterruptionReason, RecordSet, ResponseDelta, ResponseDraft, ResponseEnding, ResponseMetadata,
    TextBlock, Turn,
};

use crate::common::turns::{human, response, tid};
use crate::common::{aid, author, bid, cvid, eid};

pub fn started(turn: u64, entry: u64) -> ResponseDraft {
    let mut draft = ResponseDraft::new(tid(turn), eid(entry));
    feed(
        &mut draft,
        turn,
        entry,
        vec![DeltaOp::response_started(ResponseMetadata::new())],
    );
    draft
}

pub fn feed(draft: &mut ResponseDraft, turn: u64, entry: u64, ops: Vec<DeltaOp>) {
    for op in ops {
        draft
            .apply(&ResponseDelta::new(tid(turn), eid(entry), op))
            .unwrap();
    }
}

pub fn producing(turn: u64, entry: u64, block: u64, text: &str) -> ResponseDraft {
    let mut draft = started(turn, entry);
    feed(
        &mut draft,
        turn,
        entry,
        vec![
            DeltaOp::block_started(bid(block), BlockStart::thinking()),
            DeltaOp::thinking_appended(bid(block), "private plan"),
            DeltaOp::block_finalized(bid(block), Finalization::accumulated()),
            DeltaOp::block_started(bid(block + 1), BlockStart::text()),
            DeltaOp::text_appended(bid(block + 1), text),
        ],
    );
    draft
}

pub fn interrupted(turn: u64, entry: u64, block: u64) -> ResponseDraft {
    let mut draft = producing(turn, entry, block, "cut short");
    feed(
        &mut draft,
        turn,
        entry,
        vec![DeltaOp::response_interrupted(
            InterruptionReason::cancelled(),
        )],
    );
    draft
}

pub fn ai(id: u64, agent: u64, entries: Vec<br_llm_messages::TurnEntry>) -> Turn {
    AITurn::from_parts(tid(id), aid(agent), author("Claude"), entries)
        .unwrap()
        .into()
}

pub fn said(entry: u64, block: u64, text: &str) -> br_llm_messages::TurnEntry {
    response(
        entry,
        vec![TextBlock::new(bid(block), text).unwrap().into()],
        ResponseEnding::finished(),
    )
}

pub fn kinds(turn: &Turn) -> Vec<&'static str> {
    let Turn::InputTurn(input) = turn else {
        panic!("input turn expected");
    };
    input
        .entries()
        .iter()
        .map(|entry| match entry {
            IncomingEntry::AgentOutput(_) => "output",
            IncomingEntry::PartialAgentOutput(_) => "partial",
            _ => "recorded",
        })
        .collect()
}

pub fn history() -> Vec<Turn> {
    vec![
        ai(10, 2, vec![said(11, 12, "complete")]),
        ai(30, 2, vec![human(31)]),
        ai(50, 2, vec![human(51)]),
    ]
}

pub fn open_text(turn: u64, entry: u64, block: u64, fragments: &[&str]) -> ResponseDraft {
    let mut draft = started(turn, entry);
    let mut ops = vec![DeltaOp::block_started(bid(block), BlockStart::text())];
    ops.extend(
        fragments
            .iter()
            .map(|text| DeltaOp::text_appended(bid(block), *text)),
    );
    feed(&mut draft, turn, entry, ops);
    draft
}

pub fn rows_with(drafts: &[ResponseDraft], turns: &[Turn], agent: u64) -> RecordSet {
    let conversation = Conversation::from_parts(cvid(900), turns.to_vec()).unwrap();
    let full = RecordSet::from_conversation(&conversation).unwrap();
    let mut entries = Vec::new();
    let mut blocks = Vec::new();
    for row in full.entries() {
        let owner = full.turns().iter().find(|t| t.id() == row.turn()).unwrap();
        let kept = owner.agent() == Some(aid(agent))
            || owner.agent().is_none()
            || row.kind() == br_llm_messages::TurnEntryKind::ModelResponse;
        if kept {
            entries.push(row.clone());
        }
    }
    for row in full.blocks() {
        if entries.iter().any(|e| e.id() == row.entry()) {
            blocks.push(row.clone());
        }
    }
    let blocks_of_owner = |row: &br_llm_messages::BlockRecord| {
        let entry = entries.iter().find(|e| e.id() == row.entry()).unwrap();
        let owner = full
            .turns()
            .iter()
            .find(|t| t.id() == entry.turn())
            .unwrap();
        owner.agent() == Some(aid(agent))
            || owner.agent().is_none()
            || (row.kind() == ContentBlockKind::Text && row.parent().is_none())
    };
    blocks.retain(blocks_of_owner);
    for draft in drafts {
        let snapshot = draft.to_snapshot().unwrap();
        entries.push(snapshot.entry().clone());
        blocks.extend(
            snapshot
                .blocks()
                .iter()
                .filter(|row| row.kind() == ContentBlockKind::Text && row.parent().is_none())
                .cloned(),
        );
    }
    RecordSet::new()
        .with_turns(full.turns().to_vec())
        .with_entries(entries)
        .with_blocks(blocks)
}
