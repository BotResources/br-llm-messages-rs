mod common;
use common::Raw as _;

use br_llm_messages::{
    AgentMail, ContentBlock, ErrorCode, ExternalId, ModelName, ModelResponse, ResponseEnding,
    ResponseMetadata, TaskStarted, TurnEntry, TurnEntryKind, Usage,
};
use common::entries::all_entries;
use common::{aid, author, bid, call, cvid, eid, hid, kid, plain_text, refused, rid, signature};
use serde_json::{Value, json};

fn s(id: impl ToString) -> String {
    id.to_string()
}

fn text_node(n: u64, text: &str) -> Value {
    json!({"type": "text", "id": s(bid(n)), "text": text})
}

fn goldens() -> Vec<Value> {
    vec![
        json!({"type": "human_message", "id": s(eid(1)), "human": s(hid(1)), "content": [text_node(101, "hi")]}),
        json!({"type": "agent_request", "id": s(eid(2)), "requester": s(aid(1)), "author": "A",
            "source_conversation": s(cvid(1)), "origin": {"type": "request", "id": s(rid(1))},
            "content": [text_node(500, "x")]}),
        json!({"type": "agent_mail", "id": s(eid(3)), "sender": s(aid(1)), "author": "A",
            "source_conversation": s(cvid(1)), "content": [text_node(500, "x")]}),
        json!({"type": "request_accepted", "id": s(eid(4)), "request": s(rid(1))}),
        json!({"type": "queue_position", "id": s(eid(5)), "request": s(rid(1)), "position": 1}),
        json!({"type": "task_started", "id": s(eid(6)), "request": s(rid(1)), "task": s(kid(1))}),
        json!({"type": "task_completed", "id": s(eid(7)), "task": s(kid(1))}),
        json!({"type": "task_failed", "id": s(eid(8)), "task": s(kid(1)), "reason": "boom"}),
        json!({"type": "model_response", "id": s(eid(9)), "ending": {"type": "finished"}, "content": []}),
        json!({"type": "runtime_tool_results", "id": s(eid(10)), "results": [
            {"type": "runtime_tool_result", "id": s(bid(600)), "call": s(bid(601)), "tool_name": "t",
             "is_error": false, "content": []}]}),
        json!({"type": "turn_closure", "id": s(eid(11)), "reason": {"type": "completed"}}),
    ]
}

fn rich() -> Vec<(TurnEntry, Value)> {
    let mail = AgentMail::new(
        eid(3),
        aid(1),
        author("Codex GPT-6.1-Sol high"),
        cvid(1),
        vec![plain_text(4, "m")],
    )
    .unwrap()
    .with_in_reply_to(eid(2));
    let started = TaskStarted::new(eid(6), rid(1), kid(1)).with_conversation(cvid(9));
    let response = ModelResponse::new(
        eid(9),
        vec![
            br_llm_messages::ThinkingBlock::replay_only(bid(10), signature("sig")).into(),
            ContentBlock::from(
                call(11, "search").with_external_id(ExternalId::new("toolu_1").unwrap()),
            ),
        ],
        ResponseEnding::tool_use(),
        ResponseMetadata::new()
            .with_usage(Usage::new().with_input_tokens(1).with_output_tokens(2))
            .with_model(ModelName::new("m").unwrap())
            .with_provider_response_id(ExternalId::new("resp_1").unwrap()),
    )
    .unwrap();
    let stopped = ModelResponse::new(
        eid(12),
        vec![],
        ResponseEnding::matched_stop_sequence("</done>").unwrap(),
        ResponseMetadata::new(),
    )
    .unwrap();
    vec![
        (
            mail.into(),
            json!({"type": "agent_mail", "id": s(eid(3)), "sender": s(aid(1)), "author": "Codex GPT-6.1-Sol high",
                "source_conversation": s(cvid(1)), "in_reply_to": s(eid(2)),
                "content": [text_node(4, "m")]}),
        ),
        (
            started.into(),
            json!({"type": "task_started", "id": s(eid(6)), "request": s(rid(1)), "task": s(kid(1)),
                "conversation": s(cvid(9))}),
        ),
        (
            response.into(),
            json!({"type": "model_response", "id": s(eid(9)), "ending": {"type": "tool_use"},
            "usage": {"input_tokens": 1, "output_tokens": 2}, "model": "m", "provider_response_id": "resp_1",
            "content": [
                {"type": "thinking", "id": s(bid(10)), "text": "", "replay": {"format": "anthropic.signature", "payload": {"type": "text", "value": "sig"}}},
                {"type": "runtime_tool_call", "id": s(bid(11)), "external_id": "toolu_1", "name": "search", "arguments": {}}
            ]}),
        ),
        (
            stopped.into(),
            json!({"type": "model_response", "id": s(eid(12)),
                "ending": {"type": "stop_sequence", "sequence": "</done>"}, "content": []}),
        ),
    ]
}

