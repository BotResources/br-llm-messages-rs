mod common;

use br_llm_messages::{
    AgentMail, AgentRequest, ErrorCode, HumanMessage, RequestOrigin, TurnEntry, TurnEntryKind,
};
use common::entries::all_entries;
use common::{aid, author, bid, cvid, eid, hid, plain_text, png, refused, rid};
use serde_json::{Value, json};

#[test]
fn given_the_twelve_kinds_when_named_then_snake_case_discriminants_are_golden() {
    let expected = [
        "human_message",
        "agent_request",
        "agent_mail",
        "request_accepted",
        "queue_position",
        "task_started",
        "task_completed",
        "task_failed",
        "model_response",
        "runtime_tool_results",
        "turn_closure",
        "runtime_notice",
    ];

    let actual: Vec<&str> = TurnEntryKind::ALL.iter().map(|k| k.as_str()).collect();

    assert_eq!(actual, expected);
    for kind in TurnEntryKind::ALL.iter().copied() {
        assert_eq!(kind.as_str().parse::<TurnEntryKind>().unwrap(), kind);
        assert_eq!(
            serde_json::to_value(kind).unwrap(),
            Value::String(kind.as_str().into())
        );
    }
}

#[test]
fn given_every_entry_when_inspected_then_each_has_its_own_id_and_matching_kind() {
    let entries = all_entries();

    let ids: Vec<_> = entries.iter().map(|e| e.id()).collect();
    let kinds: Vec<_> = entries.iter().map(|e| e.kind()).collect();

    assert_eq!(ids.len(), 12);
    assert_eq!(
        ids,
        (1..=8).chain([9, 10, 11, 12]).map(eid).collect::<Vec<_>>()
    );
    assert_eq!(kinds, TurnEntryKind::ALL.to_vec());
}

#[test]
fn given_a_human_message_without_blocks_when_built_then_invalid_content() {
    let error = refused(HumanMessage::new(eid(1), hid(1), vec![]));

    assert_eq!(error.code(), ErrorCode::InvalidContent);
    assert_eq!(error.object_id(), Some(eid(1).as_uuid()));
}

#[test]
fn given_a_human_message_when_blocks_are_text_image_structured_then_accepted_in_order() {
    let blocks = vec![
        plain_text(2, "a"),
        png(3).into(),
        br_llm_messages::StructuredBlock::new(bid(4), common::oj(json!(null)))
            .unwrap()
            .into(),
    ];

    let message = HumanMessage::new(eid(1), hid(1), blocks.clone()).unwrap();

    assert_eq!(message.content(), blocks.as_slice());
}

#[test]
fn given_a_human_message_json_with_thinking_content_when_loaded_then_disallowed_kind() {
    let value = json!({"type": "human_message", "id": eid(1).to_string(), "human": hid(1).to_string(),
        "content": [{"type": "thinking", "id": bid(2).to_string(), "text": ""}]});

    let error = refused(TurnEntry::from_value(common::oj(value)));

    assert_eq!(error.code(), ErrorCode::DisallowedKind);
}

#[test]
fn given_a_human_message_json_with_an_author_key_even_null_when_loaded_then_author_forbidden() {
    for author_value in [json!(null), json!("Codex"), json!(""), json!(3)] {
        let value = json!({"type": "human_message", "id": eid(1).to_string(), "human": hid(1).to_string(),
            "content": [{"type": "text", "id": bid(2).to_string(), "text": "hi"}], "author": author_value});

        let error = refused(TurnEntry::from_value(common::oj(value)));

        assert_eq!(error.code(), ErrorCode::AuthorForbidden, "{author_value}");
    }
}

#[test]
fn given_content_reusing_the_entry_id_when_built_then_reused_identity() {
    let error = refused(HumanMessage::new(
        eid(1),
        hid(1),
        vec![plain_text(1, "same uuid as the entry")],
    ));

    assert_eq!(error.code(), ErrorCode::ReusedIdentity);
    assert_eq!(error.object_id(), Some(eid(1).as_uuid()));
}

#[test]
fn given_two_blocks_sharing_an_id_when_building_a_message_then_reused_identity() {
    let error = refused(HumanMessage::new(
        eid(1),
        hid(1),
        vec![plain_text(2, "a"), plain_text(2, "b")],
    ));

    assert_eq!(error.code(), ErrorCode::ReusedIdentity);
}

#[test]
fn given_an_agent_request_when_built_then_requester_author_source_and_origin_are_kept() {
    let request = AgentRequest::new(
        eid(1),
        aid(7),
        author(" Codex GPT-6.1-Sol high "),
        cvid(9),
        RequestOrigin::tool_call(bid(55)),
        vec![plain_text(2, "do it")],
    )
    .unwrap();

    assert_eq!(request.requester(), aid(7));
    assert_eq!(request.author().as_str(), " Codex GPT-6.1-Sol high ");
    assert_eq!(request.source_conversation(), cvid(9));
    assert_eq!(request.origin(), &RequestOrigin::tool_call(bid(55)));
}

