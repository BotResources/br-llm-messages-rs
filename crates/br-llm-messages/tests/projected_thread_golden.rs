mod common;
use common::Raw as _;
use common::pending_of;

use br_llm_messages::{
    AITurn, BlockStart, DeltaOp, ErrorCode, Perspective, ProjectedThread, ResponseDelta,
    ResponseDraft, ResponseEnding, ResponseMetadata, StructuredBlock, TextBlock, Turn,
};
use common::turns::{human, response, tid};
use common::{aid, author, bid, cvid, eid, oj, png, refused};
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

fn producing(entry: u64, block: u64, text: &str) -> ResponseDraft {
    let mut draft = ResponseDraft::new(tid(30), eid(entry));
    for op in [
        DeltaOp::response_started(ResponseMetadata::new()),
        DeltaOp::block_started(bid(block), BlockStart::text()),
        DeltaOp::text_appended(bid(block), text),
    ] {
        draft
            .apply(&ResponseDelta::new(tid(30), eid(entry), op))
            .unwrap();
    }
    draft
}

fn rich_response() -> br_llm_messages::TurnEntry {
    response(
        21,
        vec![
            TextBlock::new(bid(22), "done").unwrap().into(),
            png(23).with_observed_at(common::at(4)).into(),
            StructuredBlock::new(bid(24), oj(json!({"rows": [1, 2]})))
                .unwrap()
                .into(),
        ],
        ResponseEnding::finished(),
    )
}

fn thread() -> ProjectedThread {
    let turns = vec![
        common::turns::input_turn(1, 2).into(),
        ai(10, 1, "Codex", vec![said(11, 12, "mine")]),
        ai(
            20,
            2,
            "Claude",
            vec![rich_response(), human(25), said(26, 27, "more")],
        ),
        ai(30, 3, "Gemini", vec![]),
    ];
    Perspective::of(aid(1))
        .project_pending(
            cvid(900),
            &turns,
            &pending_of(&[producing(31, 32, "so far"), producing(33, 34, "then")]),
        )
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
fn given_the_golden_document_when_inspected_then_it_keeps_order_kinds_and_bare_blocks() {
    let document = golden();
    let kinds = |turn: usize| -> Vec<String> {
        document["turns"][turn]["entries"]
            .as_array()
            .unwrap()
            .iter()
            .map(|entry| entry["type"].as_str().unwrap().to_string())
            .collect()
    };

    assert_eq!(kinds(2), ["agent_output", "human_message", "agent_output"]);
    assert_eq!(kinds(3), ["partial_agent_output", "partial_agent_output"]);
    let blocks = &document["turns"][2]["entries"][0]["content"];
    assert_eq!(blocks[1]["type"], "image");
    assert_eq!(blocks[2]["type"], "structured_content");
    assert!(blocks[1].get("observed_at").is_none());
}

#[test]
fn given_outputs_of_two_agents_in_one_input_turn_when_loaded_then_invalid_record_structure() {
    let mut document = golden();
    let mut second = document["turns"][2]["entries"][0].clone();
    second["id"] = json!(eid(40).to_string());
    second["content"] = json!([{"id": bid(41).to_string(), "text": "x", "type": "text"}]);
    second["agent"] = json!(aid(3).to_string());
    let first = document["turns"][2]["entries"][0].clone();
    document["turns"][2]["entries"] = json!([first, second]);

    let error = refused(ProjectedThread::from_value(oj(document)));

    assert_eq!(error.code(), ErrorCode::InvalidRecordStructure);
    assert_eq!(error.location(), Some("turns[2].entries[1]"));
}
