#![allow(
    dead_code,
    reason = "each integration test crate uses a different subset of these helpers"
)]

use br_llm_messages::{
    AgentOutput, InputTurn, PartialAgentOutput, ProjectedThread, Turn, TurnEntry,
};

use crate::common::turns::{human, input_turn, tid};
use crate::common::{aid, author, cvid, eid, plain_text};
use crate::projected_thread_support::{ai, said};

pub fn output(entry: u64, agent: u64, block: u64) -> AgentOutput {
    AgentOutput::new(
        eid(entry),
        aid(agent),
        author("Codex"),
        vec![plain_text(block, "said")],
    )
    .unwrap()
}

pub fn partial(entry: u64, agent: u64, block: u64) -> PartialAgentOutput {
    PartialAgentOutput::new(
        eid(entry),
        aid(agent),
        author("Codex"),
        vec![plain_text(block, "so far")],
    )
    .unwrap()
}

pub fn human_with_block(entry: u64, block: u64) -> TurnEntry {
    br_llm_messages::HumanMessage::new(
        eid(entry),
        crate::common::hid(1),
        vec![plain_text(block, "x")],
    )
    .unwrap()
    .into()
}

pub fn foreign_turn(turn: u64, entry: u64, block: u64) -> Turn {
    InputTurn::new_projected(tid(turn), vec![output(entry, 2, block).into()])
        .unwrap()
        .into()
}

pub fn base_turns() -> Vec<Turn> {
    vec![
        input_turn(1, 2).into(),
        ai(10, 1, vec![human(14), said(15, 16, "mine")]),
        foreign_turn(20, 21, 22),
    ]
}

pub fn base() -> ProjectedThread {
    ProjectedThread::from_turns(cvid(900), aid(1), base_turns()).unwrap()
}
