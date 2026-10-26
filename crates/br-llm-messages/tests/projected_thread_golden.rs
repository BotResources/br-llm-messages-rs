mod common;
use common::Raw as _;

use br_llm_messages::{
    AITurn, BlockStart, DeltaOp, ErrorCode, Perspective, ProjectedThread, ResponseDelta,
    ResponseDraft, ResponseEnding, ResponseMetadata, TextBlock, Turn,
};
use common::turns::{response, tid};
use common::{aid, author, bid, cvid, eid, oj, refused};
use serde_json::{Value, json};

const GOLDEN: &str = include_str!("golden/projected_thread.json");

fn golden() -> Value {
    serde_json::from_str(GOLDEN).unwrap()
}

fn said(entry: u64, block: u64, text: &str) -> br_llm_messages::TurnEntry {
    response(
        entry,
        vec![TextBlock::new(bid(block), text).unwrap().into()],
        ResponseEnding::finished(),
    )
}

fn ai(id: u64, agent: u64, label: &str, entries: Vec<br_llm_messages::TurnEntry>) -> Turn {
    AITurn::from_parts(tid(id), aid(agent), author(label), entries)
        .unwrap()
        .into()
}

fn producing() -> ResponseDraft {
    let mut draft = ResponseDraft::new(tid(30), eid(31));
    for op in [
        DeltaOp::response_started(ResponseMetadata::new()),
        DeltaOp::block_started(bid(32), BlockStart::text()),
        DeltaOp::text_appended(bid(32), "so far"),
    ] {
        draft
            .apply(&ResponseDelta::new(tid(30), eid(31), op))
            .unwrap();
    }
    draft
}

fn thread() -> ProjectedThread {
    let turns = vec![
        common::turns::input_turn(1, 2).into(),
        ai(10, 1, "Codex", vec![said(11, 12, "mine")]),
        ai(20, 2, "Claude", vec![said(21, 22, "done")]),
        ai(30, 3, "Gemini", vec![]),
    ];
    Perspective::of(aid(1))
        .project_live(cvid(900), &turns, &[producing()])
        .unwrap()
}

#[test]
fn given_a_thread_when_serialized_then_it_matches_the_golden_document() {
    assert_eq!(thread().to_value().unwrap().raw(), golden());
}

#[test]
fn given_the_golden_document_when_loaded_then_it_builds_the_exact_thread() {
    let loaded = ProjectedThread::from_json(GOLDEN).unwrap();

    assert_eq!(loaded, thread());
    assert_eq!(loaded.to_value().unwrap().raw(), golden());
}

#[test]
fn given_the_golden_document_when_loaded_as_a_conversation_then_schema_mismatch() {
    let error = refused(br_llm_messages::Conversation::from_value(oj(golden())));

    assert_eq!(error.code(), ErrorCode::SchemaMismatch);
}

#[test]
fn given_outputs_of_two_agents_in_one_input_turn_when_loaded_then_invalid_record_structure() {
    let mut document = golden();
    let mut second = document["turns"][2]["entries"][0].clone();
    second["id"] = json!(eid(40).to_string());
    second["content"][0]["id"] = json!(bid(41).to_string());
    second["agent"] = json!(aid(3).to_string());
    document["turns"][2]["entries"] = json!([document["turns"][2]["entries"][0].clone(), second]);

    let error = refused(ProjectedThread::from_value(oj(document)));

    assert_eq!(error.code(), ErrorCode::InvalidRecordStructure);
    assert_eq!(error.location(), Some("turns[2].entries[1]"));
}
