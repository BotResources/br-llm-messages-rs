#![allow(
    dead_code,
    reason = "each integration test crate uses a different subset of these helpers"
)]

use br_llm_messages::{Conversation, DeltaOp, RecordSet, ResponseDelta, ResponseDraft, Turn};

use crate::common::turns::{human, input_turn, tid};
use crate::common::{bid, cvid, eid};
use crate::projection_support::*;

pub fn turns() -> Vec<Turn> {
    vec![
        input_turn(1, 2).into(),
        ai(10, 2, vec![human(13), said(11, 12, "complete")]),
        waiting(30, 2),
        waiting(50, 2),
        waiting(70, 3),
        ai(90, 1, vec![said(91, 92, "mine")]),
        waiting(110, 1),
    ]
}

pub fn crashed_history() -> Conversation {
    Conversation::from_parts(cvid(900), turns()).unwrap()
}

pub fn drafts() -> Vec<ResponseDraft> {
    vec![
        producing(30, 40, 43, "typing"),
        interrupted(50, 60, 63),
        open_text(70, 80, 83, &["other agent typing"]),
        open_text(110, 120, 123, &["mine typing"]),
    ]
}

pub fn crashed(drafts: &[ResponseDraft]) -> RecordSet {
    let complete = RecordSet::from_conversation(&crashed_history()).unwrap();
    let mut entries = complete.entries().to_vec();
    let mut blocks = complete.blocks().to_vec();
    for draft in drafts {
        let snapshot = draft.to_snapshot().unwrap();
        entries.push(snapshot.entry().clone());
        blocks.extend(snapshot.blocks().iter().cloned());
    }
    complete.with_entries(entries).with_blocks(blocks)
}

pub fn more_text(turn: u64, entry: u64, text_block: u64, text: &str) -> ResponseDelta {
    ResponseDelta::new(
        tid(turn),
        eid(entry),
        DeltaOp::text_appended(bid(text_block), text),
    )
}
