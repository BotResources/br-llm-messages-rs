mod common;
mod stream_support;

use br_llm_messages::{
    ContentBlock, DeltaOp, ErrorCode, ProviderToolExecutionRecord, ProviderToolResult,
    RedactedThinkingBlock, ResponseEnding, ResultExposure,
};
use common::{bid, call, envelope, plain_text, png, result, signature, text_block};
use stream_support::{finish, ok, refuse, run, start_text, text};

fn insert(position: u32, block: impl Into<ContentBlock>) -> DeltaOp {
    DeltaOp::block_inserted(position, block)
}

#[test]
fn given_atomic_kinds_when_inserted_then_they_complete_without_any_fragment_delta() {
    let provider_result =
        ProviderToolResult::new(bid(12), bid(99), None, vec![plain_text(13, "found")], None)
            .unwrap();
    let record = ProviderToolExecutionRecord::new(
        bid(14),
        None,
        None,
        None,
        ResultExposure::opaque(envelope(serde_json::json!({"enc": "x"}))),
    )
    .unwrap();
    let redacted = RedactedThinkingBlock::new(bid(11), signature("redacted"));
    let mut draft = run(vec![
        insert(0, png(10)),
        insert(1, redacted.clone()),
        insert(2, provider_result.clone()),
        insert(3, record.clone()),
    ]);

    ok(&mut draft, finish());

    let blocks = draft.response().unwrap().blocks();
    assert_eq!(blocks[0], ContentBlock::from(png(10)));
    assert_eq!(blocks[1], ContentBlock::from(redacted));
    assert_eq!(blocks[2], ContentBlock::from(provider_result));
    assert_eq!(blocks[3], ContentBlock::from(record));
}

#[test]
fn given_a_runtime_tool_result_when_inserted_then_disallowed_block_kind() {
    let mut draft = run(vec![]);

    let error = refuse(&mut draft, insert(0, result(10, 11, "t")));

    assert_eq!(error.code(), ErrorCode::DisallowedBlockKind);
    assert_eq!(error.object_id(), Some(bid(10).as_uuid()));
}

#[test]
fn given_an_occupied_position_when_a_block_is_inserted_there_then_invalid_position() {
    let mut draft = run(vec![start_text(10, 0), insert(1, png(11))]);

    for position in [0, 1] {
        let error = refuse(&mut draft, insert(position, png(12)));
        assert_eq!(error.code(), ErrorCode::InvalidPosition);
        assert_eq!(error.object_id(), Some(bid(12).as_uuid()));
    }
}

#[test]
fn given_a_known_identity_when_a_block_is_inserted_then_reused_identity() {
    let mut draft = run(vec![start_text(10, 0), insert(1, png(11))]);

    let declared = refuse(&mut draft, insert(2, png(10)));
    let inserted = refuse(&mut draft, insert(2, png(11)));

    assert_eq!(declared.code(), ErrorCode::ReusedIdentity);
    assert_eq!(inserted.code(), ErrorCode::ReusedIdentity);
}

#[test]
fn given_nested_identities_that_clash_when_inserted_then_reused_identity_and_unchanged() {
    let mut draft = run(vec![insert(0, png(10))]);
    let clash =
        ProviderToolResult::new(bid(11), bid(99), None, vec![plain_text(10, "x")], None).unwrap();
    let cited = text_block(12, "cited");
    let with_citation = br_llm_messages::TextBlock::new(
        bid(13),
        "abc",
        vec![stream_support::citation(50, 0, 2)],
        None,
    )
    .unwrap();

    let nested = refuse(&mut draft, insert(1, clash));
    ok(&mut draft, insert(1, cited));
    ok(&mut draft, insert(2, with_citation));
    let citation = refuse(
        &mut draft,
        insert(3, text_block(50, "reuses a citation id")),
    );

    assert_eq!(nested.code(), ErrorCode::ReusedIdentity);
    assert_eq!(citation.code(), ErrorCode::ReusedIdentity);
}

#[test]
fn given_an_inserted_complete_block_when_mutated_afterwards_then_stream_phase() {
    let mut draft = run(vec![insert(0, text_block(10, "done"))]);

    let append = refuse(&mut draft, text(10, " more"));
    let finalize = refuse(&mut draft, stream_support::finalize(10));

    assert_eq!(append.code(), ErrorCode::StreamPhase);
    assert_eq!(finalize.code(), ErrorCode::StreamPhase);
}

#[test]
fn given_inserted_and_streamed_blocks_when_mixed_then_completion_validates_the_whole_response() {
    let mut draft = run(vec![
        insert(0, call(10, "lookup")),
        start_text(11, 1),
        text(11, "x"),
    ]);
    ok(&mut draft, stream_support::finalize(11));

    ok(
        &mut draft,
        stream_support::complete(ResponseEnding::ToolResultsRequested),
    );

    assert_eq!(draft.response().unwrap().runtime_calls().count(), 1);
}
