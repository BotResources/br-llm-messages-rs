mod common;

use br_llm_messages::{ContentBlock, ErrorCode, PlainBlock};
use common::{bid, code_of, v7};
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

fn plain_code(value: &Value) -> ErrorCode {
    code_of::<PlainBlock>(&value.to_string())
}

#[test]
fn given_non_plain_blocks_inside_plain_content_when_loaded_then_disallowed_block_kind() {
    let thinking = json!({"type": "thinking", "id": id(9), "text": "t"});
    let call = json!({"type": "runtime_tool_call", "id": id(8), "name": "t", "arguments": {}});
    let nested_result = json!({"type": "runtime_tool_result", "id": id(7), "call": id(6), "tool_name": "t", "status": "success", "content": []});
    let result = |content: Value| json!({"type": "runtime_tool_result", "id": id(1), "call": id(2), "tool_name": "t", "status": "success", "content": content});

    for inner in [thinking, call, nested_result] {
        assert_eq!(
            code(&result(json!([text_node(3), inner.clone()]))),
            ErrorCode::DisallowedBlockKind
        );
        assert_eq!(plain_code(&inner), ErrorCode::DisallowedBlockKind);
    }
    let error =
        serde_json::from_value::<PlainBlock>(json!({"type": "thinking", "id": id(9), "text": ""}))
            .unwrap_err();
    assert_eq!(
        br_llm_messages::Error::from_json_error(&error).object_id(),
        Some(v7(9))
    );
}

#[test]
fn given_unknown_type_or_replay_on_plain_content_when_loaded_then_unsupported_variant_or_invalid_content()
 {
    assert_eq!(
        plain_code(&json!({"type": "audio", "id": id(1)})),
        ErrorCode::UnsupportedVariant
    );
    assert_eq!(plain_code(&json!({"id": id(1)})), ErrorCode::MissingContent);
    let replayed = json!({"type": "text", "id": id(1), "text": "x", "replay": {"format": "a.b", "payload": {"type": "text", "value": "s"}}});
    assert_eq!(plain_code(&replayed), ErrorCode::InvalidContent);
    assert_eq!(
        code(
            &json!({"type": "provider_tool_result", "id": id(2), "call": id(5), "content": [replayed]})
        ),
        ErrorCode::InvalidContent
    );
    assert!(ContentBlock::from_value(common::oj(replayed)).is_ok());
}

#[test]
fn given_duplicate_nested_ids_in_json_when_loaded_then_reused_identity() {
    let result = json!({"type": "runtime_tool_result", "id": id(1), "call": id(2), "tool_name": "t", "status": "success",
        "content": [text_node(3), text_node(3)]});
    let own = json!({"type": "runtime_tool_result", "id": id(1), "call": id(2), "tool_name": "t", "status": "success",
        "content": [text_node(1)]});

    assert_eq!(code(&result), ErrorCode::ReusedIdentity);
    assert_eq!(code(&own), ErrorCode::ReusedIdentity);
}

#[test]
fn given_runtime_result_with_bad_status_or_missing_content_when_loaded_then_refused() {
    let mut bad = json!({"type": "runtime_tool_result", "id": id(1), "call": id(2), "tool_name": "t", "status": "success", "content": []});
    bad["status"] = json!("ok");
    assert_eq!(code(&bad), ErrorCode::UnsupportedVariant);
    let mut hidden = json!({"type": "runtime_tool_result", "id": id(1), "call": id(2), "tool_name": "t", "status": "success", "content": []});
    hidden["content"] = json!([{"type": "provider_tool_call", "id": id(4)}]);
    assert_eq!(code(&hidden), ErrorCode::DisallowedBlockKind);
}

#[test]
fn given_provider_result_without_content_or_replay_when_loaded_then_missing_content() {
    let bare = json!({"type": "provider_tool_result", "id": id(1), "call": id(2)});
    let empty = json!({"type": "provider_tool_result", "id": id(1), "call": id(2), "content": []});
    let opaque = json!({"type": "provider_tool_result", "id": id(1), "call": id(2), "replay": {"format": "a.b", "payload": {"type": "json", "value": 1}}});

    assert_eq!(code(&bare), ErrorCode::MissingContent);
    assert_eq!(code(&empty), ErrorCode::MissingContent);
    assert!(ContentBlock::from_value(common::oj(opaque)).is_ok());
}

#[test]
fn given_inconsistent_exposure_in_json_when_loaded_then_refused() {
    let record = |exposure: Value| json!({"type": "provider_tool_execution_record", "id": id(1), "exposure": exposure});
    let material = json!({"format": "a.b", "payload": {"type": "text", "value": "s"}});

    assert_eq!(
        code(&record(json!({"type": "readable", "content": []}))),
        ErrorCode::MissingContent
    );
    assert_eq!(
        code(&record(json!({"type": "readable"}))),
        ErrorCode::MissingContent
    );
    assert_eq!(
        code(&record(
            json!({"type": "mixed", "content": [], "material": material})
        )),
        ErrorCode::MissingContent
    );
    assert_eq!(
        code(&record(json!({"type": "mixed", "content": [text_node(2)]}))),
        ErrorCode::MissingContent
    );
    assert_eq!(
        code(&record(json!({"type": "opaque"}))),
        ErrorCode::MissingContent
    );
    assert_eq!(
        code(&record(
            json!({"type": "opaque", "content": [text_node(2)], "material": material})
        )),
        ErrorCode::UnsupportedField
    );
    assert_eq!(
        code(&record(
            json!({"type": "not_exposed", "material": material})
        )),
        ErrorCode::UnsupportedField
    );
    assert_eq!(
        code(&record(json!({"type": "partial"}))),
        ErrorCode::UnsupportedVariant
    );
    assert_eq!(
        code(&record(
            json!({"type": "readable", "content": [text_node(1)]})
        )),
        ErrorCode::ReusedIdentity
    );
}

#[test]
fn given_an_unknown_execution_status_when_loaded_then_unsupported_variant() {
    let mut value = json!({"type": "provider_tool_execution_record", "id": id(1), "exposure": {"type": "not_exposed"}});
    value["status"] = json!("exploded");

    assert_eq!(code(&value), ErrorCode::UnsupportedVariant);
}