#[test]
fn given_an_agent_request_without_content_when_built_then_invalid_content() {
    let error = refused(AgentRequest::new(
        eid(1),
        aid(1),
        author("A"),
        cvid(1),
        RequestOrigin::request(rid(1)),
        vec![],
    ));

    assert_eq!(error.code(), ErrorCode::InvalidContent);
}

#[test]
fn given_an_agent_request_json_missing_a_required_field_when_loaded_then_invalid_content() {
    let full = json!({"type": "agent_request", "id": eid(1).to_string(), "requester": aid(1).to_string(),
        "author": "A", "source_conversation": cvid(1).to_string(),
        "origin": {"type": "tool_call", "block": bid(5).to_string()},
        "content": [{"type": "text", "id": bid(2).to_string(), "text": "hi"}]});
    assert!(TurnEntry::from_value(common::oj(full.clone())).is_ok());

    for field in [
        "requester",
        "author",
        "source_conversation",
        "origin",
        "content",
    ] {
        let mut stripped = full.clone();
        stripped.as_object_mut().unwrap().remove(field);

        let error = refused(TurnEntry::from_value(common::oj(stripped)));

        assert_eq!(error.code(), ErrorCode::InvalidContent, "{field}");
    }
}

#[test]
fn given_an_agent_request_json_with_a_blank_author_when_loaded_then_invalid_author() {
    let value = json!({"type": "agent_request", "id": eid(1).to_string(), "requester": aid(1).to_string(),
        "author": "  ", "source_conversation": cvid(1).to_string(),
        "origin": {"type": "request", "id": rid(1).to_string()},
        "content": [{"type": "text", "id": bid(2).to_string(), "text": "hi"}]});

    let error = refused(TurnEntry::from_value(common::oj(value)));

    assert_eq!(error.code(), ErrorCode::InvalidAuthor);
}

#[test]
fn given_an_agent_request_json_with_an_unknown_origin_when_loaded_then_variant_mismatch() {
    let value = json!({"type": "agent_request", "id": eid(1).to_string(), "requester": aid(1).to_string(),
        "author": "A", "source_conversation": cvid(1).to_string(),
        "origin": {"type": "carrier_pigeon"},
        "content": [{"type": "text", "id": bid(2).to_string(), "text": "hi"}]});

    let error = refused(TurnEntry::from_value(common::oj(value)));

    assert_eq!(error.code(), ErrorCode::VariantMismatch);
}

#[test]
fn given_an_agent_mail_when_built_then_correlation_is_optional_and_kept() {
    let plain = AgentMail::new(
        eid(1),
        aid(1),
        author("A"),
        cvid(1),
        vec![plain_text(2, "m")],
    )
    .unwrap();
    let replying = plain.clone().with_in_reply_to(eid(77));

    assert_eq!(plain.in_reply_to(), None);
    assert_eq!(replying.in_reply_to(), Some(eid(77)));
    assert_eq!(
        serde_json::to_value(&replying).unwrap()["in_reply_to"],
        json!(eid(77).to_string())
    );
}

#[test]
fn given_an_agent_mail_json_when_it_has_a_source_message_id_then_invalid_content() {
    let value = json!({"type": "agent_mail", "id": eid(1).to_string(), "sender": aid(1).to_string(),
        "author": "A", "source_conversation": cvid(1).to_string(), "source_message": eid(9).to_string(),
        "content": [{"type": "text", "id": bid(2).to_string(), "text": "hi"}]});

    let error = refused(TurnEntry::from_value(common::oj(value)));

    assert_eq!(error.code(), ErrorCode::InvalidContent);
}

#[test]
fn given_an_agent_mail_json_without_author_or_source_when_loaded_then_invalid_content() {
    let full = json!({"type": "agent_mail", "id": eid(1).to_string(), "sender": aid(1).to_string(),
        "author": "A", "source_conversation": cvid(1).to_string(),
        "content": [{"type": "text", "id": bid(2).to_string(), "text": "hi"}]});
    assert!(TurnEntry::from_value(common::oj(full.clone())).is_ok());

    for field in ["sender", "author", "source_conversation"] {
        let mut stripped = full.clone();
        stripped.as_object_mut().unwrap().remove(field);

        let error = refused(TurnEntry::from_value(common::oj(stripped)));

        assert_eq!(error.code(), ErrorCode::InvalidContent, "{field}");
    }
}
