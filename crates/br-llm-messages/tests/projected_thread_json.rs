mod common;
mod projected_thread_support;
use common::Raw as _;

use br_llm_messages::{
    AITurn, Conversation, ErrorCode, IncomingEntry, ProjectedThread, Turn, TurnEntry,
};
use common::turns::tid;
use common::{aid, cvid, oj, refused};
use projected_thread_support::*;
use serde_json::json;

#[test]
fn given_a_thread_with_both_projected_variants_when_round_tripped_then_nothing_is_lost() {
    let thread = thread();

    let json = thread.to_json().unwrap();
    let reloaded = ProjectedThread::from_json(&json).unwrap();

    assert_eq!(reloaded, thread);
    assert_eq!(reloaded.conversation(), cvid(900));
    assert_eq!(reloaded.agent(), aid(1));
    let document = document();
    assert_eq!(document["schema"], "br-llm-messages-projected-thread/1");
    assert_eq!(document["turns"][2]["entries"][0]["type"], "agent_output");
    assert_eq!(
        document["turns"][3]["entries"][0]["type"],
        "partial_agent_output"
    );
    let kinds: Vec<&str> = document["turns"][4]["entries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry["type"].as_str().unwrap())
        .collect();
    assert_eq!(kinds, ["agent_output", "human_message", "agent_output"]);
    assert_eq!(
        ProjectedThread::schema(),
        "br-llm-messages-projected-thread/1"
    );
}

#[test]
fn given_a_serde_container_when_the_thread_is_decoded_then_the_same_rules_apply() {
    let text = serde_json::to_string(&thread()).unwrap();

    let decoded: ProjectedThread = serde_json::from_str(&text).unwrap();

    assert_eq!(decoded, thread());
    let mut changed = document();
    changed["turns"][2]["entries"][0]["agent"] = json!(aid(1).to_string());
    assert!(serde_json::from_value::<ProjectedThread>(changed).is_err());
}

#[test]
fn given_a_conversation_and_a_thread_when_loaded_as_each_other_then_schema_mismatch() {
    let conversation = Conversation::new(cvid(1)).to_value().unwrap().raw();
    let as_thread = refused(load(conversation));
    let as_conversation = refused(Conversation::from_value(oj(document())));

    assert_eq!(as_thread.code(), ErrorCode::SchemaMismatch);
    assert_eq!(as_conversation.code(), ErrorCode::SchemaMismatch);
    assert_eq!(as_thread.location(), Some("schema"));
}

#[test]
fn given_a_1x_conversation_document_when_loaded_then_schema_mismatch() {
    let mut old = Conversation::new(cvid(1)).to_value().unwrap().raw();
    old["schema"] = json!("br-llm-messages/4");

    assert_eq!(
        refused(Conversation::from_value(oj(old.clone()))).code(),
        ErrorCode::SchemaMismatch
    );
    assert_eq!(refused(load(old)).code(), ErrorCode::SchemaMismatch);
}

#[test]
fn given_unknown_fields_or_discriminants_or_duplicate_keys_when_loaded_then_refused_with_existing_reasons()
 {
    let mut extra = document();
    extra["extra"] = json!(1);
    let mut turn_extra = document();
    turn_extra["turns"][0]["extra"] = json!(1);
    let mut bad_type = document();
    bad_type["turns"][0]["type"] = json!("other_turn");
    let mut no_conversation = document();
    no_conversation
        .as_object_mut()
        .unwrap()
        .remove("conversation");
    let mut output_extra = document();
    output_extra["turns"][2]["entries"][0]["observed_at"] = json!("2026-10-23T08:00:00Z");

    assert_eq!(refused(load(extra)).code(), ErrorCode::InvalidContent);
    assert_eq!(refused(load(turn_extra)).code(), ErrorCode::InvalidContent);
    assert_eq!(refused(load(bad_type)).code(), ErrorCode::VariantMismatch);
    assert_eq!(
        refused(load(no_conversation)).code(),
        ErrorCode::InvalidContent
    );
    assert_eq!(
        refused(load(output_extra)).code(),
        ErrorCode::InvalidContent
    );
    let duplicate = r#"{"schema":"br-llm-messages-projected-thread/1","schema":"x"}"#;
    assert_eq!(
        refused(ProjectedThread::from_json(duplicate)).code(),
        ErrorCode::MalformedJson
    );
}

#[test]
fn given_an_ai_turn_of_another_agent_when_loaded_then_disallowed_kind_at_that_turn() {
    let mut document = document();
    document["agent"] = json!(aid(2).to_string());

    let error = refused(load(document));

    assert_eq!(error.code(), ErrorCode::DisallowedKind);
    assert_eq!(error.location(), Some("turns[1]"));
}

#[test]
fn given_an_output_of_the_target_agent_itself_when_loaded_then_disallowed_kind() {
    let mut document = document();
    document["turns"][2]["entries"][0]["agent"] = json!(aid(1).to_string());

    let error = refused(load(document));

    assert_eq!(error.code(), ErrorCode::DisallowedKind);
    assert_eq!(error.location(), Some("turns[2].entries[0]"));
}

#[test]
fn given_a_repeated_identity_in_the_thread_when_loaded_then_reused_identity() {
    let mut document = document();
    let output = document["turns"][2]["entries"][0].clone();
    document["turns"][3]["entries"] = json!([output]);

    let error = refused(load(document));

    assert_eq!(error.code(), ErrorCode::ReusedIdentity);
}

#[test]
fn given_projected_entries_in_stored_shapes_when_loaded_then_disallowed_kind() {
    let output = document()["turns"][2]["entries"][0].clone();
    let partial = document()["turns"][3]["entries"][0].clone();
    for entry in [output, partial] {
        let in_ai_turn = json!({"type": "ai_turn", "id": tid(5).to_string(),
            "agent": aid(1).to_string(), "author": "A", "entries": [entry.clone()]});
        let in_input_turn = json!({"type": "input_turn", "id": tid(6).to_string(),
            "entries": [entry.clone()]});
        let in_conversation = json!({"schema": "br-llm-messages/5", "id": cvid(1).to_string(),
            "turns": [in_input_turn.clone()]});

        assert_eq!(
            refused(AITurn::from_value(oj(in_ai_turn))).code(),
            ErrorCode::DisallowedKind
        );
        assert_eq!(
            refused(Turn::from_value(oj(in_input_turn))).code(),
            ErrorCode::DisallowedKind
        );
        assert_eq!(
            refused(Conversation::from_value(oj(in_conversation))).code(),
            ErrorCode::DisallowedKind
        );
        assert_eq!(
            refused(TurnEntry::from_value(oj(entry.clone()))).code(),
            ErrorCode::DisallowedKind
        );
        assert_eq!(
            refused(IncomingEntry::from_value(oj(entry))).code(),
            ErrorCode::DisallowedKind
        );
    }
}
