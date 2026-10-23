mod common;
use common::Raw as _;
mod stream_support;

use br_llm_messages::{DeltaOp, DraftSnapshot, ErrorCode, ReplayFormat, ResponseDraft};
use common::{bid, signature};
use serde_json::json;
use stream_support::{ok, refuse, run, start_thinking};

fn fragment(block: u64, value: &str) -> DeltaOp {
    DeltaOp::replay_fragment_appended(
        bid(block),
        ReplayFormat::new("anthropic.signature").unwrap(),
        value,
    )
}

fn set_whole(block: u64, value: &str) -> DeltaOp {
    DeltaOp::replay_material_set(bid(block), signature(value))
}

fn restored(draft: &ResponseDraft) -> ResponseDraft {
    let snapshot = DraftSnapshot::from_json(&draft.to_snapshot().unwrap().to_json().unwrap());
    ResponseDraft::from_snapshot(&snapshot.unwrap()).unwrap()
}

#[test]
fn given_whole_text_material_when_a_fragment_arrives_then_replay_mismatch_and_nothing_changes() {
    let mut draft = run(vec![start_thinking(10, 0), set_whole(10, "Ab")]);
    let before = draft.clone();

    let error = refuse(&mut draft, fragment(10, "cd"));

    assert_eq!(error.code(), ErrorCode::StreamTargetError);
    assert_eq!(error.object_id(), Some(bid(10).as_uuid()));
    assert_eq!(draft, before);
}

#[test]
fn given_whole_text_material_when_snapshotted_and_restored_then_fragments_stay_refused() {
    let draft = run(vec![start_thinking(10, 0), set_whole(10, "Ab")]);
    let mut again = restored(&draft);

    let error = refuse(&mut again, fragment(10, "cd"));

    assert_eq!(error.code(), ErrorCode::StreamTargetError);
}

#[test]
fn given_fragment_material_when_snapshotted_and_restored_then_fragments_continue() {
    let draft = run(vec![start_thinking(10, 0), fragment(10, "Ab")]);
    let mut again = restored(&draft);

    ok(&mut again, fragment(10, "cd"));

    assert_eq!(again.view().slots()[0].replay(), Some(&signature("Abcd")));
    assert_eq!(restored(&again), again);
}

#[test]
fn given_a_fragment_mode_marker_without_text_material_when_loaded_then_invalid_record() {
    let draft = run(vec![start_thinking(10, 0)]);
    let mut document = draft.to_snapshot().unwrap().to_value().unwrap().raw();
    document["blocks"][0]["payload"]["replay_fragments"] = json!(true);

    let error = DraftSnapshot::from_value(common::oj(document)).unwrap_err();

    assert_eq!(error.code(), ErrorCode::InvalidRecordStructure);
    assert_eq!(error.object_id(), Some(bid(10).as_uuid()));
}
