mod common;

use br_llm_messages::{ContentBlock, ErrorCode};
use common::{bid, code_of};
use serde_json::{Value, json};

fn id(n: u64) -> String {
    bid(n).to_string()
}

fn text_node(n: u64) -> Value {
    json!({"type": "text", "id": id(n), "text": "ok"})
}

fn code(value: &Value) -> ErrorCode {
    code_of::<ContentBlock>(&value.to_string())
}

fn sample(kind: &str) -> (Value, &'static [&'static str]) {
    let sig = json!({"format": "a.b", "payload": {"type": "text", "value": "s"}});
    match kind {
        "text" => (text_node(1), &["id", "text"]),
        "image" => (
            json!({"type": "image", "id": id(1), "source": {"type": "inline", "media_type": "image/png", "data": "QUJD"}}),
            &["id", "source"],
        ),
        "structured_content" => (
            json!({"type": "structured_content", "id": id(1), "value": 1}),
            &["id", "value"],
        ),
        "thinking" => (
            json!({"type": "thinking", "id": id(1), "text": "t"}),
            &["id", "text"],
        ),
        "redacted_thinking" => (
            json!({"type": "redacted_thinking", "id": id(1), "material": sig}),
            &["id", "material"],
        ),
        "runtime_tool_call" => (
            json!({"type": "runtime_tool_call", "id": id(1), "name": "t", "arguments": {}}),
            &["id", "name", "arguments"],
        ),
        "runtime_tool_result" => (
            json!({"type": "runtime_tool_result", "id": id(1), "call": id(2), "tool_name": "t", "is_error": false, "content": []}),
            &["id", "call", "tool_name", "is_error", "content"],
        ),
        "provider_tool_call" => (json!({"type": "provider_tool_call", "id": id(1)}), &["id"]),
        "provider_tool_result" => (
            json!({"type": "provider_tool_result", "id": id(1), "call": id(2), "content": [text_node(3)]}),
            &["id", "call"],
        ),
        _ => (
            json!({"type": "provider_tool_execution_record", "id": id(1)}),
            &["id"],
        ),
    }
}

const KINDS: [&str; 10] = [
    "text",
    "image",
    "structured_content",
    "thinking",
    "redacted_thinking",
    "runtime_tool_call",
    "runtime_tool_result",
    "provider_tool_call",
    "provider_tool_result",
    "provider_tool_execution_record",
];

#[test]
fn given_an_extra_field_on_any_kind_when_loaded_then_invalid_content() {
    for kind in KINDS {
        let (mut value, _) = sample(kind);
        value["bogus"] = json!(1);

        assert_eq!(code(&value), ErrorCode::InvalidContent, "{kind}");
    }
}

#[test]
fn given_a_missing_required_field_on_any_kind_when_loaded_then_invalid_content() {
    for kind in KINDS {
        let (value, required) = sample(kind);
        assert!(
            ContentBlock::from_value(common::oj(value.clone())).is_ok(),
            "{kind}"
        );
        for field in required {
            let mut stripped = value.clone();
            stripped.as_object_mut().unwrap().remove(*field);

            assert_eq!(code(&stripped), ErrorCode::InvalidContent, "{kind}.{field}");
        }
    }
}

#[test]
fn given_a_bad_or_missing_type_when_loaded_then_variant_mismatch() {
    assert_eq!(
        code(&json!({"type": "tool_use", "id": id(1)})),
        ErrorCode::VariantMismatch
    );
    assert_eq!(
        code(&json!({"id": id(1), "text": "x"})),
        ErrorCode::VariantMismatch
    );
    assert_eq!(
        code_of::<ContentBlock>("{\"type\":"),
        ErrorCode::MalformedJson
    );
}

#[test]
fn given_a_non_v7_or_malformed_block_id_when_loaded_then_invalid_identity() {
    let mut value = text_node(1);
    value["id"] = json!("00000000-0000-0000-0000-000000000000");
    assert_eq!(code(&value), ErrorCode::InvalidIdentity);
    value["id"] = json!("not-a-uuid");
    assert_eq!(code(&value), ErrorCode::InvalidIdentity);
}

#[test]
fn given_text_blank_or_with_out_of_range_or_repeated_citations_when_loaded_then_refused() {
    let cited = |citations: Value| json!({"type": "text", "id": id(1), "text": "abc", "citations": citations});
    let citation = |n: u64, end: u64| json!({"id": id(n), "source": {"reference": "d"}, "range": {"start": 0, "end": end}});

    assert_eq!(
        code(&json!({"type": "text", "id": id(1), "text": " \n"})),
        ErrorCode::InvalidContent
    );
    assert_eq!(
        code(&cited(json!([citation(2, 4)]))),
        ErrorCode::InvalidContent
    );
    assert_eq!(
        code(&cited(json!([citation(2, 1), citation(2, 2)]))),
        ErrorCode::ReusedIdentity
    );
    assert!(ContentBlock::from_value(common::oj(cited(json!([citation(2, 3)])))).is_ok());
}

#[test]
fn given_image_data_or_media_type_invalid_when_loaded_then_refused() {
    let image = |source: Value| json!({"type": "image", "id": id(1), "source": source});

    assert_eq!(
        code(&image(
            json!({"type": "inline", "media_type": "image/bmp", "data": "QUJD"})
        )),
        ErrorCode::InvalidContent
    );
    assert_eq!(
        code(&image(
            json!({"type": "inline", "media_type": "image/png", "data": "QUJ"})
        )),
        ErrorCode::InvalidContent
    );
    assert_eq!(
        code(&image(json!({"type": "inline", "data": "QUJD"}))),
        ErrorCode::InvalidContent
    );
    assert_eq!(
        code(&image(json!({"type": "url", "url": "x"}))),
        ErrorCode::VariantMismatch
    );
}

#[test]
fn given_a_runtime_call_with_empty_external_id_or_bad_name_when_loaded_then_invalid_content() {
    let call = |extra: Value| {
        let mut base =
            json!({"type": "runtime_tool_call", "id": id(1), "name": "t", "arguments": {}});
        base.as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        base
    };

    assert_eq!(
        code(&call(json!({"external_id": ""}))),
        ErrorCode::InvalidContent
    );
    assert_eq!(
        code(&call(json!({"name": "bad name"}))),
        ErrorCode::InvalidContent
    );
    assert_eq!(code(&call(json!({"name": ""}))), ErrorCode::InvalidContent);
}
