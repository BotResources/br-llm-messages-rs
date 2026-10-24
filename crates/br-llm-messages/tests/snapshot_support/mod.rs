#![allow(
    dead_code,
    reason = "each integration test crate uses a different subset of these helpers"
)]

use br_llm_messages::{
    BlockMetadataUpdate, BlockStart, DeltaOp, ExecutionStatus, ExternalId, Finalization, ItemRef,
    ModelName, ProviderToolExecutionRecord, ProviderToolResult, RedactedThinkingBlock,
    ResponseDelta, ResponseDraft, ResponseEnding, ResponseMetadata, ResponseMetadataUpdate, Usage,
};

use crate::common::{at, bid, plain_text, png, signature, tool};
use crate::stream_support::{
    citation, delta, finalize, json, operation, start_json, start_provider_call, start_record, text,
};

fn format() -> br_llm_messages::ReplayFormat {
    br_llm_messages::ReplayFormat::new("anthropic.signature").unwrap()
}

pub fn script() -> Vec<DeltaOp> {
    let hosted_result = ProviderToolResult::new(bid(18), bid(15), vec![plain_text(31, "found")])
        .unwrap()
        .with_observed_at(at(7));
    let record = ProviderToolExecutionRecord::new(bid(16))
        .with_content(vec![plain_text(30, "log")])
        .unwrap()
        .with_operation(operation("code_run"))
        .with_status(ExecutionStatus::Completed)
        .with_observed_at(at(6));
    vec![
        DeltaOp::response_started(
            ResponseMetadata::new()
                .with_model(ModelName::new("m").unwrap())
                .with_observed_at(at(1)),
        ),
        DeltaOp::block_started(bid(11), BlockStart::thinking().with_observed_at(at(4))),
        DeltaOp::block_inserted(png(17).with_observed_at(at(5))),
        DeltaOp::block_started(
            bid(12),
            BlockStart::runtime_tool_call(None, None).with_observed_at(at(3)),
        ),
        DeltaOp::block_started(bid(10), BlockStart::text().with_observed_at(at(2))),
        start_json(13),
        start_provider_call(15),
        start_record(16),
        text(10, "Hello "),
        DeltaOp::thinking_appended(bid(11), "plan"),
        DeltaOp::replay_fragment_appended(bid(11), format(), "Sig"),
        DeltaOp::block_metadata_updated(
            bid(10),
            BlockMetadataUpdate::citation_added(citation(50, 0, 5)),
        ),
        json(12, "{\"query\":"),
        DeltaOp::response_metadata_updated(
            ResponseMetadataUpdate::new().with_usage(Usage::new().with_input_tokens(3)),
        ),
        text(10, "w\u{f6}rld"),
        DeltaOp::block_metadata_updated(
            bid(12),
            BlockMetadataUpdate::tool_name_set(tool("lookup")),
        ),
        DeltaOp::block_metadata_updated(
            bid(12),
            BlockMetadataUpdate::external_id_set(ExternalId::new("toolu_1").unwrap()),
        ),
        DeltaOp::block_metadata_updated(
            bid(10),
            BlockMetadataUpdate::item_ref_set(ItemRef::new("msg_1").unwrap()),
        ),
        json(12, "\"rust\"}"),
        json(13, "{\"a\":[1,"),
        json(15, "{\"q\""),
        json(13, "2]}"),
        DeltaOp::block_metadata_updated(
            bid(16),
            BlockMetadataUpdate::execution_status_set(ExecutionStatus::Running),
        ),
        json(15, ":\"x\"}"),
        DeltaOp::replay_fragment_appended(bid(10), format(), "Ab"),
        DeltaOp::replay_fragment_appended(bid(10), format(), "Cd"),
        finalize(10),
        finalize(11),
        finalize(12),
        finalize(13),
        finalize(15),
        DeltaOp::block_finalized(bid(16), Finalization::authoritative(record.into())),
        DeltaOp::block_inserted(hosted_result),
        DeltaOp::block_inserted(RedactedThinkingBlock::new(bid(19), signature("opaque"))),
        DeltaOp::response_metadata_updated(
            ResponseMetadataUpdate::new()
                .with_usage(Usage::new().with_input_tokens(3).with_output_tokens(9)),
        ),
        DeltaOp::response_completed(ResponseEnding::tool_use()),
    ]
}

pub fn deltas(ops: &[DeltaOp]) -> Vec<ResponseDelta> {
    ops.iter().cloned().map(delta).collect()
}

pub fn draft_after(script: &[ResponseDelta], count: usize) -> ResponseDraft {
    let mut draft = ResponseDraft::new(script[0].turn(), script[0].entry());
    for step in &script[..count] {
        draft.apply(step).unwrap();
    }
    draft
}
