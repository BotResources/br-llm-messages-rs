mod common;
mod stream_support;

use br_llm_messages::{
    AITurn, ContentBlock, Conversation, ErrorCode, ModelResponse, ProviderToolCall, RecordSet,
    ReplayFormat, ReplayMaterial, ReplayPayload, ResponseEnding, ResponseMetadata, RuntimeToolCall,
    StructuredBlock, ThinkingBlock, ToolDefinition, ToolMounted,
};
use common::{aid, author, bid, cvid, eid, refused, tool};
use serde_json::{Value, json};
use stream_support::{finalize, json as raw_json, ok, refuse, run, start_json};

const LIMIT: usize = 64;

fn nested(levels: usize) -> Value {
    (0..levels).fold(json!(1), |inner, level| match (levels - 1 - level) % 2 {
        0 => json!({ "k": inner }),
        _ => json!([inner]),
    })
}

fn raw_nested(levels: usize) -> String {
    format!("{}1{}", "[".repeat(levels), "]".repeat(levels))
}

fn every_constructor(value: &Value) -> Vec<Result<(), br_llm_messages::Error>> {
    vec![
        StructuredBlock::new(bid(1), common::oj(value.clone())).map(drop),
        RuntimeToolCall::new(bid(2), tool("t"), common::oj(value.clone())).map(drop),
        ProviderToolCall::new(bid(3))
            .with_arguments(common::oj(value.clone()))
            .map(drop),
        ReplayPayload::json(common::oj(value.clone())).map(drop),
        ToolDefinition::inline(tool("t"), "d", common::oj(value.clone())).map(drop),
    ]
}

fn conversation_holding(value: &Value) -> Conversation {
    let material = ReplayMaterial::new(
        ReplayFormat::new("openai.reasoning").unwrap(),
        ReplayPayload::json(common::oj(value.clone())).unwrap(),
    );
    let blocks: Vec<ContentBlock> = vec![
        StructuredBlock::new(bid(1), common::oj(value.clone()))
            .unwrap()
            .into(),
        RuntimeToolCall::new(bid(2), tool("t"), common::oj(value.clone()))
            .unwrap()
            .into(),
        ProviderToolCall::new(bid(3))
            .with_arguments(common::oj(value.clone()))
            .unwrap()
            .into(),
        ThinkingBlock::new(bid(4), "").with_replay(material).into(),
    ];
    let response = ModelResponse::new(
        eid(10),
        blocks,
        ResponseEnding::tool_results_requested(),
        ResponseMetadata::new(),
    )
    .unwrap();
    let definition = ToolDefinition::inline(tool("t"), "d", common::oj(value.clone())).unwrap();
    let turn = AITurn::from_parts(
        common::turns::tid(100),
        aid(1),
        author("Codex"),
        vec![
            ToolMounted::new(eid(11), definition).into(),
            response.into(),
        ],
    )
    .unwrap();
    Conversation::from_parts(cvid(1000), vec![turn.into()]).unwrap()
}

#[test]
fn given_a_value_at_the_depth_limit_when_stored_everywhere_then_every_constructor_accepts_it() {
    let value = nested(LIMIT);

    let results = every_constructor(&value);

    assert!(results.iter().all(Result::is_ok), "{results:?}");
}

#[test]
fn given_a_value_beyond_the_depth_limit_when_stored_then_every_constructor_refuses_it() {
    let value = nested(LIMIT + 1);

    let results = every_constructor(&value);

    for result in results {
        assert_eq!(refused(result).code(), ErrorCode::InvalidContent);
    }
}

#[test]
fn given_a_value_at_the_depth_limit_when_a_conversation_is_saved_then_it_loads_back_equal() {
    let original = conversation_holding(&nested(LIMIT));

    let text = original.to_json().unwrap();
    let back = Conversation::from_json(&text).unwrap();
    let records = RecordSet::from_conversation(&original)
        .unwrap()
        .to_json()
        .unwrap();
    let restored = RecordSet::from_json(&records)
        .unwrap()
        .into_conversation(original.id())
        .unwrap();

    assert_eq!(back, original);
    assert_eq!(restored, original);
}

#[test]
fn given_a_document_whose_opaque_value_is_beyond_the_limit_when_loaded_then_it_is_refused() {
    let block =
        ContentBlock::from(StructuredBlock::new(bid(1), common::oj(nested(LIMIT))).unwrap());
    let text = block.to_json().unwrap();
    let too_deep = text.replacen(
        &serde_json::to_string(&nested(LIMIT)).unwrap(),
        &serde_json::to_string(&nested(LIMIT + 1)).unwrap(),
        1,
    );

    assert!(ContentBlock::from_json(&text).is_ok());
    assert_eq!(
        refused(ContentBlock::from_json(&too_deep)).code(),
        ErrorCode::InvalidContent
    );
}

#[test]
fn given_raw_json_beyond_the_depth_limit_when_a_draft_block_is_finalized_then_it_is_refused() {
    let mut accepted = run(vec![start_json(20, 0), raw_json(20, &raw_nested(LIMIT))]);
    let mut refused_draft = run(vec![
        start_json(21, 0),
        raw_json(21, &raw_nested(LIMIT + 1)),
    ]);

    ok(&mut accepted, finalize(20));
    let error = refuse(&mut refused_draft, finalize(21));

    assert_eq!(error.code(), ErrorCode::InvalidContent);
}
