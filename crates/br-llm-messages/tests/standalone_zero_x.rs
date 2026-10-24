mod common;

use br_llm_messages::{AITurn, ContentBlock, ErrorCode, IncomingEntry, InputTurn, Turn, TurnEntry};
use common::{code_of, refused};
use serde_json::{Value, json};

const ID: &str = "ab000000-0000-7000-8000-000000000001";

fn zero_x_turn() -> Value {
    json!({
        "id": ID,
        "author": {"id": ID, "name": "Codex"},
        "items": [
            {"type": "step", "content": [{"type": "text", "text": "hi"}], "stop_reason": "end_turn"},
            {"type": "tool_results", "results": []}
        ]
    })
}

#[test]
fn given_a_standalone_zero_x_turn_when_loaded_then_variant_mismatch_and_never_a_schema_error() {
    let document = zero_x_turn().to_string();

    assert_eq!(code_of::<Turn>(&document), ErrorCode::VariantMismatch);
    assert_eq!(code_of::<AITurn>(&document), ErrorCode::VariantMismatch);
    assert_eq!(code_of::<InputTurn>(&document), ErrorCode::VariantMismatch);
    assert_eq!(refused(Turn::from_json(&document)).location(), Some("type"));
}

#[test]
fn given_standalone_zero_x_turn_items_when_loaded_as_entries_then_variant_mismatch() {
    for item in zero_x_turn()["items"].as_array().unwrap() {
        let document = item.to_string();

        assert_eq!(code_of::<TurnEntry>(&document), ErrorCode::VariantMismatch);
        assert_eq!(
            refused(IncomingEntry::from_json(&document)).code(),
            ErrorCode::VariantMismatch
        );
    }
}

#[test]
fn given_standalone_zero_x_blocks_of_a_retired_kind_when_loaded_then_variant_mismatch() {
    for block in [
        json!({"type": "structured", "value": {"a": 1}}),
        json!({"type": "tool_call", "id": "call_1", "name": "lookup", "arguments": {}}),
        json!({"type": "tool_result", "tool_call_id": "call_1", "tool_name": "lookup", "content": [], "is_error": false}),
    ] {
        assert_eq!(
            code_of::<ContentBlock>(&block.to_string()),
            ErrorCode::VariantMismatch,
            "{block}"
        );
    }
}

#[test]
fn given_standalone_zero_x_blocks_of_a_kept_kind_when_loaded_then_the_missing_id_is_a_field_error()
{
    for block in [
        json!({"type": "text", "text": "hi"}),
        json!({"type": "thinking", "text": "x", "signature": "s"}),
        json!({"type": "redacted_thinking", "data": "d"}),
    ] {
        let code = code_of::<ContentBlock>(&block.to_string());

        assert_eq!(code, ErrorCode::InvalidContent, "{block}");
        assert_ne!(code, ErrorCode::SchemaMismatch);
    }
}

#[test]
fn given_standalone_current_values_when_written_then_no_schema_field_and_a_schema_field_is_refused()
{
    let turn = Turn::from(common::turns::open_turn(1));
    let entry = common::turns::finished(2);
    let block = ContentBlock::from_value(common::oj(
        json!({"type": "structured_content", "id": "ab000000-0000-7000-8000-000000000009", "value": 1}),
    ))
    .unwrap();

    for (written, refused_code) in [
        (
            turn.to_value().unwrap(),
            code_of::<Turn> as fn(&str) -> ErrorCode,
        ),
        (entry.to_value().unwrap(), code_of::<TurnEntry>),
        (block.to_value().unwrap(), code_of::<ContentBlock>),
    ] {
        let mut document = written.deserialize_into::<Value>().unwrap();
        assert!(document.get("schema").is_none());
        document["schema"] = json!("br-llm-messages/4");

        assert_eq!(
            refused_code(&document.to_string()),
            ErrorCode::InvalidContent
        );
    }
}
