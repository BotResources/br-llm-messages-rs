mod common;
use common::Raw as _;

use br_llm_messages::{Conversation, ErrorCode, Turn};
use common::conversations::rich;
use common::refused;
use serde_json::{Value, json};

const GOLDEN: &str = include_str!("golden/rich_conversation.json");

fn golden() -> Value {
    serde_json::from_str(GOLDEN).unwrap()
}

fn code_of(document: Value) -> ErrorCode {
    refused(Conversation::from_value(common::oj(document))).code()
}

#[test]
fn given_the_current_schema_when_serialized_then_it_names_br_llm_messages_5_exactly() {
    let json = rich().to_value().unwrap().raw();

    assert_eq!(json["schema"], "br-llm-messages/5");
    assert_eq!(json["turns"][0]["type"], "input_turn");
    assert_eq!(json["turns"][1]["type"], "ai_turn");
}

#[test]
fn given_other_schemas_when_loaded_then_schema_mismatch_before_any_shape_error() {
    let broken_shape = json!({"turns": "not a list", "id": 7, "extra": true});
    let documents = [
        json!({"schema": "br-llm-messages/3", "turns": "not a list"}),
        json!({"schema": "br-llm-messages/4 ", "turns": "not a list"}),
        json!({"schema": "BR-LLM-MESSAGES/4", "turns": 1}),
        json!({"schema": "br-llm-messages-delta/1"}),
        json!({"schema": 4, "turns": []}),
        json!({"schema": null}),
        broken_shape,
        json!([]),
        json!("br-llm-messages/4"),
        json!(null),
    ];

    for document in documents {
        let error = refused(Conversation::from_value(common::oj(document.clone())));
        assert_eq!(error.code(), ErrorCode::SchemaMismatch, "{document}");
        assert_eq!(error.location(), Some("schema"));
    }
}

#[test]
fn given_text_with_a_wrong_schema_when_loaded_then_schema_mismatch() {
    let text = r#"{"schema":"br-llm-messages/3","id":"x","turns":[{"nonsense":1}]}"#;

    assert_eq!(
        refused(Conversation::from_json(text)).code(),
        ErrorCode::SchemaMismatch
    );
}

#[test]
fn given_a_rich_conversation_when_serialized_then_it_matches_the_golden_document() {
    let json = rich().to_value().unwrap().raw();

    assert_eq!(json, golden());
}

#[test]
fn given_the_golden_document_when_loaded_then_it_builds_the_exact_conversation() {
    let loaded = Conversation::from_json(GOLDEN).unwrap();

    assert_eq!(loaded, rich());
    assert_eq!(loaded.to_value().unwrap().raw(), golden());
}

#[test]
fn given_a_rich_conversation_when_round_tripped_as_text_then_nothing_is_lost() {
    let original = rich();

    let reloaded = Conversation::from_json(&original.to_json().unwrap()).unwrap();

    assert_eq!(reloaded, original);
    assert_eq!(reloaded.to_json().unwrap(), original.to_json().unwrap());
    let response = reloaded.turns()[1].as_ai().unwrap().entries()[0]
        .to_value()
        .unwrap()
        .raw();
    assert_eq!(response["usage"]["output_tokens"], u64::MAX);
}

#[test]
fn given_an_empty_conversation_when_round_tripped_then_it_stays_empty() {
    let empty = Conversation::new(common::cvid(1));

    let reloaded = Conversation::from_json(&empty.to_json().unwrap()).unwrap();

    assert_eq!(reloaded, empty);
    assert!(reloaded.turns().is_empty());
}

#[test]
fn given_a_conversation_inside_a_serde_container_when_decoded_then_the_same_rules_apply() {
    let wrapper = json!({"conversation": golden()});

    #[derive(serde::Deserialize)]
    struct Holder {
        conversation: Conversation,
    }
    let holder: Holder = common::oj(wrapper).deserialize_into().unwrap();

    assert_eq!(holder.conversation, rich());
    let mut bad = golden();
    bad["schema"] = json!("br-llm-messages/3");
    let failure = refused(common::oj(json!({"conversation": bad})).deserialize_into::<Holder>());
    assert_eq!(failure.code(), ErrorCode::SchemaMismatch);
}

#[test]
fn given_an_unknown_variant_anywhere_when_loaded_then_variant_mismatch() {
    let mut turn_kind = golden();
    turn_kind["turns"][0]["type"] = json!("system_turn");
    let mut entry_kind = golden();
    entry_kind["turns"][1]["entries"][0]["type"] = json!("system_instruction");
    let mut block_kind = golden();
    block_kind["turns"][0]["entries"][0]["content"][0]["type"] = json!("audio");
    let mut closure_kind = golden();
    closure_kind["turns"][1]["entries"][4]["reason"]["type"] = json!("paused");

    for document in [turn_kind, entry_kind, block_kind, closure_kind] {
        assert_eq!(code_of(document), ErrorCode::VariantMismatch);
    }
}

#[test]
fn given_an_unknown_field_anywhere_when_loaded_then_invalid_content() {
    let mut top = golden();
    top["revision"] = json!(3);
    let mut turn = golden();
    turn["turns"][1]["color"] = json!("red");
    let mut input_entry = golden();
    input_entry["turns"][0]["entries"][0]["priority"] = json!(1);
    let mut ai_entry = golden();
    ai_entry["turns"][1]["entries"][0]["note"] = json!("x");
    let mut nested_block = golden();
    nested_block["turns"][1]["entries"][1]["results"][0]["content"][1]["alt"] = json!("x");

    for document in [top, turn, input_entry, ai_entry, nested_block] {
        assert_eq!(code_of(document), ErrorCode::InvalidContent);
    }
}

#[test]
fn given_truncated_or_invalid_text_when_loaded_then_malformed_json() {
    let cut = &GOLDEN[..GOLDEN.len() / 2];

    for text in [cut, "", "{", "not json", "{\"schema\": }"] {
        assert_eq!(
            refused(Conversation::from_json(text)).code(),
            ErrorCode::MalformedJson,
            "{text}"
        );
    }
}

#[test]
fn given_old_zero_x_data_when_loaded_then_schema_mismatch_and_nothing_is_reinterpreted() {
    let old = json!({
        "schema": "br-llm-messages/3",
        "id": "ab000000-0000-7000-8000-000000000001",
        "turns": [{
            "id": "ab000000-0000-7000-8000-000000000002",
            "author": {"id": "ab000000-0000-7000-8000-000000000003", "name": "Codex"},
            "items": [{"type": "step"}]
        }]
    });
    let older = json!({"id": "ab000000-0000-7000-8000-000000000001", "turns": []});

    assert_eq!(code_of(old), ErrorCode::SchemaMismatch);
    assert_eq!(code_of(older), ErrorCode::SchemaMismatch);
}

#[test]
fn given_a_known_payload_that_is_invalid_when_loaded_then_it_is_refused_with_its_location() {
    let mut document = golden();
    document["turns"][1]["entries"][2]["author"] = json!("   ");

    let error = refused(Conversation::from_value(common::oj(document)));

    assert_eq!(error.code(), ErrorCode::InvalidAuthor);
    assert!(error.location().unwrap().starts_with("turns[1].entries[2]"));
}

#[test]
fn given_each_turn_loaded_alone_when_compared_then_it_equals_the_conversation_turn() {
    let conversation = rich();

    for turn in conversation.turns() {
        let alone = Turn::from_json(&turn.to_json().unwrap()).unwrap();
        assert_eq!(&alone, turn);
    }
}
