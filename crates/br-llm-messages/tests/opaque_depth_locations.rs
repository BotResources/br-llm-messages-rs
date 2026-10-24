mod common;
mod stream_support;

use br_llm_messages::{
    AITurn, BlockStart, ContentBlock, Conversation, DeltaOp, ModelResponse, ProviderToolCall,
    ReplayFormat, ReplayMaterial, ReplayPayload, ResponseDelta, ResponseEnding, ResponseMetadata,
    RuntimeToolCall, StructuredBlock, ThinkingBlock,
};
use common::{aid, author, bid, cvid, eid, oj, refused, tool};
use serde_json::{Value, json};
use stream_support::{delta, tid};

const LIMIT: usize = 64;

fn nested(levels: usize) -> Value {
    (0..levels).fold(json!(1), |inner, level| match (levels - 1 - level) % 2 {
        0 => json!({ "k": inner }),
        _ => json!([inner]),
    })
}

fn too_deep(text: &str) -> String {
    let at_limit = serde_json::to_string(&nested(LIMIT)).unwrap();
    assert!(text.contains(&at_limit));
    text.replacen(
        &at_limit,
        &serde_json::to_string(&nested(LIMIT + 1)).unwrap(),
        1,
    )
}

fn location_of<T: std::fmt::Debug>(result: Result<T, br_llm_messages::Error>) -> String {
    let error = refused(result);
    assert_eq!(error.code(), br_llm_messages::ErrorCode::InvalidContent);
    error.location().unwrap().to_owned()
}

fn replay_holding(value: Value) -> ReplayMaterial {
    ReplayMaterial::new(
        ReplayFormat::new("openai.reasoning").unwrap(),
        ReplayPayload::json(oj(value)).unwrap(),
    )
}

#[test]
fn given_a_structured_block_beyond_the_limit_when_loaded_then_the_location_names_value() {
    let block = ContentBlock::from(StructuredBlock::new(bid(1), oj(nested(LIMIT))).unwrap());

    let text = too_deep(&block.to_json().unwrap());

    assert_eq!(location_of(ContentBlock::from_json(&text)), "value");
}

#[test]
fn given_runtime_call_arguments_beyond_the_limit_when_loaded_then_the_location_names_arguments() {
    let block =
        ContentBlock::from(RuntimeToolCall::new(bid(1), tool("t"), oj(nested(LIMIT))).unwrap());

    let text = too_deep(&block.to_json().unwrap());

    assert_eq!(location_of(ContentBlock::from_json(&text)), "arguments");
}

#[test]
fn given_provider_call_arguments_beyond_the_limit_when_loaded_then_the_location_names_arguments() {
    let call = ProviderToolCall::new(bid(1))
        .with_arguments(oj(nested(LIMIT)))
        .unwrap();
    let block = ContentBlock::from(call);

    let text = too_deep(&block.to_json().unwrap());

    assert_eq!(location_of(ContentBlock::from_json(&text)), "arguments");
}

#[test]
fn given_replay_json_beyond_the_limit_when_a_thinking_block_is_loaded_then_the_location_names_value()
 {
    let block = ContentBlock::from(ThinkingBlock::replay_only(
        bid(1),
        replay_holding(nested(LIMIT)),
    ));

    let text = too_deep(&block.to_json().unwrap());

    assert_eq!(location_of(ContentBlock::from_json(&text)), "value");
}

#[test]
fn given_block_start_arguments_beyond_the_limit_when_a_delta_is_loaded_then_the_location_names_arguments()
 {
    let start = BlockStart::runtime_tool_call(Some(tool("t")), None)
        .with_arguments(oj(nested(LIMIT)))
        .unwrap();
    let text = delta(DeltaOp::block_started(bid(1), start))
        .to_json()
        .unwrap();

    assert_eq!(
        location_of(ResponseDelta::from_json(&too_deep(&text))),
        "arguments"
    );
}

#[test]
fn given_a_replay_set_beyond_the_limit_when_a_delta_is_loaded_then_the_location_names_value() {
    let op = DeltaOp::replay_material_set(bid(1), replay_holding(nested(LIMIT)));
    let text = delta(op).to_json().unwrap();

    assert_eq!(
        location_of(ResponseDelta::from_json(&too_deep(&text))),
        "value"
    );
}

#[test]
fn given_a_conversation_holding_a_too_deep_value_when_loaded_then_the_location_leads_to_the_entry()
{
    let call = RuntimeToolCall::new(bid(2), tool("t"), oj(nested(LIMIT))).unwrap();
    let response = ModelResponse::new(
        eid(10),
        vec![call.into()],
        ResponseEnding::tool_use(),
        ResponseMetadata::new(),
    )
    .unwrap();
    let turn =
        AITurn::from_parts(tid(100), aid(1), author("Codex"), vec![response.into()]).unwrap();
    let conversation = Conversation::from_parts(cvid(1000), vec![turn.into()]).unwrap();

    let text = too_deep(&conversation.to_json().unwrap());
    let location = location_of(Conversation::from_json(&text));

    assert!(location.starts_with("turns[0].entries[0]"), "{location}");
    assert!(location.ends_with("arguments"), "{location}");
}
