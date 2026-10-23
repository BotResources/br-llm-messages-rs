#![allow(dead_code)] // each integration test crate uses a different subset of these helpers

use br_llm_messages::{
    BlockMetadataUpdate, DeltaOp, ExecutionStatus, ExternalId, Finalization, ModelName,
    ProviderToolExecutionRecord, ProviderToolResult, RedactedThinkingBlock, ResponseDelta,
    ResponseDraft, ResponseEnding, ResponseMetadata, ResponseMetadataUpdate, ResultExposure, Usage,
};

use crate::common::{bid, plain_text, png, signature, tool};
use crate::stream_support::{
    citation, delta, finalize, json, operation, start_call, start_json, start_provider_call,
    start_record, start_text, start_thinking, text,
};

fn format() -> br_llm_messages::ReplayFormat {
    br_llm_messages::ReplayFormat::new("anthropic.signature").unwrap()
}

pub fn script() -> Vec<DeltaOp> {
    let hosted_result =
        ProviderToolResult::new(bid(18), bid(15), vec![plain_text(31, "found")]).unwrap();
    let record = ProviderToolExecutionRecord::new(
        bid(16),
        ResultExposure::readable(vec![plain_text(30, "log")]).unwrap(),
    )
    .unwrap()
    .with_operation(operation("code_run"))
    .with_status(ExecutionStatus::Completed);
    vec![
        DeltaOp::response_started(ResponseMetadata::new().with_model(ModelName::new("m").unwrap())),
        start_text(10, 3),
        start_thinking(11, 0),
        start_call(12, 2, None, None),
        start_json(13, 5),
        start_text(14, 4),
        start_provider_call(15, 6),
        start_record(16, 7),
        text(10, "Hello "),
        DeltaOp::thinking_appended(bid(11), "plan"),
        DeltaOp::replay_fragment_appended(bid(11), format(), "Sig"),
        DeltaOp::block_metadata_updated(
            bid(10),
            BlockMetadataUpdate::citation_added(citation(50, 0, 5)),
        ),
        json(12, "{\"query\":"),
        text(14, "  "),
        DeltaOp::response_metadata_updated(
            ResponseMetadataUpdate::new().with_usage(Usage::new().with_input_tokens(3)),
        ),
        text(10, "w\u{f6}rld"),
        DeltaOp::block_metadata_updated(
            bid(12),
            BlockMetadataUpdate::runtime_tool_call_identified(
                Some(tool("lookup")),
                Some(ExternalId::new("toolu_1").unwrap()),
            ),
        ),
        json(12, "\"rust\"}"),
        json(13, "{\"a\":[1,"),
        DeltaOp::block_inserted(1, png(17)),
        json(15, "{\"q\""),
        json(13, "2]}"),
        DeltaOp::block_metadata_updated(
            bid(16),
            BlockMetadataUpdate::execution_status_observed(ExecutionStatus::Running),
        ),
        json(15, ":\"x\"}"),
        DeltaOp::replay_fragment_appended(bid(10), format(), "Ab"),
        DeltaOp::replay_fragment_appended(bid(10), format(), "Cd"),
        finalize(10),
        finalize(11),
        finalize(12),
        finalize(13),
        DeltaOp::block_discarded(bid(14)),
        finalize(15),
        DeltaOp::block_finalized(bid(16), Finalization::authoritative(record.into())),
        DeltaOp::block_inserted(8, hosted_result),
        DeltaOp::block_inserted(9, RedactedThinkingBlock::new(bid(19), signature("opaque"))),
        DeltaOp::response_metadata_updated(
            ResponseMetadataUpdate::new()
                .with_usage(Usage::new().with_input_tokens(3).with_output_tokens(9)),
        ),
        DeltaOp::response_completed(ResponseEnding::tool_results_requested()),
    ]
}

pub fn deltas(ops: &[DeltaOp]) -> Vec<ResponseDelta> {
    ops.iter().cloned().map(delta).collect()
}

pub fn draft_after(script: &[ResponseDelta], count: usize) -> ResponseDraft {
    let mut draft = ResponseDraft::begin(&script[0]).unwrap();
    for step in &script[1..count] {
        draft.apply(step).unwrap();
    }
    draft
}
