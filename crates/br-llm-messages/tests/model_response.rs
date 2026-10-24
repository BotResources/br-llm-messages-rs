mod common;
use common::Raw as _;

use br_llm_messages::{
    ContentBlock, ErrorCode, ExternalId, ModelName, ModelResponse, ProviderToolCall,
    ProviderToolResult, ResponseEnding, ResponseMetadata, ThinkingBlock, TurnEntry, Usage,
};
use common::{bid, call, cid, eid, plain_text, refused, result, signature, text_block};
use serde_json::json;

fn response(
    blocks: Vec<ContentBlock>,
    ending: ResponseEnding,
) -> Result<ModelResponse, br_llm_messages::Error> {
    ModelResponse::new(eid(1), blocks, ending, ResponseMetadata::new())
}

fn every_ending() -> Vec<ResponseEnding> {
    vec![
        ResponseEnding::finished(),
        ResponseEnding::tool_use(),
        ResponseEnding::provider_continuation(),
        ResponseEnding::output_exhaustion(),
        ResponseEnding::stop_sequence(),
        ResponseEnding::refusal(),
        ResponseEnding::filtering(),
        ResponseEnding::unclassified("vendor_specific").unwrap(),
    ]
}

#[test]
fn given_blocks_in_a_mixed_order_when_built_then_the_reported_order_is_kept() {
    let blocks: Vec<ContentBlock> = vec![
        ThinkingBlock::new(bid(10), "first").unwrap().into(),
        ProviderToolCall::new(bid(11)).into(),
        ProviderToolResult::new(bid(12), bid(11), vec![plain_text(13, "r")])
            .unwrap()
            .into(),
        call(14, "t").into(),
        text_block(15, "tail text").into(),
        call(16, "u").into(),
    ];

    let built = response(blocks.clone(), ResponseEnding::tool_use()).unwrap();

    assert_eq!(built.blocks(), blocks.as_slice());
    let calls: Vec<_> = built.runtime_calls().map(|c| c.id()).collect();
    assert_eq!(calls, vec![bid(14), bid(16)]);
}

#[test]
fn given_a_runtime_call_that_is_not_last_when_ending_requests_results_then_accepted() {
    let blocks = vec![
        call(10, "t").into(),
        text_block(11, "after the call").into(),
    ];

    assert!(response(blocks, ResponseEnding::tool_use()).is_ok());
}

#[test]
fn given_no_blocks_when_any_ending_but_tool_results_requested_then_accepted() {
    for ending in every_ending() {
        let outcome = response(vec![], ending.clone());

        if ending == ResponseEnding::tool_use() {
            assert_eq!(refused(outcome).code(), ErrorCode::InvalidContent);
        } else {
            assert_eq!(outcome.unwrap().ending(), &ending);
        }
    }
}

#[test]
fn given_tool_results_requested_without_a_runtime_call_when_built_then_invalid_content() {
    let blocks = vec![
        text_block(10, "no call here").into(),
        ProviderToolCall::new(bid(11)).into(),
    ];

    let error = refused(response(blocks, ResponseEnding::tool_use()));

    assert_eq!(error.code(), ErrorCode::InvalidContent);
    assert_eq!(error.object_id(), Some(eid(1).as_uuid()));
}

#[test]
fn given_a_runtime_call_when_the_ending_is_finished_then_the_ending_is_not_rewritten() {
    for ending in every_ending() {
        let built = response(vec![call(10, "t").into()], ending.clone()).unwrap();

        assert_eq!(built.ending(), &ending);
    }
}

#[test]
fn given_two_blocks_sharing_an_id_when_built_then_reused_identity_names_it() {
    let blocks = vec![text_block(10, "a").into(), text_block(10, "b").into()];

    let error = refused(response(blocks, ResponseEnding::finished()));

    assert_eq!(error.code(), ErrorCode::ReusedIdentity);
    assert_eq!(error.object_id(), Some(bid(10).as_uuid()));
}

