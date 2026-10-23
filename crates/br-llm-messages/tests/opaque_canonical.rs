mod common;
mod stream_support;

use br_llm_messages::{
    AITurn, ContentBlock, Conversation, ModelResponse, OpaqueJson, ProviderOperation,
    ProviderToolCall, RecordSet, ReplayFormat, ReplayMaterial, ReplayPayload, ResponseEnding,
    ResponseMetadata, RuntimeToolCall, StructuredBlock, ThinkingBlock, TurnEntry,
};
use common::{aid, author, bid, cvid, eid, tool};
use serde_json::Value;
use stream_support::{finalize, json, ok, run, start_call, start_json};

const NASTY: &str = "{\"z\":1,\"a\":[1.0,1e+2,0.1000000000000000055511151231257827,18446744073709551616,-0.0,10e-1],\"m\":{\"y\":1,\"b\":2},\"s\":\"\u{e9}\\n\"}";

const CANONICAL: &str = "{\"a\":[1.0,100.0,0.1,1.8446744073709552e+19,-0.0,1.0],\"m\":{\"b\":2,\"y\":1},\"s\":\"\u{e9}\\n\",\"z\":1}";

fn nasty() -> Value {
    serde_json::from_str(NASTY).unwrap()
}

fn structured() -> ContentBlock {
    StructuredBlock::new(bid(1), common::oj(nasty()))
        .unwrap()
        .into()
}

fn runtime_call() -> ContentBlock {
    RuntimeToolCall::new(bid(2), tool("lookup"), common::oj(nasty()))
        .unwrap()
        .into()
}

fn replay_json() -> ContentBlock {
    let material = ReplayMaterial::new(
        ReplayFormat::new("openai.reasoning").unwrap(),
        ReplayPayload::json(common::oj(nasty())).unwrap(),
    );
    ThinkingBlock::replay_only(bid(3), material).into()
}

fn provider_call() -> ContentBlock {
    ProviderToolCall::new(bid(4))
        .with_operation(ProviderOperation::new("web_search").unwrap())
        .with_arguments(common::oj(nasty()))
        .unwrap()
        .into()
}

fn every_opaque_block() -> Vec<ContentBlock> {
    vec![structured(), runtime_call(), replay_json(), provider_call()]
}

fn response() -> TurnEntry {
    ModelResponse::new(
        eid(10),
        every_opaque_block(),
        ResponseEnding::tool_use(),
        ResponseMetadata::new(),
    )
    .unwrap()
    .into()
}

fn conversation() -> Conversation {
    let turn = AITurn::from_parts(
        common::turns::tid(100),
        aid(1),
        author("Codex"),
        vec![response()],
    )
    .unwrap();
    Conversation::from_parts(cvid(1000), vec![turn.into()]).unwrap()
}

fn written(text: &str) -> String {
    let value = OpaqueJson::parse(text).unwrap();
    ContentBlock::from(StructuredBlock::new(bid(1), value).unwrap())
        .to_json()
        .unwrap()
}

fn stored_value(text: &str) -> String {
    let json = written(text);
    json.split_once("\"value\":")
        .and_then(|(_, tail)| tail.strip_suffix('}'))
        .unwrap()
        .to_owned()
}

#[test]
fn given_integers_up_to_the_64_bit_limits_when_stored_then_they_stay_exact() {
    for integer in [
        "9223372036854775807",
        "-9223372036854775808",
        "18446744073709551615",
        "0",
        "7",
    ] {
        assert_eq!(stored_value(integer), integer);
    }
}

#[test]
fn given_an_integer_above_u64_when_stored_then_it_becomes_a_float() {
    assert_eq!(
        stored_value("18446744073709551616"),
        "1.8446744073709552e+19"
    );
}

#[test]
fn given_non_integer_numbers_when_stored_then_they_are_f64_values() {
    assert_eq!(stored_value("0.1000000000000000055511151231257827"), "0.1");
    assert_eq!(stored_value("1.0"), "1.0");
    assert_eq!(stored_value("-2.5"), "-2.5");
}

#[test]
fn given_differently_spelled_numbers_when_stored_then_the_spelling_is_not_kept() {
    assert_eq!(stored_value("1.50"), "1.5");
    assert_eq!(stored_value("1e2"), "100.0");
    assert_eq!(stored_value("1E2"), "100.0");
    assert_eq!(stored_value("12.50e-3"), "0.0125");
    assert_eq!(stored_value("10e-1"), "1.0");
}

#[test]
fn given_a_negative_zero_when_stored_then_it_is_the_float_negative_zero() {
    assert_eq!(stored_value("-0"), "-0.0");
    assert_eq!(stored_value("-0.0"), "-0.0");
}

#[test]
fn given_objects_in_any_key_order_when_stored_then_keys_are_sorted_and_the_values_are_equal() {
    let first = OpaqueJson::parse("{\"b\":2,\"a\":{\"y\":1,\"x\":0}}").unwrap();
    let second = OpaqueJson::parse("{\"a\":{\"x\":0,\"y\":1},\"b\":2}").unwrap();

    assert_eq!(first, second);
    assert_eq!(
        first.to_json_string().unwrap(),
        "{\"a\":{\"x\":0,\"y\":1},\"b\":2}"
    );
    assert_eq!(written("{\"b\":2,\"a\":1}"), written("{\"a\":1,\"b\":2}"));
}

#[test]
fn given_each_opaque_block_when_round_tripped_then_the_canonical_text_is_stable() {
    for block in every_opaque_block() {
        let text = block.to_json().unwrap();

        let back = ContentBlock::from_json(&text).unwrap();

        assert!(text.contains(CANONICAL), "{text}");
        assert_eq!(back, block);
        assert_eq!(back.to_json().unwrap(), text);
    }
}

#[test]
fn given_opaque_values_in_entries_when_round_tripped_then_the_canonical_text_is_stable() {
    let entry = response();
    let text = entry.to_json().unwrap();

    let back = TurnEntry::from_json(&text).unwrap();

    assert!(text.contains(CANONICAL), "{text}");
    assert_eq!(back, entry);
    assert_eq!(back.to_json().unwrap(), text);
}

#[test]
fn given_a_conversation_with_opaque_values_when_round_tripped_then_the_document_text_is_stable() {
    let original = conversation();
    let text = original.to_json().unwrap();

    let back = Conversation::from_json(&text).unwrap();

    assert_eq!(back, original);
    assert_eq!(back.to_json().unwrap(), text);
    assert_eq!(text.matches(CANONICAL).count(), 4);
}

#[test]
fn given_a_conversation_when_decomposed_into_records_then_the_opaque_values_survive_storage() {
    let original = conversation();
    let stored = RecordSet::from_conversation(&original)
        .unwrap()
        .to_json()
        .unwrap();

    let restored = RecordSet::from_json(&stored)
        .unwrap()
        .into_conversation(original.id())
        .unwrap();

    assert_eq!(restored, original);
    assert_eq!(restored.to_json().unwrap(), original.to_json().unwrap());
}

#[test]
fn given_raw_json_accumulated_by_a_draft_when_finalized_then_the_value_is_the_parsed_json() {
    let (first, second) = NASTY.split_at(17);
    let mut draft = run(vec![
        start_json(20, 0),
        json(20, first),
        json(20, second),
        start_call(21, 1, Some("lookup"), None),
        json(21, NASTY),
    ]);

    ok(&mut draft, finalize(20));
    ok(&mut draft, finalize(21));

    let view = draft.view();
    for id in [20, 21] {
        let block = view.slot(bid(id)).unwrap().final_block().unwrap();
        assert!(block.to_json().unwrap().contains(CANONICAL));
    }
}
