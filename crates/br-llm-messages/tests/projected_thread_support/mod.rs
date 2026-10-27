#![allow(
    dead_code,
    reason = "each integration test crate uses a different subset of these helpers"
)]

use crate::common::Raw as _;
use crate::common::pending_of;
use crate::common::turns::{human, response, tid};
use crate::common::{aid, author, bid, cvid, eid, oj};
use br_llm_messages::{
    AITurn, BlockStart, DeltaOp, Perspective, ProjectedThread, ResponseDelta, ResponseDraft,
    ResponseEnding, ResponseMetadata, TextBlock, Turn, TurnEntry,
};
use serde_json::Value;

pub fn ai(id: u64, agent: u64, entries: Vec<TurnEntry>) -> Turn {
    AITurn::from_parts(tid(id), aid(agent), author("Claude"), entries)
        .unwrap()
        .into()
}

pub fn said(entry: u64, block: u64, text: &str) -> TurnEntry {
    response(
        entry,
        vec![TextBlock::new(bid(block), text).unwrap().into()],
        ResponseEnding::finished(),
    )
}

pub fn draft() -> ResponseDraft {
    let mut draft = ResponseDraft::new(tid(30), eid(32));
    for op in [
        DeltaOp::response_started(ResponseMetadata::new()),
        DeltaOp::block_started(bid(33), BlockStart::text()),
        DeltaOp::text_appended(bid(33), "so far"),
    ] {
        draft
            .apply(&ResponseDelta::new(tid(30), eid(32), op))
            .unwrap();
    }
    draft
}

pub fn thread() -> ProjectedThread {
    let turns = vec![
        crate::common::turns::input_turn(1, 2).into(),
        ai(10, 1, vec![human(14), said(15, 16, "mine")]),
        ai(20, 2, vec![said(21, 22, "complete")]),
        AITurn::open(tid(30), aid(3), author("Claude")).into(),
        ai(
            40,
            4,
            vec![said(41, 42, "one"), human(43), said(44, 45, "two")],
        ),
    ];
    Perspective::of(aid(1))
        .project_pending(cvid(900), &turns, &pending_of(&[draft()]))
        .unwrap()
}

pub fn document() -> Value {
    thread().to_value().unwrap().raw()
}

pub fn load(document: Value) -> Result<ProjectedThread, br_llm_messages::Error> {
    ProjectedThread::from_value(oj(document))
}
