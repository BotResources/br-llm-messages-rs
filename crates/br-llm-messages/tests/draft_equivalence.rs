mod common;
mod stream_support;

use br_llm_messages::{
    BlockMetadataUpdate, BlockStart, ContentBlock, DeltaOp, ExternalId, ModelResponse,
    ProviderToolCall, ReplayFormat, ResponseEnding, ResponseMetadata, ResponseMetadataUpdate,
    RuntimeToolCall, StructuredBlock, TextBlock, ThinkingBlock, Usage,
};
use common::{bid, eid, png, signature, tool};
use serde_json::json;
use stream_support::{citation, finalize, operation, run, start_json, start_thinking};

const TEXT: &str = "Héllo wörld, once more";
const THINKING: &str = "I should look this up";
const CALL_JSON: &str = "{\"query\":\"rust\",\"limit\":3}";
const STRUCTURED_JSON: &str = "{\"a\":[1,2,{\"b\":null}]}";
const PROVIDER_JSON: &str = "{\"q\":\"x\"}";

fn chunks(text: &str, size: usize) -> Vec<String> {
    let characters: Vec<char> = text.chars().collect();
    characters
        .chunks(size.max(1))
        .map(|piece| piece.iter().collect())
        .collect()
}

fn usage() -> Usage {
    Usage::new().with_input_tokens(11).with_output_tokens(7)
}

fn direct() -> ModelResponse {
    let blocks: Vec<ContentBlock> = vec![
        TextBlock::new(bid(10), TEXT)
            .unwrap()
            .with_citations(vec![citation(50, 0, 5)])
            .unwrap()
            .with_replay(signature("AbCdEf"))
            .into(),
        ThinkingBlock::new(bid(11), THINKING)
            .unwrap()
            .with_replay(signature("ThinkSig"))
            .into(),
        RuntimeToolCall::new(
            bid(12),
            tool("lookup"),
            common::oj(json!({"query": "rust", "limit": 3})),
        )
        .unwrap()
        .with_external_id(ExternalId::new("toolu_1").unwrap())
        .into(),
        png(13).into(),
        StructuredBlock::new(bid(14), common::oj(json!({"a": [1, 2, {"b": null}]})))
            .unwrap()
            .into(),
        ProviderToolCall::new(bid(15))
            .with_operation(operation("web_search"))
            .with_arguments(common::oj(json!({"q": "x"})))
            .unwrap()
            .into(),
    ];
    let metadata = ResponseMetadata::new()
        .with_usage(usage())
        .with_model(br_llm_messages::ModelName::new("model-x").unwrap());
    ModelResponse::new(eid(2), blocks, ResponseEnding::tool_use(), metadata).unwrap()
}

fn declarations() -> Vec<DeltaOp> {
    vec![
        DeltaOp::block_started(bid(10), BlockStart::text()),
        start_thinking(11),
        DeltaOp::block_started(
            bid(12),
            BlockStart::runtime_tool_call(
                Some(tool("lookup")),
                Some(ExternalId::new("toolu_1").unwrap()),
            ),
        ),
        DeltaOp::block_inserted(png(13)),
        start_json(14),
        DeltaOp::block_started(
            bid(15),
            BlockStart::provider_tool_call(Some(operation("web_search")), None),
        ),
    ]
}

fn per_block(size: usize) -> Vec<Vec<DeltaOp>> {
    let format = ReplayFormat::new("anthropic.signature").unwrap();
    let mut text_ops: Vec<DeltaOp> = chunks(TEXT, size)
        .into_iter()
        .map(|piece| DeltaOp::text_appended(bid(10), piece))
        .collect();
    text_ops.push(DeltaOp::block_metadata_updated(
        bid(10),
        BlockMetadataUpdate::citation_added(citation(50, 0, 5)),
    ));
    for piece in chunks("AbCdEf", size) {
        text_ops.push(DeltaOp::replay_fragment_appended(
            bid(10),
            format.clone(),
            piece,
        ));
    }
    text_ops.push(finalize(10));
    let mut thinking_ops: Vec<DeltaOp> = chunks(THINKING, size)
        .into_iter()
        .map(|piece| DeltaOp::thinking_appended(bid(11), piece))
        .collect();
    for piece in chunks("ThinkSig", size) {
        thinking_ops.push(DeltaOp::replay_fragment_appended(
            bid(11),
            format.clone(),
            piece,
        ));
    }
    thinking_ops.push(finalize(11));
    let json_ops = |block: u64, source: &str| -> Vec<DeltaOp> {
        let mut ops: Vec<DeltaOp> = chunks(source, size)
            .into_iter()
            .map(|piece| DeltaOp::json_appended(bid(block), piece))
            .collect();
        ops.push(finalize(block));
        ops
    };
    vec![
        text_ops,
        thinking_ops,
        json_ops(12, CALL_JSON),
        json_ops(14, STRUCTURED_JSON),
        json_ops(15, PROVIDER_JSON),
    ]
}

fn sequential(groups: Vec<Vec<DeltaOp>>) -> Vec<DeltaOp> {
    groups.into_iter().flatten().collect()
}

fn round_robin(groups: Vec<Vec<DeltaOp>>) -> Vec<DeltaOp> {
    let mut queues: Vec<_> = groups.into_iter().map(|g| g.into_iter()).collect();
    let mut ordered = Vec::new();
    loop {
        let mut progressed = false;
        for queue in &mut queues {
            if let Some(op) = queue.next() {
                ordered.push(op);
                progressed = true;
            }
        }
        if !progressed {
            return ordered;
        }
    }
}

fn tail() -> Vec<DeltaOp> {
    vec![
        DeltaOp::response_metadata_updated(
            ResponseMetadataUpdate::new().with_usage(Usage::new().with_input_tokens(1)),
        ),
        DeltaOp::response_metadata_updated(
            ResponseMetadataUpdate::new()
                .with_usage(usage())
                .with_model(br_llm_messages::ModelName::new("model-x").unwrap()),
        ),
        DeltaOp::response_completed(ResponseEnding::tool_use()),
    ]
}

fn streamed(script: Vec<DeltaOp>) -> ModelResponse {
    let mut ops = declarations();
    ops.extend(script);
    ops.extend(tail());
    run(ops).into_response().unwrap()
}

#[test]
fn given_a_streamed_response_when_completed_then_it_equals_the_directly_built_one() {
    let streamed = streamed(sequential(per_block(4)));

    assert_eq!(streamed, direct());
}

#[test]
fn given_any_fragment_splitting_when_streamed_then_the_result_is_identical() {
    for size in [1, 2, 3, 5, 8, 1000] {
        assert_eq!(
            streamed(sequential(per_block(size))),
            direct(),
            "size {size}"
        );
    }
}

#[test]
fn given_blocks_arriving_interleaved_when_streamed_then_content_order_is_unchanged() {
    for size in [1, 4, 1000] {
        assert_eq!(
            streamed(round_robin(per_block(size))),
            direct(),
            "size {size}"
        );
    }
}

#[test]
fn given_blocks_arriving_in_reverse_when_streamed_then_positions_decide_the_order() {
    let mut groups = per_block(3);
    groups.reverse();

    assert_eq!(streamed(sequential(groups)), direct());
}

#[test]
fn given_coalesced_and_split_fragments_when_streamed_then_exact_text_is_the_same() {
    let split = streamed(sequential(per_block(1)));
    let coalesced = streamed(sequential(per_block(10_000)));

    assert_eq!(split.blocks(), coalesced.blocks());
    assert_eq!(split.usage(), coalesced.usage());
    assert_eq!(split.id(), eid(2));
}
