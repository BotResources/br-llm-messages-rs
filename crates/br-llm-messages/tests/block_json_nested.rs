mod common;

use br_llm_messages::{ContentBlock, ErrorCode, PlainBlock};
use common::{bid, code_of, refused, v7};
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
fn given_non_plain_blocks_inside_plain_content_when_loaded_then_disallowed_kind() {
    let thinking = json!({"type": "thinking", "id": id(9), "text": "t"});
    let call = json!({"type": "runtime_tool_call", "id": id(8), "name": "t", "arguments": {}});
    let nested_result = json!({"type": "runtime_tool_result", "id": id(7), "call": id(6), "tool_name": "t", "is_error": false, "content": []});
    let result = |content: Value| json!({"type": "runtime_tool_result", "id": id(1), "call": id(2), "tool_name": "t", "is_error": false, "content": content});

    for inner in [thinking, call, nested_result] {
        assert_eq!(
            code(&result(json!([text_node(3), inner.clone()]))),
            ErrorCode::DisallowedKind
        );
        assert_eq!(plain_code(&inner), ErrorCode::DisallowedKind);
    }
    let error = refused(
        common::oj(json!({"type": "thinking", "id": id(9), "text": ""}))
            .deserialize_into::<PlainBlock>(),
    );
    assert_eq!(error.object_id(), Some(v7(9)));
}

#[test]
fn given_unknown_type_or_replay_on_plain_content_when_loaded_then_variant_mismatch_or_disallowed_kind()
 {
    assert_eq!(
        plain_code(&json!({"type": "audio", "id": id(1)})),
        ErrorCode::VariantMismatch
    );
    assert_eq!(
        plain_code(&json!({"id": id(1)})),
        ErrorCode::VariantMismatch
    );
    let replayed = json!({"type": "text", "id": id(1), "text": "x", "replay": {"format": "a.b", "payload": {"type": "text", "value": "s"}}});
    assert_eq!(plain_code(&replayed), ErrorCode::DisallowedKind);
    assert_eq!(
        code(
            &json!({"type": "provider_tool_result", "id": id(2), "call": id(5), "content": [replayed]})
        ),
        ErrorCode::DisallowedKind
    );
    let referenced = json!({"type": "text", "id": id(1), "text": "x", "item_ref": "msg_1"});
    assert_eq!(plain_code(&referenced), ErrorCode::DisallowedKind);
    assert!(ContentBlock::from_value(common::oj(referenced)).is_ok());
    assert!(ContentBlock::from_value(common::oj(replayed)).is_ok());
}

#[test]
fn given_duplicate_nested_ids_in_json_when_loaded_then_reused_identity() {
    let result = json!({"type": "runtime_tool_result", "id": id(1), "call": id(2), "tool_name": "t", "is_error": false,
        "content": [text_node(3), text_node(3)]});
    let own = json!({"type": "runtime_tool_result", "id": id(1), "call": id(2), "tool_name": "t", "is_error": false,
        "content": [text_node(1)]});

    assert_eq!(code(&result), ErrorCode::ReusedIdentity);
    assert_eq!(code(&own), ErrorCode::ReusedIdentity);
}

#[test]
fn given_runtime_result_with_a_bad_flag_or_invalid_content_when_loaded_then_refused() {
    let mut bad = json!({"type": "runtime_tool_result", "id": id(1), "call": id(2), "tool_name": "t", "is_error": false, "content": []});
    bad["is_error"] = json!("ok");
    assert_eq!(code(&bad), ErrorCode::InvalidContent);
    let mut legacy = bad.clone();
    legacy.as_object_mut().unwrap().remove("is_error");
    legacy["status"] = json!("success");
    assert_eq!(code(&legacy), ErrorCode::InvalidContent);
    let mut hidden = json!({"type": "runtime_tool_result", "id": id(1), "call": id(2), "tool_name": "t", "is_error": false, "content": []});
    hidden["content"] = json!([{"type": "provider_tool_call", "id": id(4)}]);
    assert_eq!(code(&hidden), ErrorCode::DisallowedKind);
}

#[test]
fn given_provider_result_without_content_or_replay_when_loaded_then_invalid_content() {
    let bare = json!({"type": "provider_tool_result", "id": id(1), "call": id(2)});
    let empty = json!({"type": "provider_tool_result", "id": id(1), "call": id(2), "content": []});
    let opaque = json!({"type": "provider_tool_result", "id": id(1), "call": id(2), "replay": {"format": "a.b", "payload": {"type": "json", "value": 1}}});

    assert_eq!(code(&bare), ErrorCode::InvalidContent);
    assert_eq!(code(&empty), ErrorCode::InvalidContent);
    assert!(ContentBlock::from_value(common::oj(opaque)).is_ok());
}

#[test]
fn given_a_record_in_json_with_a_stored_exposure_or_bad_content_when_loaded_then_refused() {
    let record = |extra: Value| {
        let mut base = json!({"type": "provider_tool_execution_record", "id": id(1)});
        base.as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        base
    };

    assert_eq!(
        code(&record(json!({"exposure": {"type": "not_exposed"}}))),
        ErrorCode::InvalidContent
    );
    assert_eq!(
        code(&record(json!({"content": [text_node(1)]}))),
        ErrorCode::ReusedIdentity
    );
    assert_eq!(
        code(&record(
            json!({"content": [{"type": "thinking", "id": id(2), "text": "t"}]})
        )),
        ErrorCode::DisallowedKind
    );
    assert!(ContentBlock::from_value(common::oj(record(json!({"content": []})))).is_ok());
}

#[test]
fn given_an_unknown_execution_status_when_loaded_then_variant_mismatch() {
    let mut value = json!({"type": "provider_tool_execution_record", "id": id(1)});
    value["status"] = json!("exploded");

    assert_eq!(code(&value), ErrorCode::VariantMismatch);
}
