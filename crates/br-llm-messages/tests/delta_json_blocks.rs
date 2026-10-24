mod common;
use common::Raw as _;
mod stream_support;

use br_llm_messages::{
    ContentBlock, DeltaOp, ErrorCode, ExecutionStatus, ExternalId, Finalization,
    ProviderToolExecutionRecord, ProviderToolResult, RedactedThinkingBlock, ResponseDelta,
    RuntimeToolCall, TextBlock,
};
use common::{bid, envelope, plain_text, png, signature, tool};
use serde_json::json;
use stream_support::{citation, delta, operation};

fn every_block() -> Vec<ContentBlock> {
    vec![
        png(10).into(),
        RedactedThinkingBlock::new(bid(11), signature("opaque")).into(),
        ProviderToolResult::new(bid(12), bid(99), vec![plain_text(13, "nested")])
            .unwrap()
            .with_external_id(ExternalId::new("srv_1").unwrap())
            .with_replay(envelope(json!({"enc": [1, 2]})))
            .into(),
        ProviderToolExecutionRecord::new(bid(14))
            .with_content(vec![plain_text(15, "out")])
            .unwrap()
            .with_replay(signature("s"))
            .with_operation(operation("code_run"))
            .with_status(ExecutionStatus::Completed)
            .into(),
        ProviderToolExecutionRecord::new(bid(16)).into(),
        RuntimeToolCall::new(
            bid(17),
            tool("lookup"),
            common::oj(json!({"q": [1, {"z": null}]})),
        )
        .unwrap()
        .with_external_id(ExternalId::new("toolu_1").unwrap())
        .into(),
        TextBlock::new(bid(18), "cited text")
            .unwrap()
            .with_citations(vec![citation(50, 0, 5)])
            .unwrap()
            .with_replay(signature("sig"))
            .into(),
    ]
}

#[test]
fn given_every_block_kind_when_inserted_or_authoritatively_finalized_then_json_round_trips() {
    for block in every_block() {
        let inserted = delta(DeltaOp::block_inserted(block.clone()));
        let finalized = delta(DeltaOp::block_finalized(
            block.id(),
            Finalization::authoritative(block.clone()),
        ));

        for original in [inserted, finalized] {
            let text = original.to_json().unwrap();
            assert_eq!(ResponseDelta::from_json(&text).unwrap(), original);
        }
    }
}

#[test]
fn given_an_inserted_blank_text_block_when_loaded_then_invalid_content_survives_buffering() {
    let mut value = delta(DeltaOp::block_inserted(png(10)))
        .to_value()
        .unwrap()
        .raw();
    value["block"] = json!(bid(11).to_string());
    value["content"] = json!({"type": "text", "id": bid(11).to_string(), "text": "  "});

    let error = ResponseDelta::from_value(common::oj(value)).unwrap_err();

    assert_eq!(error.code(), ErrorCode::InvalidContent);
}

#[test]
fn given_an_inserted_block_with_a_disallowed_nested_kind_when_loaded_then_disallowed_kind() {
    let mut value = delta(DeltaOp::block_inserted(png(10)))
        .to_value()
        .unwrap()
        .raw();
    value["block"] = json!(bid(11).to_string());
    value["content"] = json!({
        "type": "provider_tool_result",
        "id": bid(11).to_string(),
        "call": bid(12).to_string(),
        "content": [{"type": "thinking", "id": bid(13).to_string(), "text": "x"}]
    });

    let error = ResponseDelta::from_value(common::oj(value)).unwrap_err();

    assert_eq!(error.code(), ErrorCode::DisallowedKind);
}
