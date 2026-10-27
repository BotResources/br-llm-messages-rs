mod common;

use br_llm_messages::{
    ContentBlock, ErrorCode, ModelResponse, ResponseEnding, ResponseMetadata, TurnEntry, Usage,
};
use common::turns::{hosted_call, hosted_result};
use common::{bid, eid, refused};
use serde_json::json;

fn built(blocks: Vec<ContentBlock>) -> Result<ModelResponse, br_llm_messages::Error> {
    ModelResponse::new(
        eid(1),
        blocks,
        ResponseEnding::finished(),
        ResponseMetadata::new(),
    )
}

#[test]
fn given_a_provider_result_after_its_call_in_one_response_when_built_then_accepted() {
    let response = built(vec![hosted_call(11), hosted_result(12, 11)]).unwrap();

    assert_eq!(response.blocks().len(), 2);
}

#[test]
fn given_a_provider_result_before_its_call_in_one_response_when_built_then_unknown_reference() {
    let error = refused(built(vec![hosted_result(11, 12), hosted_call(12)]));

    assert_eq!(error.code(), ErrorCode::UnknownReference);
    assert_eq!(error.object_id(), Some(bid(12).as_uuid()));
    assert_eq!(error.location(), Some("content[0].call"));
}

#[test]
fn given_two_provider_results_for_one_call_in_one_response_when_built_then_duplicate_result() {
    let error = refused(built(vec![
        hosted_call(11),
        hosted_result(12, 11),
        hosted_result(13, 11),
    ]));

    assert_eq!(error.code(), ErrorCode::DuplicateResult);
    assert_eq!(error.object_id(), Some(bid(11).as_uuid()));
    assert_eq!(error.location(), Some("content[2].call"));
}

#[test]
fn given_a_provider_result_for_a_call_outside_the_response_when_built_then_the_link_waits_for_the_turn()
 {
    let response = built(vec![hosted_result(12, 99)]).unwrap();

    assert_eq!(response.blocks().len(), 1);
}

#[test]
fn given_a_single_observed_count_when_a_response_is_built_then_the_usage_is_accepted() {
    let usage = Usage::of_thinking_tokens(0);

    let response = ModelResponse::new(
        eid(1),
        vec![],
        ResponseEnding::finished(),
        ResponseMetadata::new().with_usage(usage.clone()),
    )
    .unwrap();

    assert_eq!(response.usage(), Some(&usage));
}

#[test]
fn given_an_empty_usage_in_json_when_a_response_is_loaded_then_invalid_content() {
    let wire = json!({"type": "model_response", "id": eid(1).to_string(), "ending": {"type": "finished"},
        "content": [], "usage": {}});

    assert_eq!(
        common::code_of::<TurnEntry>(&wire.to_string()),
        ErrorCode::InvalidContent
    );
}
