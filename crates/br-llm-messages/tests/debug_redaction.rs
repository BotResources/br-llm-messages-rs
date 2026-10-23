mod common;
mod stream_support;

use br_llm_messages::{
    AITurn, ContentBlock, Conversation, DeltaOp, ModelResponse, ProviderToolCall,
    RedactedThinkingBlock, ReplayFormat, ReplayMaterial, ReplayPayload, ResponseEnding,
    ResponseMetadata, RuntimeToolCall, ThinkingBlock, TurnEntry,
};
use common::{aid, author, bid, cvid, eid, tool};
use serde_json::json;
use stream_support::{delta, draft};

const SENTINEL: &str = "SENTINEL_REPLAY_SECRET";

fn text_material() -> ReplayMaterial {
    common::signature(SENTINEL)
}

fn json_material() -> ReplayMaterial {
    ReplayMaterial::new(
        ReplayFormat::new("openai.reasoning").unwrap(),
        ReplayPayload::json(json!({"encrypted": SENTINEL, "n": 7})).unwrap(),
    )
}

fn blocks() -> Vec<ContentBlock> {
    vec![
        ThinkingBlock::new(bid(1), "", Some(text_material())).into(),
        RedactedThinkingBlock::new(bid(2), json_material()).into(),
        RuntimeToolCall::new(bid(3), tool("t"), json!({}))
            .unwrap()
            .with_replay(json_material())
            .into(),
        ProviderToolCall::new(bid(4))
            .with_replay(text_material())
            .into(),
    ]
}

fn response() -> ModelResponse {
    ModelResponse::new(
        eid(10),
        blocks(),
        ResponseEnding::ToolResultsRequested,
        ResponseMetadata::new(),
    )
    .unwrap()
}

fn conversation() -> Conversation {
    let turn = AITurn::from_parts(
        common::turns::tid(100),
        aid(1),
        author("Codex"),
        vec![response().into()],
    )
    .unwrap();
    Conversation::from_parts(cvid(1000), vec![turn.into()]).unwrap()
}

#[test]
fn given_replay_material_when_debug_printed_then_the_payload_is_redacted_and_the_format_stays() {
    for material in [text_material(), json_material()] {
        let printed = format!("{material:?}");

        assert!(!printed.contains(SENTINEL), "{printed}");
        assert!(printed.contains("<redacted>"), "{printed}");
        assert!(printed.contains(material.format().as_str()), "{printed}");
    }
}

#[test]
fn given_replay_material_nested_in_every_container_when_debug_printed_then_no_payload_leaks() {
    let entry: TurnEntry = response().into();
    let printed = [
        format!("{:?}", blocks()),
        format!("{:?}", response()),
        format!("{entry:?}"),
        format!("{:#?}", conversation()),
    ];

    for text in printed {
        assert!(!text.contains(SENTINEL), "{text}");
        assert!(text.contains("<redacted>"));
    }
}

#[test]
fn given_replay_material_in_a_draft_when_debug_printed_then_no_payload_leaks() {
    let mut open = draft();
    open.apply(&delta(DeltaOp::block_started(
        bid(5),
        0,
        br_llm_messages::BlockStart::thinking(),
    )))
    .unwrap();
    open.apply(&delta(DeltaOp::replay_material_set(
        bid(5),
        text_material(),
    )))
    .unwrap();

    let printed = format!("{open:?} {:?}", open.view());

    assert!(!printed.contains(SENTINEL), "{printed}");
}

#[test]
fn given_an_error_when_displayed_or_debug_printed_then_it_carries_no_payload() {
    let error = ReplayFormat::new(SENTINEL).unwrap_err();

    assert!(!format!("{error} {error:?}").contains(SENTINEL));
}