#[test]
fn given_every_entry_kind_when_serialized_then_json_matches_the_golden() {
    let entries = all_entries();
    let goldens = goldens();
    assert_eq!(entries.len(), goldens.len());

    for (entry, golden) in entries.iter().zip(&goldens) {
        assert_eq!(&entry.to_value().unwrap().raw(), golden, "{}", entry.kind());
        assert_eq!(golden["type"], Value::String(entry.kind().to_string()));
    }
}

#[test]
fn given_every_golden_when_loaded_then_the_same_entry_is_rebuilt() {
    for (entry, golden) in all_entries().into_iter().zip(goldens()) {
        let loaded = TurnEntry::from_value(common::oj(golden.clone())).unwrap();

        assert_eq!(loaded, entry, "{}", entry.kind());
        assert_eq!(TurnEntry::from_json(&golden.to_string()).unwrap(), entry);
        assert_eq!(
            TurnEntry::from_json(&entry.to_json().unwrap()).unwrap(),
            entry
        );
    }
}

#[test]
fn given_entries_with_every_optional_field_set_when_round_tripped_then_goldens_hold() {
    for (entry, golden) in rich() {
        assert_eq!(entry.to_value().unwrap().raw(), golden, "{}", entry.kind());
        assert_eq!(TurnEntry::from_value(common::oj(golden)).unwrap(), entry);
    }
}

#[test]
fn given_the_goldens_when_listed_then_all_eleven_kinds_are_covered_once() {
    let kinds: Vec<TurnEntryKind> = goldens()
        .iter()
        .map(|g| g["type"].as_str().unwrap().parse().unwrap())
        .collect();

    assert_eq!(kinds, TurnEntryKind::ALL.to_vec());
}

#[test]
fn given_an_unknown_field_on_any_entry_kind_when_loaded_then_invalid_content() {
    for golden in goldens()
        .into_iter()
        .chain(rich().into_iter().map(|(_, g)| g))
    {
        let mut value = golden.clone();
        value["bogus"] = json!(1);

        let error = refused(TurnEntry::from_value(common::oj(value)));

        assert_eq!(
            error.code(),
            ErrorCode::InvalidContent,
            "{}",
            golden["type"]
        );
    }
}

#[test]
fn given_a_missing_required_field_on_any_entry_kind_when_loaded_then_invalid_content() {
    for golden in goldens() {
        let fields: Vec<String> = golden
            .as_object()
            .unwrap()
            .keys()
            .filter(|key| key.as_str() != "type")
            .cloned()
            .collect();
        for field in fields {
            let optional = matches!(
                (golden["type"].as_str().unwrap(), field.as_str()),
                ("agent_mail", "in_reply_to") | ("task_started", "conversation")
            );
            let mut stripped = golden.clone();
            stripped.as_object_mut().unwrap().remove(&field);

            let outcome = TurnEntry::from_value(common::oj(stripped));

            assert!(outcome.is_ok() == optional, "{} {field}", golden["type"]);
            if !optional {
                assert_eq!(refused(outcome).code(), ErrorCode::InvalidContent);
            }
        }
    }
}

#[test]
fn given_a_missing_or_unknown_type_tag_when_loaded_then_invalid_content_or_variant_mismatch() {
    let mut untagged = goldens().remove(3);
    untagged.as_object_mut().unwrap().remove("type");
    let mut unknown = goldens().remove(3);
    unknown["type"] = json!("request_rejected");

    assert_eq!(
        refused(TurnEntry::from_value(common::oj(untagged))).code(),
        ErrorCode::InvalidContent
    );
    assert_eq!(
        refused(TurnEntry::from_value(common::oj(unknown))).code(),
        ErrorCode::VariantMismatch
    );
}

#[test]
fn given_truncated_entry_json_when_loaded_then_malformed_json() {
    let text = goldens().remove(0).to_string();

    let error = refused(TurnEntry::from_json(&text[..text.len() - 3]));

    assert_eq!(error.code(), ErrorCode::MalformedJson);
}

#[test]
fn given_a_non_v7_or_non_canonical_id_in_entry_json_when_loaded_then_invalid_identity() {
    let uppercase = s(eid(4)).to_uppercase();
    for bad in [
        json!("not-a-uuid"),
        json!("00000000-0000-0000-0000-000000000000"),
        json!(uppercase),
    ] {
        let mut value = goldens().remove(3);
        value["id"] = bad.clone();

        let error = refused(TurnEntry::from_value(common::oj(value)));

        assert_eq!(error.code(), ErrorCode::InvalidIdentity, "{bad}");
    }
}

#[test]
fn given_the_retired_tool_offering_kinds_when_loaded_then_variant_mismatch() {
    for retired in ["tool_mounted", "tool_unmounted"] {
        let value = json!({"type": retired, "id": s(eid(1))});

        assert_eq!(
            refused(TurnEntry::from_value(common::oj(value))).code(),
            ErrorCode::VariantMismatch,
            "{retired}"
        );
        assert!(retired.parse::<TurnEntryKind>().is_err());
    }
}
