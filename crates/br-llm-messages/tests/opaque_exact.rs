mod common;
mod stream_support;

use br_llm_messages::{
    AITurn, ContentBlock, Conversation, ModelResponse, ProviderOperation, ProviderToolCall,
    RecordSet, ReplayFormat, ReplayMaterial, ReplayPayload, ResponseEnding, ResponseMetadata,
    RuntimeToolCall, StructuredBlock, ThinkingBlock, ToolDefinition, ToolMounted, TurnEntry,
};
use common::{aid, author, bid, cvid, eid, tool};
use serde_json::Value;
use stream_support::{finalize, json, ok, run, start_call, start_json};

const NASTY: &str = "{\"z\":1,\"a\":[1.0,1e+2,0.1000000000000000055511151231257827,18446744073709551616,-0.0,10e-1],\"m\":{\"y\":1,\"b\":2},\"s\":\"\u{e9}\\n\"}";

fn nasty() -> Value {
    serde_json::from_str(NASTY).unwrap()
}

fn structured() -> ContentBlock {
    StructuredBlock::new(bid(1), nasty()).unwrap().into()
}

fn runtime_call() -> ContentBlock {
    RuntimeToolCall::new(bid(2), tool("lookup"), nasty())
        .unwrap()
        .into()
}

fn replay_json() -> ContentBlock {
    let material = ReplayMaterial::new(
        ReplayFormat::new("openai.reasoning").unwrap(),
        ReplayPayload::json(nasty()).unwrap(),
    );
    ThinkingBlock::new(bid(3), "", Some(material)).into()
}

fn provider_call() -> ContentBlock {
    ProviderToolCall::new(bid(4))
        .with_operation(ProviderOperation::new("web_search").unwrap())
        .with_arguments(nasty())
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
        ResponseEnding::ToolResultsRequested,
        ResponseMetadata::new(),
    )
    .unwrap()
    .into()
}

fn mounting() -> TurnEntry {
    let definition = ToolDefinition::inline(tool("lookup"), "finds", nasty()).unwrap();
    ToolMounted::new(eid(11), definition).into()
}

fn conversation() -> Conversation {
    let turn = AITurn::from_parts(
        common::turns::tid(100),
        aid(1),
        author("Codex"),
        vec![mounting(), response()],
    )
    .unwrap();
    Conversation::from_parts(cvid(1000), vec![turn.into()]).unwrap()
}

#[test]
fn given_a_value_parsed_from_text_when_serialized_then_keys_and_numbers_keep_their_text() {
    assert_eq!(serde_json::to_string(&nasty()).unwrap(), NASTY);
}

#[test]
fn given_each_opaque_block_when_round_tripped_then_the_opaque_text_is_byte_exact() {
    for block in every_opaque_block() {
        let text = block.to_json().unwrap();

        let back = ContentBlock::from_json(&text).unwrap();

        assert!(text.contains(NASTY), "{text}");
        assert_eq!(back, block);
        assert_eq!(back.to_json().unwrap(), text);
    }
}

#[test]
fn given_two_objects_differing_only_in_key_order_when_compared_then_they_are_distinct_values() {
    let written = |text: &str| {
        let value = serde_json::from_str(text).unwrap();
        ContentBlock::from(StructuredBlock::new(bid(1), value).unwrap())
            .to_json()
            .unwrap()
    };

    assert_ne!(written("{\"a\":1,\"b\":2}"), written("{\"b\":2,\"a\":1}"));
}

#[test]
fn given_exponent_spellings_when_stored_then_digits_and_magnitude_are_kept_with_a_canonical_exponent()
 {
    let written = |text: &str| {
        let value = serde_json::from_str(text).unwrap();
        ContentBlock::from(StructuredBlock::new(bid(1), value).unwrap())
            .to_json()
            .unwrap()
    };

    assert!(written("1E2").ends_with("\"value\":1e+2}"));
    assert!(written("12.50e-3").ends_with("\"value\":12.50e-3}"));
}

#[test]
fn given_numbers_with_the_same_magnitude_when_stored_then_their_written_forms_stay_distinct() {
    let forms = ["1", "1.0", "1e+0", "10e-1"];

    let texts: Vec<String> = forms
        .iter()
        .map(|form| {
            let value = serde_json::from_str(form).unwrap();
            ContentBlock::from(StructuredBlock::new(bid(1), value).unwrap())
                .to_json()
                .unwrap()
        })
        .collect();

    for (form, text) in forms.iter().zip(&texts) {
        assert!(text.ends_with(&format!("\"value\":{form}}}")), "{text}");
    }
}

#[test]
fn given_opaque_values_in_entries_when_round_tripped_then_the_entry_text_is_exact() {
    for entry in [response(), mounting()] {
        let text = entry.to_json().unwrap();

        let back = TurnEntry::from_json(&text).unwrap();

        assert!(text.contains(NASTY), "{text}");
        assert_eq!(back, entry);
        assert_eq!(back.to_json().unwrap(), text);
    }
}

#[test]
fn given_a_conversation_with_opaque_values_when_round_tripped_then_the_document_text_is_exact() {
    let original = conversation();
    let text = original.to_json().unwrap();

    let back = Conversation::from_json(&text).unwrap();

    assert_eq!(back, original);
    assert_eq!(back.to_json().unwrap(), text);
    assert_eq!(text.matches(NASTY).count(), 5);
}

#[test]
fn given_a_conversation_when_decomposed_into_records_then_the_opaque_text_survives_storage() {
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
fn given_raw_json_accumulated_by_a_draft_when_finalized_then_the_opaque_text_is_exact() {
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
        assert!(block.to_json().unwrap().contains(NASTY));
    }
}

#[test]
fn given_a_negative_zero_integer_when_stored_then_the_sign_is_not_kept() {
    let written = |text: &str| {
        let value = serde_json::from_str(text).unwrap();
        ContentBlock::from(StructuredBlock::new(bid(1), value).unwrap())
            .to_json()
            .unwrap()
    };

    assert!(written("-0").ends_with("\"value\":0}"), "{}", written("-0"));
    assert!(written("-0.0").ends_with("\"value\":-0.0}"));
}