#[test]
fn given_a_block_reusing_the_entry_id_or_a_citation_id_when_built_then_reused_identity() {
    let entry_clash = vec![ContentBlock::from(text_block(1, "same uuid as the entry"))];
    let nested_clash = vec![
        ContentBlock::from(text_block(10, "a")),
        ContentBlock::from(
            ProviderToolResult::new(bid(11), bid(5), vec![plain_text(10, "nested")]).unwrap(),
        ),
    ];

    assert_eq!(
        refused(response(entry_clash, ResponseEnding::finished())).code(),
        ErrorCode::ReusedIdentity
    );
    assert_eq!(
        refused(response(nested_clash, ResponseEnding::finished())).code(),
        ErrorCode::ReusedIdentity
    );
    let citation_clash = br_llm_messages::TextBlock::new(bid(20), "abcd")
        .unwrap()
        .with_citations(vec![br_llm_messages::Citation::new(
            cid(21),
            br_llm_messages::CitationSource::new("doc").unwrap(),
        )])
        .unwrap();
    let blocks = vec![
        ContentBlock::from(citation_clash),
        ContentBlock::from(text_block(21, "reuses the citation id")),
    ];
    assert_eq!(
        refused(response(blocks, ResponseEnding::finished())).code(),
        ErrorCode::ReusedIdentity
    );
}

#[test]
fn given_a_runtime_tool_result_block_when_built_then_disallowed_kind() {
    let blocks = vec![ContentBlock::from(result(10, 11, "t"))];

    let error = refused(response(blocks, ResponseEnding::finished()));

    assert_eq!(error.code(), ErrorCode::DisallowedKind);
    assert_eq!(error.object_id(), Some(bid(10).as_uuid()));
    assert_eq!(error.location(), Some("content[0]"));
}

#[test]
fn given_a_response_json_with_a_runtime_tool_result_block_when_loaded_then_refused() {
    let value = json!({"type": "model_response", "id": eid(1).to_string(), "ending": {"type": "finished"},
        "content": [{"type": "runtime_tool_result", "id": bid(10).to_string(), "call": bid(11).to_string(),
            "tool_name": "t", "is_error": false, "content": []}]});

    let error = refused(TurnEntry::from_value(common::oj(value)));

    assert_eq!(error.code(), ErrorCode::DisallowedKind);
}

#[test]
fn given_a_response_json_without_ending_or_content_when_loaded_then_invalid_content() {
    let full = json!({"type": "model_response", "id": eid(1).to_string(), "ending": {"type": "finished"}, "content": []});
    assert!(TurnEntry::from_value(common::oj(full.clone())).is_ok());

    for field in ["ending", "content"] {
        let mut stripped = full.clone();
        stripped.as_object_mut().unwrap().remove(field);

        assert_eq!(
            refused(TurnEntry::from_value(common::oj(stripped))).code(),
            ErrorCode::InvalidContent,
            "{field}"
        );
    }
}

#[test]
fn given_no_usage_model_or_provider_id_when_built_then_absence_is_retained() {
    let built = response(vec![], ResponseEnding::finished()).unwrap();

    assert_eq!(built.usage(), None);
    assert_eq!(built.model(), None);
    assert_eq!(built.provider_response_id(), None);
    let value = serde_json::to_value(TurnEntry::from(built)).unwrap();
    for key in ["usage", "model", "provider_response_id"] {
        assert!(value.get(key).is_none(), "{key} must stay absent");
    }
}

#[test]
fn given_partial_usage_when_round_tripped_then_missing_counts_stay_absent_not_zero() {
    let metadata = ResponseMetadata::new()
        .with_usage(Usage::new().with_output_tokens(u64::MAX))
        .with_model(ModelName::new("model-x").unwrap())
        .with_provider_response_id(ExternalId::new("resp_1").unwrap());
    let built = ModelResponse::new(eid(1), vec![], ResponseEnding::finished(), metadata).unwrap();

    let entry = TurnEntry::from(built.clone());
    let value = entry.to_value().unwrap().raw();
    let reloaded = TurnEntry::from_value(common::oj(value.clone())).unwrap();

    assert_eq!(
        value["usage"],
        json!({"output_tokens": u64::MAX}),
        "only the observed count is written"
    );
    assert_eq!(reloaded, entry);
    let TurnEntry::ModelResponse(back) = reloaded else {
        panic!("kind changed")
    };
    assert_eq!(back.usage().unwrap().input_tokens(), None);
    assert_eq!(back.usage().unwrap().output_tokens(), Some(u64::MAX));
    assert_eq!(back.metadata(), built.metadata());
}

#[test]
fn given_a_provider_replay_block_when_built_then_replay_material_is_kept_unchanged() {
    let thinking = ThinkingBlock::replay_only(bid(10), signature("opaque=="));

    let built = response(vec![thinking.clone().into()], ResponseEnding::finished()).unwrap();

    assert_eq!(built.blocks(), &[ContentBlock::from(thinking)]);
}
