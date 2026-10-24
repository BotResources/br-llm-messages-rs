mod common;
use common::Raw as _;

use br_llm_messages::{ContentBlock, ContentBlockKind, ErrorCode};
use common::{bid, cid, refused};
use serde_json::{Value, json};

fn id(n: u64) -> String {
    bid(n).to_string()
}

fn sig() -> Value {
    json!({"format": "anthropic.signature", "payload": {"type": "text", "value": "sig=="}})
}

fn envelope() -> Value {
    json!({"format": "openai.reasoning", "payload": {"type": "json", "value": {"enc": "AAA=", "n": [1, null]}}})
}

fn text_node(n: u64) -> Value {
    json!({"type": "text", "id": id(n), "text": "ok"})
}

pub fn goldens() -> Vec<(ContentBlockKind, Value)> {
    vec![
        (
            ContentBlockKind::Text,
            json!({"type": "text", "id": id(1), "text": " hello é😀 ", "citations": [
                {"id": cid(2).to_string(), "source": {"reference": "doc1", "title": "T"}, "range": {"start": 0, "end": 5}}
            ], "item_ref": "msg_01", "replay": sig()}),
        ),
        (
            ContentBlockKind::Image,
            json!({"type": "image", "id": id(1), "source": {"type": "inline", "media_type": "image/png", "data": "QUJD"}}),
        ),
        (
            ContentBlockKind::StructuredContent,
            json!({"type": "structured_content", "id": id(1), "value": {"a": [1, null, 2.5], "b": {}}}),
        ),
        (
            ContentBlockKind::Thinking,
            json!({"type": "thinking", "id": id(1), "text": "", "replay": envelope()}),
        ),
        (
            ContentBlockKind::RedactedThinking,
            json!({"type": "redacted_thinking", "id": id(1), "material": sig()}),
        ),
        (
            ContentBlockKind::RuntimeToolCall,
            json!({"type": "runtime_tool_call", "id": id(1), "external_id": "toolu_01", "item_ref": "fc_01", "name": "search", "arguments": {"q": "x"}, "replay": sig()}),
        ),
        (
            ContentBlockKind::RuntimeToolResult,
            json!({"type": "runtime_tool_result", "id": id(2), "call": id(1), "tool_name": "search", "is_error": true, "content": [
                text_node(3),
                {"type": "image", "id": id(4), "source": {"type": "inline", "media_type": "image/webp", "data": "QQ=="}},
                {"type": "structured_content", "id": id(5), "value": null}
            ]}),
        ),
        (
            ContentBlockKind::ProviderToolCall,
            json!({"type": "provider_tool_call", "id": id(1), "external_id": "srv_1", "item_ref": "ws_01", "operation": "web_search", "arguments": {"query": "x"}, "replay": sig()}),
        ),
        (
            ContentBlockKind::ProviderToolResult,
            json!({"type": "provider_tool_result", "id": id(2), "call": id(1), "external_id": "res_1", "content": [text_node(3)], "replay": envelope()}),
        ),
        (
            ContentBlockKind::ProviderToolExecutionRecord,
            json!({"type": "provider_tool_execution_record", "id": id(1), "external_id": "ex_1", "operation": "code_execution", "status": "incomplete",
                "content": [text_node(2)], "replay": sig()}),
        ),
    ]
}

