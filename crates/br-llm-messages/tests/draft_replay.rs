mod common;
mod stream_support;

use br_llm_messages::{
    ContentBlock, DeltaOp, ErrorCode, ReplayFormat, ReplayMaterial, ReplayPayload,
};
use common::{bid, envelope, signature};
use stream_support::{
    finalize, finish, json, ok, refuse, run, start_call, start_json, start_record, start_text,
    start_thinking, text,
};

fn format(name: &str) -> ReplayFormat {
    ReplayFormat::new(name).unwrap()
}

fn fragment(block: u64, name: &str, value: &str) -> DeltaOp {
    DeltaOp::replay_fragment_appended(bid(block), format(name), value)
}

#[test]
fn given_signature_fragments_when_appended_then_one_text_material_is_built_exactly() {
    let mut draft = run(vec![start_thinking(10, 0)]);

    ok(&mut draft, fragment(10, "anthropic.signature", "Ab+/"));
    ok(&mut draft, fragment(10, "anthropic.signature", "cd=="));
    ok(&mut draft, finalize(10));
    ok(&mut draft, finish());

    match &draft.response().unwrap().blocks()[0] {
        ContentBlock::Thinking(block) => {
            assert_eq!(block.replay(), Some(&signature("Ab+/cd==")));
        }
        other => panic!("unexpected block {other:?}"),
    }
}

#[test]
fn given_material_with_another_format_when_a_fragment_arrives_then_replay_mismatch() {
    let mut draft = run(vec![
        start_thinking(10, 0),
        fragment(10, "anthropic.signature", "a"),
    ]);

    let error = refuse(&mut draft, fragment(10, "openai.reasoning", "b"));

    assert_eq!(error.code(), ErrorCode::StreamTargetError);
    assert_eq!(error.object_id(), Some(bid(10).as_uuid()));
}

#[test]
fn given_json_material_when_a_fragment_arrives_then_replay_mismatch_and_objects_are_never_joined() {
    let mut draft = run(vec![
        start_thinking(10, 0),
        DeltaOp::replay_material_set(bid(10), envelope(serde_json::json!({"enc": "A"}))),
    ]);

    let error = refuse(&mut draft, fragment(10, "openai.reasoning", "tail"));

    assert_eq!(error.code(), ErrorCode::StreamTargetError);
    ok(&mut draft, finalize(10));
    match &draft.view().slot(bid(10)).unwrap().final_block() {
        Some(ContentBlock::Thinking(block)) => {
            let payload = block.replay().unwrap().payload();
            assert_eq!(
                payload,
                &ReplayPayload::json(common::oj(serde_json::json!({"enc": "A"}))).unwrap()
            );
        }
        other => panic!("unexpected {other:?}"),
    }
}

#[test]
fn given_present_material_when_set_arrives_then_replay_mismatch() {
    let mut draft = run(vec![
        start_thinking(10, 0),
        start_text(11, 1),
        fragment(10, "anthropic.signature", "a"),
        DeltaOp::replay_material_set(bid(11), signature("first")),
    ]);

    let after_fragment = refuse(
        &mut draft,
        DeltaOp::replay_material_set(bid(10), signature("b")),
    );
    let after_set = refuse(
        &mut draft,
        DeltaOp::replay_material_set(bid(11), signature("c")),
    );

    assert_eq!(after_fragment.code(), ErrorCode::StreamTargetError);
    assert_eq!(after_set.code(), ErrorCode::StreamTargetError);
}

#[test]
fn given_a_target_without_a_replay_slot_when_replay_arrives_then_stream_target() {
    let mut draft = run(vec![start_json(10, 0), start_record(11, 1)]);

    for block in [10, 11] {
        let set = refuse(
            &mut draft,
            DeltaOp::replay_material_set(bid(block), signature("s")),
        );
        let piece = refuse(&mut draft, fragment(block, "anthropic.signature", "s"));
        assert_eq!(set.code(), ErrorCode::StreamTargetError);
        assert_eq!(piece.code(), ErrorCode::StreamTargetError);
    }
}

#[test]
fn given_replay_after_visible_text_when_finalized_then_text_and_replay_stay_independent() {
    let mut draft = run(vec![start_text(10, 0), text(10, "visible")]);

    ok(&mut draft, fragment(10, "anthropic.signature", "late"));
    ok(&mut draft, text(10, " tail"));
    ok(&mut draft, finalize(10));
    ok(&mut draft, finish());

    match &draft.response().unwrap().blocks()[0] {
        ContentBlock::Text(block) => {
            assert_eq!(block.text().as_str(), "visible tail");
            assert_eq!(block.replay(), Some(&signature("late")));
        }
        other => panic!("unexpected block {other:?}"),
    }
}

#[test]
fn given_a_tool_call_when_replay_is_set_then_it_survives_finalization() {
    let mut draft = run(vec![
        start_call(10, 0, Some("lookup"), None),
        json(10, "{}"),
        DeltaOp::replay_material_set(bid(10), envelope(serde_json::json!({"item": 1}))),
    ]);

    ok(&mut draft, finalize(10));

    match draft.view().slot(bid(10)).unwrap().final_block() {
        Some(ContentBlock::RuntimeToolCall(call)) => {
            let expected = ReplayMaterial::new(
                format("openai.reasoning"),
                ReplayPayload::json(common::oj(serde_json::json!({"item": 1}))).unwrap(),
            );
            assert_eq!(call.replay(), Some(&expected));
        }
        other => panic!("unexpected {other:?}"),
    }
}
