mod common;
mod stream_support;

use br_llm_messages::{ContentBlock, DeltaOp, ErrorCode, ReplayFormat};
use common::bid;
use stream_support::{
    finalize, finish, json, ok, refuse, run, start_call, start_json, start_provider_call,
    start_record, start_text, start_thinking, text,
};

#[test]
fn given_text_fragments_when_appended_then_the_text_is_exactly_their_concatenation() {
    let mut draft = run(vec![start_text(10, 0)]);
    let fragments = ["  lead", " space é ", "\n\n", "trail  "];

    for fragment in fragments {
        ok(&mut draft, text(10, fragment));
    }
    ok(&mut draft, finalize(10));
    ok(&mut draft, finish());

    match &draft.response().unwrap().blocks()[0] {
        ContentBlock::Text(block) => assert_eq!(block.text().as_str(), fragments.concat()),
        other => panic!("unexpected block {other:?}"),
    }
}

#[test]
fn given_thinking_fragments_when_appended_then_thinking_text_accumulates_and_may_stay_empty() {
    let mut draft = run(vec![start_thinking(10, 0), start_thinking(11, 1)]);

    ok(
        &mut draft,
        DeltaOp::thinking_appended(bid(10), "step one; "),
    );
    ok(&mut draft, DeltaOp::thinking_appended(bid(10), "step two"));
    ok(&mut draft, finalize(10));
    ok(&mut draft, finalize(11));
    ok(&mut draft, finish());

    let blocks = draft.response().unwrap().blocks();
    match (&blocks[0], &blocks[1]) {
        (ContentBlock::Thinking(first), ContentBlock::Thinking(second)) => {
            assert_eq!(first.text(), "step one; step two");
            assert_eq!(second.text(), "");
        }
        other => panic!("unexpected blocks {other:?}"),
    }
}

#[test]
fn given_a_wrong_kind_target_when_appending_then_stream_target_with_the_block_id() {
    let mut draft = run(vec![
        start_text(10, 0),
        start_thinking(11, 1),
        start_record(12, 2),
    ]);
    let format = ReplayFormat::new("x.y").unwrap();

    let cases = [
        text(11, "a"),
        text(12, "a"),
        DeltaOp::thinking_appended(bid(10), "a"),
        DeltaOp::thinking_appended(bid(12), "a"),
        json(10, "{}"),
        json(11, "{}"),
        json(12, "{}"),
        DeltaOp::replay_fragment_appended(bid(12), format, "frag"),
    ];
    for op in cases {
        let block = op.block_id().unwrap();
        let error = refuse(&mut draft, op);
        assert_eq!(error.code(), ErrorCode::StreamTarget);
        assert_eq!(error.object_id(), Some(block.as_uuid()));
    }
}

#[test]
fn given_an_unknown_target_when_appending_then_stream_target() {
    let mut draft = run(vec![start_text(10, 0)]);

    let ops = [
        text(99, "a"),
        DeltaOp::thinking_appended(bid(99), "a"),
        json(99, "{}"),
        finalize(99),
        DeltaOp::block_discarded(bid(99)),
    ];
    for op in ops {
        let error = refuse(&mut draft, op);
        assert_eq!(error.code(), ErrorCode::StreamTarget);
        assert_eq!(error.object_id(), Some(bid(99).as_uuid()));
    }
}

#[test]
fn given_a_finalized_block_when_any_fragment_arrives_then_stream_phase() {
    let mut draft = run(vec![
        start_text(10, 0),
        text(10, "done"),
        finalize(10),
        start_thinking(11, 1),
        finalize(11),
        start_json(12, 2),
        json(12, "1"),
        finalize(12),
    ]);

    let ops = [
        text(10, "more"),
        DeltaOp::thinking_appended(bid(11), "more"),
        json(12, "2"),
        DeltaOp::replay_fragment_appended(bid(10), ReplayFormat::new("x.y").unwrap(), "f"),
        DeltaOp::replay_material_set(bid(10), common::signature("s")),
    ];
    for op in ops {
        let error = refuse(&mut draft, op);
        assert_eq!(error.code(), ErrorCode::StreamPhase);
    }
}

#[test]
fn given_json_fragments_when_appended_then_raw_text_accumulates_even_when_invalid_so_far() {
    let mut draft = run(vec![start_json(10, 0)]);

    ok(&mut draft, json(10, "{\"a\": [1, "));
    ok(&mut draft, json(10, "tru"));

    let slot = draft.view().slot(bid(10)).unwrap();
    assert_eq!(slot.raw_json(), Some("{\"a\": [1, tru"));
    assert!(slot.final_block().is_none());
}

#[test]
fn given_structured_runtime_and_provider_targets_when_json_is_appended_then_each_keeps_raw_text_only()
 {
    let mut draft = run(vec![
        start_json(10, 0),
        start_call(11, 1, Some("lookup"), None),
        start_provider_call(12, 2),
    ]);

    for block in [10, 11, 12] {
        ok(&mut draft, json(block, "{\"k\""));
        ok(&mut draft, json(block, ": 1}"));
    }

    let view = draft.view();
    for block in [10, 11, 12] {
        let slot = view.slot(bid(block)).unwrap();
        assert_eq!(slot.raw_json(), Some("{\"k\": 1}"));
        assert!(slot.final_block().is_none());
    }
}

#[test]
fn given_fragments_that_never_become_json_when_finalized_then_malformed_json_and_still_open() {
    let mut draft = run(vec![start_json(10, 0), json(10, "{\"a\":")]);

    let error = refuse(&mut draft, finalize(10));

    assert_eq!(error.code(), ErrorCode::MalformedJson);
    assert_eq!(
        draft.view().slot(bid(10)).unwrap().raw_json(),
        Some("{\"a\":")
    );
}
