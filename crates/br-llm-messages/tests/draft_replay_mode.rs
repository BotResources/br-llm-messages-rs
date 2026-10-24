mod common;
use common::Raw as _;
mod stream_support;

use br_llm_messages::{DeltaOp, DraftSnapshot, ErrorCode, ReplayFormat, ResponseDraft};
use common::{bid, signature};
use serde_json::json;
use stream_support::{ok, run, start_text, start_thinking};

fn format() -> ReplayFormat {
    ReplayFormat::new("anthropic.signature").unwrap()
}

fn fragment(block: u64, value: &str) -> DeltaOp {
    DeltaOp::replay_fragment_appended(bid(block), format(), value)
}

fn set_whole(block: u64, value: &str) -> DeltaOp {
    DeltaOp::replay_material_set(bid(block), signature(value))
}

fn restored(draft: &ResponseDraft) -> ResponseDraft {
    let snapshot = DraftSnapshot::from_json(&draft.to_snapshot().unwrap().to_json().unwrap());
    ResponseDraft::from_snapshot(&snapshot.unwrap()).unwrap()
}

#[test]
fn given_whole_text_material_when_a_fragment_of_the_same_format_arrives_then_it_is_appended() {
    let mut draft = run(vec![start_thinking(10), set_whole(10, "Ab")]);

    ok(&mut draft, fragment(10, "cd"));

    assert_eq!(draft.view().slots()[0].replay(), Some(&signature("Abcd")));
}

#[test]
fn given_whole_text_material_when_snapshotted_and_restored_then_fragments_keep_appending() {
    let draft = run(vec![start_thinking(10), set_whole(10, "Ab")]);
    let mut again = restored(&draft);

    ok(&mut again, fragment(10, "cd"));

    assert_eq!(again.view().slots()[0].replay(), Some(&signature("Abcd")));
    assert_eq!(restored(&again), again);
}

#[test]
fn given_fragment_material_when_snapshotted_and_restored_then_fragments_continue() {
    let draft = run(vec![start_thinking(10), fragment(10, "Ab")]);
    let mut again = restored(&draft);

    ok(&mut again, fragment(10, "cd"));

    assert_eq!(again.view().slots()[0].replay(), Some(&signature("Abcd")));
    assert_eq!(restored(&again), again);
}

#[test]
fn given_a_set_after_fragments_when_applied_then_it_is_refused_because_material_exists() {
    let mut draft = run(vec![start_thinking(10), fragment(10, "Ab")]);

    let error = stream_support::refuse(&mut draft, set_whole(10, "x"));

    assert_eq!(error.code(), ErrorCode::StreamTargetError);
}

#[test]
fn given_a_row_with_the_retired_fragment_mode_marker_when_loaded_then_it_is_an_unknown_field() {
    let draft = run(vec![start_thinking(10)]);
    let mut document = draft.to_snapshot().unwrap().to_value().unwrap().raw();
    document["blocks"][0]["payload"]["replay_fragments"] = json!(true);

    let error = DraftSnapshot::from_value(common::oj(document)).unwrap_err();

    assert_eq!(error.code(), ErrorCode::InvalidContent);
}

#[test]
fn given_no_material_when_an_empty_fragment_arrives_then_nothing_is_created_and_the_draft_is_equal()
{
    let mut draft = run(vec![start_text(10), start_thinking(11)]);
    let before = draft.clone();

    ok(&mut draft, fragment(10, ""));
    ok(&mut draft, fragment(11, ""));

    assert_eq!(draft, before);
    assert!(draft.view().slots()[1].replay().is_none());
}

#[test]
fn given_material_when_an_empty_fragment_arrives_then_it_is_unchanged() {
    let mut draft = run(vec![start_thinking(10), fragment(10, "Ab")]);
    let before = draft.clone();

    ok(&mut draft, fragment(10, ""));

    assert_eq!(draft, before);
}

#[test]
fn given_an_empty_first_fragment_then_a_real_one_when_finalized_then_the_signature_is_the_real_one()
{
    let mut draft = run(vec![
        start_thinking(10),
        fragment(10, ""),
        fragment(10, "Ab"),
    ]);

    ok(&mut draft, stream_support::finalize(10));

    let block = draft.view().slot(bid(10)).unwrap().final_block().cloned();
    assert_eq!(
        block,
        Some(br_llm_messages::ThinkingBlock::replay_only(bid(10), signature("Ab")).into())
    );
}
