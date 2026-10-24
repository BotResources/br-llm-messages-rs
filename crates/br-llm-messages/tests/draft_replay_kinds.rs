mod common;
mod stream_support;

use br_llm_messages::{
    BlockStart, ContentBlock, DeltaOp, DraftSnapshot, ErrorCode, ProviderToolResult,
    RedactedThinkingBlock, ReplayFormat, ResponseDraft,
};
use common::{bid, envelope, signature};
use stream_support::{finalize, finish, ok, refuse, run, start_provider_call, start_record};

fn format() -> ReplayFormat {
    ReplayFormat::new("anthropic.signature").unwrap()
}

fn start_redacted(block: u64) -> DeltaOp {
    DeltaOp::block_started(bid(block), BlockStart::redacted_thinking())
}

fn start_result(block: u64, call: u64) -> DeltaOp {
    DeltaOp::block_started(
        bid(block),
        BlockStart::provider_tool_result(bid(call), None),
    )
}

fn fragment(block: u64, value: &str) -> DeltaOp {
    DeltaOp::replay_fragment_appended(bid(block), format(), value)
}

fn final_block(draft: &ResponseDraft, block: u64) -> ContentBlock {
    draft
        .view()
        .slot(bid(block))
        .unwrap()
        .final_block()
        .cloned()
        .unwrap()
}

#[test]
fn given_a_redacted_thinking_block_when_replay_arrives_whole_or_in_fragments_then_it_finalizes_with_exactly_that_material()
 {
    let mut draft = run(vec![
        start_redacted(10),
        start_redacted(11),
        fragment(10, "Ab+/"),
        fragment(10, "cd=="),
        DeltaOp::replay_material_set(bid(11), envelope(serde_json::json!({"enc": [1]}))),
    ]);

    ok(&mut draft, finalize(10));
    ok(&mut draft, finalize(11));

    assert_eq!(
        final_block(&draft, 10),
        RedactedThinkingBlock::new(bid(10), signature("Ab+/cd==")).into()
    );
    assert_eq!(
        final_block(&draft, 11),
        RedactedThinkingBlock::new(bid(11), envelope(serde_json::json!({"enc": [1]}))).into()
    );
}

#[test]
fn given_a_redacted_thinking_block_without_replay_when_finalized_then_inconsistent_finalization() {
    let mut draft = run(vec![start_redacted(10), fragment(10, "")]);

    let error = refuse(&mut draft, finalize(10));

    assert_eq!(error.code(), ErrorCode::InconsistentFinalization);
    assert_eq!(error.object_id(), Some(bid(10).as_uuid()));
}

#[test]
fn given_a_provider_result_started_with_its_call_when_replay_arrives_then_the_response_links_and_completes()
 {
    let mut draft = run(vec![
        start_provider_call(10),
        DeltaOp::block_started(
            bid(11),
            BlockStart::provider_tool_result(
                bid(10),
                Some(br_llm_messages::ExternalId::new("srv_1").unwrap()),
            ),
        ),
        fragment(11, "opaque"),
    ]);
    ok(&mut draft, finalize(10));
    ok(&mut draft, finalize(11));

    ok(&mut draft, finish());

    let expected = ProviderToolResult::replay_only(bid(11), bid(10), signature("opaque"))
        .unwrap()
        .with_external_id(br_llm_messages::ExternalId::new("srv_1").unwrap());
    assert_eq!(final_block(&draft, 11), expected.into());
    assert_eq!(draft.response().unwrap().blocks().len(), 2);
}

#[test]
fn given_a_provider_result_declared_before_its_call_in_the_same_response_when_completing_then_refused()
 {
    let mut draft = run(vec![
        start_result(11, 12),
        fragment(11, "opaque"),
        start_provider_call(12),
    ]);
    ok(&mut draft, finalize(11));
    ok(&mut draft, finalize(12));

    let error = refuse(&mut draft, finish());

    assert_eq!(error.code(), ErrorCode::UnknownReference);
}

#[test]
fn given_a_provider_result_without_replay_when_finalized_accumulated_then_inconsistent_finalization()
 {
    let mut draft = run(vec![start_provider_call(10), start_result(11, 10)]);

    let error = refuse(&mut draft, finalize(11));

    assert_eq!(error.code(), ErrorCode::InconsistentFinalization);
}

#[test]
fn given_an_execution_record_when_replay_arrives_in_fragments_then_it_is_kept_and_the_record_finalizes()
 {
    let mut draft = run(vec![
        start_record(10),
        fragment(10, "lo"),
        fragment(10, "g"),
    ]);

    ok(&mut draft, finalize(10));

    match final_block(&draft, 10) {
        ContentBlock::ProviderToolExecutionRecord(record) => {
            assert_eq!(record.replay(), Some(&signature("log")));
        }
        other => panic!("unexpected {other:?}"),
    }
}

#[test]
fn given_open_blocks_of_the_replay_kinds_when_snapshotted_and_restored_then_the_draft_is_equal_and_continues()
 {
    let draft = run(vec![
        start_redacted(10),
        fragment(10, "Ab"),
        start_provider_call(11),
        start_result(12, 11),
        fragment(12, "R"),
        start_record(13),
        fragment(13, "L"),
    ]);
    let snapshot = draft.to_snapshot().unwrap();
    let reloaded = DraftSnapshot::from_json(&snapshot.to_json().unwrap()).unwrap();
    let mut restored = ResponseDraft::from_snapshot(&reloaded).unwrap();

    assert_eq!(restored, draft);
    for (block, tail) in [(10, "cd"), (12, "s"), (13, "g")] {
        ok(&mut restored, fragment(block, tail));
    }
    let replays: Vec<_> = [10, 12, 13]
        .into_iter()
        .map(|block| restored.view().slot(bid(block)).unwrap().replay().cloned())
        .collect();
    assert_eq!(
        replays,
        [
            Some(signature("Abcd")),
            Some(signature("Rs")),
            Some(signature("Lg"))
        ]
    );
}