#[test]
fn given_the_ten_kinds_when_named_then_snake_case_discriminants_are_golden() {
    let expected = [
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

    assert_eq!(ContentBlockKind::ALL.len(), expected.len());
    for (kind, name) in ContentBlockKind::ALL.iter().copied().zip(expected) {
        assert_eq!(kind.as_str(), name);
        assert_eq!(kind.to_string(), name);
        assert_eq!(name.parse::<ContentBlockKind>().unwrap(), kind);
        assert_eq!(serde_json::to_value(kind).unwrap(), json!(name));
        assert_eq!(
            serde_json::from_value::<ContentBlockKind>(json!(name)).unwrap(),
            kind
        );
    }
}

#[test]
fn given_an_unknown_kind_name_when_parsed_then_variant_mismatch() {
    for other in ["Text", "tool_call", "", "text ", "provider_tool"] {
        assert_eq!(
            refused(other.parse::<ContentBlockKind>()).code(),
            ErrorCode::VariantMismatch
        );
    }
    assert_eq!(
        common::code_of::<ContentBlockKind>("\"nope\""),
        ErrorCode::VariantMismatch
    );
}

#[test]
fn given_each_golden_json_when_loaded_then_kind_matches_and_it_serializes_back_identically() {
    for (kind, golden) in goldens() {
        let block = ContentBlock::from_value(common::oj(golden.clone())).unwrap();

        assert_eq!(block.kind(), kind);
        assert_eq!(block.to_value().unwrap().raw(), golden, "{kind}");
        assert_eq!(
            ContentBlock::from_json(&block.to_json().unwrap()).unwrap(),
            block
        );
        assert_eq!(ContentBlock::from_json(&golden.to_string()).unwrap(), block);
    }
}

#[test]
fn given_every_content_and_replay_combination_when_round_tripped_then_the_wire_has_no_exposure_key()
{
    let record = |extra: Value| {
        let mut base = json!({"type": "provider_tool_execution_record", "id": id(1)});
        base.as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        base
    };
    for golden in [
        record(json!({"content": [text_node(2)]})),
        record(json!({"replay": envelope()})),
        record(json!({"content": [text_node(2)], "replay": sig()})),
        record(json!({})),
    ] {
        let block = ContentBlock::from_value(common::oj(golden.clone())).unwrap();

        assert_eq!(block.to_value().unwrap().raw(), golden);
        assert!(block.to_json().unwrap().find("exposure").is_none());
    }
}

#[test]
fn given_minimal_optional_fields_when_serialized_then_absent_ones_are_omitted() {
    let call = json!({"type": "provider_tool_call", "id": id(1)});
    let thinking = json!({"type": "thinking", "id": id(1), "text": "x"});
    let empty_result = json!({"type": "runtime_tool_result", "id": id(1), "call": id(2), "tool_name": "t", "is_error": false, "content": []});

    for golden in [call, thinking, empty_result] {
        let block = ContentBlock::from_value(common::oj(golden.clone())).unwrap();

        assert_eq!(block.to_value().unwrap().raw(), golden);
    }
}

#[test]
fn given_a_null_on_an_optional_provider_call_key_when_loaded_then_invalid_content() {
    for key in [
        "external_id",
        "operation",
        "replay",
        "arguments",
        "item_ref",
        "observed_at",
    ] {
        let mut wire = json!({"type": "provider_tool_call", "id": id(1)});
        wire[key] = Value::Null;

        let error = ContentBlock::from_value(common::oj(wire)).unwrap_err();

        assert_eq!(error.code(), ErrorCode::InvalidContent, "{key}");
    }
}

#[test]
fn given_replay_payloads_when_round_tripped_then_values_are_preserved_exactly() {
    let thinking = json!({"type": "thinking", "id": id(1), "text": "a\u{0}b\n  ", "replay":
        {"format": "x.y", "payload": {"type": "text", "value": "  AbC+/= \n"}}});

    let block = ContentBlock::from_value(common::oj(thinking.clone())).unwrap();

    assert_eq!(block.to_value().unwrap().raw(), thinking);
}

#[test]
fn given_u64_max_arguments_when_round_tripped_then_exact() {
    let golden = json!({"type": "runtime_tool_call", "id": id(1), "name": "t", "arguments": {"n": u64::MAX, "f": 0.1}});

    let block = ContentBlock::from_json(&golden.to_string()).unwrap();

    assert_eq!(block.to_value().unwrap().raw(), golden);
}
