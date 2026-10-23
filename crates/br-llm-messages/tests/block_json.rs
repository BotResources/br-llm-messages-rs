mod common;

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
            ], "replay": sig()}),
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
            json!({"type": "runtime_tool_call", "id": id(1), "external_id": "toolu_01", "name": "search", "arguments": {"q": "x"}, "replay": sig()}),
        ),
        (
            ContentBlockKind::RuntimeToolResult,
            json!({"type": "runtime_tool_result", "id": id(2), "call": id(1), "tool_name": "search", "status": "error", "content": [
                text_node(3),
                {"type": "image", "id": id(4), "source": {"type": "inline", "media_type": "image/webp", "data": "QQ=="}},
                {"type": "structured_content", "id": id(5), "value": null}
            ]}),
        ),
        (
            ContentBlockKind::ProviderToolCall,
            json!({"type": "provider_tool_call", "id": id(1), "external_id": "srv_1", "operation": "web_search", "arguments": null, "replay": sig()}),
        ),
        (
            ContentBlockKind::ProviderToolResult,
            json!({"type": "provider_tool_result", "id": id(2), "call": id(1), "external_id": "res_1", "content": [text_node(3)], "replay": envelope()}),
        ),
        (
            ContentBlockKind::ProviderToolExecutionRecord,
            json!({"type": "provider_tool_execution_record", "id": id(1), "external_id": "ex_1", "operation": "code_execution", "status": "incomplete",
                "exposure": {"type": "mixed", "content": [text_node(2)], "material": sig()}}),
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
    for (kind, name) in ContentBlockKind::ALL.into_iter().zip(expected) {
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
fn given_an_unknown_kind_name_when_parsed_then_unsupported_variant() {
    for other in ["Text", "tool_call", "", "text ", "provider_tool"] {
        assert_eq!(
            refused(other.parse::<ContentBlockKind>()).code(),
            ErrorCode::UnsupportedVariant
        );
    }
    assert_eq!(
        common::code_of::<ContentBlockKind>("\"nope\""),
        ErrorCode::UnsupportedVariant
    );
}

#[test]
fn given_each_golden_json_when_loaded_then_kind_matches_and_it_serializes_back_identically() {
    for (kind, golden) in goldens() {
        let block = ContentBlock::from_value(golden.clone()).unwrap();

        assert_eq!(block.kind(), kind);
        assert_eq!(block.to_value().unwrap(), golden, "{kind}");
        assert_eq!(
            ContentBlock::from_json(&block.to_json().unwrap()).unwrap(),
            block
        );
        assert_eq!(ContentBlock::from_json(&golden.to_string()).unwrap(), block);
    }
}

#[test]
fn given_exposure_variants_when_round_tripped_then_each_wire_form_is_golden() {
    let record = |exposure: Value| json!({"type": "provider_tool_execution_record", "id": id(1), "exposure": exposure});
    for exposure in [
        json!({"type": "readable", "content": [text_node(2)]}),
        json!({"type": "opaque", "material": envelope()}),
        json!({"type": "mixed", "content": [text_node(2)], "material": sig()}),
        json!({"type": "not_exposed"}),
    ] {
        let golden = record(exposure);
        let block = ContentBlock::from_value(golden.clone()).unwrap();

        assert_eq!(block.to_value().unwrap(), golden);
    }
}

#[test]
fn given_minimal_optional_fields_when_serialized_then_absent_ones_are_omitted() {
    let call = json!({"type": "provider_tool_call", "id": id(1)});
    let thinking = json!({"type": "thinking", "id": id(1), "text": ""});
    let empty_result = json!({"type": "runtime_tool_result", "id": id(1), "call": id(2), "tool_name": "t", "status": "success", "content": []});

    for golden in [call, thinking, empty_result] {
        let block = ContentBlock::from_value(golden.clone()).unwrap();

        assert_eq!(block.to_value().unwrap(), golden);
    }
}

#[test]
fn given_null_optionals_when_loaded_then_they_read_as_absent_and_provider_null_arguments_survive() {
    let loaded = ContentBlock::from_value(json!({
        "type": "provider_tool_call", "id": id(1), "external_id": null, "operation": null, "replay": null, "arguments": null
    }))
    .unwrap();

    let ContentBlock::ProviderToolCall(call) = loaded else {
        panic!("wrong kind");
    };
    assert_eq!(call.external_id(), None);
    assert_eq!(call.operation(), None);
    assert_eq!(call.replay(), None);
    assert_eq!(call.arguments(), Some(&Value::Null));
}

#[test]
fn given_replay_payloads_when_round_tripped_then_values_are_preserved_exactly() {
    let thinking = json!({"type": "thinking", "id": id(1), "text": "a\u{0}b\n  ", "replay":
        {"format": "x.y", "payload": {"type": "text", "value": "  AbC+/= \n"}}});

    let block = ContentBlock::from_value(thinking.clone()).unwrap();

    assert_eq!(block.to_value().unwrap(), thinking);
}

#[test]
fn given_u64_max_arguments_when_round_tripped_then_exact() {
    let golden = json!({"type": "runtime_tool_call", "id": id(1), "name": "t", "arguments": {"n": u64::MAX, "f": 0.1}});

    let block = ContentBlock::from_json(&golden.to_string()).unwrap();

    assert_eq!(block.to_value().unwrap(), golden);
}
