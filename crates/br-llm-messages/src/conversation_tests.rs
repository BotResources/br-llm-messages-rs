use super::*;
use crate::block::{AssistantBlock, ToolCall, UserBlock};
use crate::stop_reason::StopReason;
use crate::user_input::UserSource;
use crate::value::{Author, InputId, Text, ToolCallId, ToolName};

fn input(body: &str) -> UserInput {
    UserInput::new(
        InputId::new(format!("in-{body}")).unwrap(),
        UserSource::Human,
        None,
        vec![UserBlock::text(Text::new(body).unwrap())],
    )
    .unwrap()
}

fn await_step(id: &str) -> Step {
    Step::new(
        vec![AssistantBlock::ToolCall(ToolCall {
            id: ToolCallId::new(id).unwrap(),
            name: ToolName::new("search").unwrap(),
            arguments: serde_json::json!({}),
        })],
        StopReason::AwaitingToolResults,
        None,
        None,
    )
    .unwrap()
}

fn end_step() -> Step {
    Step::new(
        vec![AssistantBlock::Text {
            text: Text::new("done").unwrap(),
        }],
        StopReason::EndTurn,
        None,
        None,
    )
    .unwrap()
}

fn open_turn(turn_id: &str, agent: &str, call: &str) -> Turn {
    Turn::new(
        TurnId::new(turn_id).unwrap(),
        Some(Author::new(format!("id-{agent}"), agent).unwrap()),
        await_step(call),
    )
}

#[test]
fn given_duplicate_turn_id_when_push_turn_then_refused() {
    let mut conversation = Conversation::new();
    conversation
        .push_turn(open_turn("t1", "agent-a", "c1"))
        .unwrap();
    assert_eq!(
        conversation.push_turn(open_turn("t1", "agent-b", "c2")),
        Err(MessageError::DuplicateEntryId {
            id: "t1".to_owned()
        })
    );
}

#[test]
fn given_unknown_turn_when_push_result_then_not_found() {
    let mut conversation = Conversation::new();
    let result = ToolResult::new(
        ToolCallId::new("c1").unwrap(),
        ToolName::new("search").unwrap(),
        Vec::new(),
        false,
    );
    assert!(matches!(
        conversation.push_result(&TurnId::new("ghost").unwrap(), result),
        Err(MessageError::TurnNotFound { .. })
    ));
}

#[test]
fn given_two_parallel_agents_when_open_turns_then_both_listed() {
    let mut conversation = Conversation::new();
    conversation
        .push_turn(open_turn("t1", "agent-a", "c1"))
        .unwrap();
    conversation
        .push_turn(open_turn("t2", "agent-b", "c2"))
        .unwrap();
    assert_eq!(conversation.open_turns().count(), 2);
    conversation
        .push_result(
            &TurnId::new("t1").unwrap(),
            ToolResult::new(
                ToolCallId::new("c1").unwrap(),
                ToolName::new("search").unwrap(),
                Vec::new(),
                false,
            ),
        )
        .unwrap();
    conversation
        .push_step(&TurnId::new("t1").unwrap(), end_step())
        .unwrap();
    assert_eq!(conversation.open_turns().count(), 1);
}

#[test]
fn given_two_users_and_two_agents_in_a_row_when_pushed_then_order_preserved() {
    let mut conversation = Conversation::new();
    conversation.push_input(input("first")).unwrap();
    conversation.push_input(input("second")).unwrap();
    conversation
        .push_turn(open_turn("t1", "agent-a", "c1"))
        .unwrap();
    conversation
        .push_turn(open_turn("t2", "agent-b", "c2"))
        .unwrap();
    assert_eq!(conversation.entries().len(), 4);
}

#[test]
fn given_conversation_when_serialized_then_carries_schema() {
    let mut conversation = Conversation::new();
    conversation.push_input(input("hi")).unwrap();
    let json = serde_json::to_value(&conversation).unwrap();
    assert_eq!(json["schema"], serde_json::json!(crate::SCHEMA_VERSION));
}

#[test]
fn given_wrong_schema_when_deserialized_then_refused() {
    let json = serde_json::json!({ "schema": "br-llm-messages/999", "entries": [] });
    assert!(serde_json::from_value::<Conversation>(json).is_err());
}

#[test]
fn given_two_turns_with_same_id_when_deserialized_then_refused() {
    let json = serde_json::json!({
        "schema": crate::SCHEMA_VERSION,
        "entries": [
            {
                "type": "turn",
                "id": "t1",
                "items": [
                    {
                        "type": "step",
                        "content": [ { "type": "text", "text": "one" } ],
                        "stop_reason": { "type": "end_turn" }
                    }
                ]
            },
            {
                "type": "turn",
                "id": "t1",
                "items": [
                    {
                        "type": "step",
                        "content": [ { "type": "text", "text": "two" } ],
                        "stop_reason": { "type": "end_turn" }
                    }
                ]
            }
        ]
    });
    let error = serde_json::from_value::<Conversation>(json).unwrap_err();
    let rule = MessageError::DuplicateEntryId {
        id: "t1".to_owned(),
    };
    assert!(error.to_string().contains(&rule.to_string()), "{error}");
}

#[test]
fn given_conversation_when_round_tripped_then_identical() {
    let mut conversation = Conversation::new();
    conversation.push_input(input("hello")).unwrap();
    let mut turn = open_turn("t1", "agent-a", "c1");
    turn.push_result(ToolResult::new(
        ToolCallId::new("c1").unwrap(),
        ToolName::new("search").unwrap(),
        Vec::new(),
        false,
    ))
    .unwrap();
    turn.push_step(end_step()).unwrap();
    conversation.push_turn(turn).unwrap();
    let json = serde_json::to_string(&conversation).unwrap();
    assert_eq!(
        serde_json::from_str::<Conversation>(&json).unwrap(),
        conversation
    );
}

#[test]
fn given_schema_constant_when_read_then_version_three() {
    assert_eq!(crate::SCHEMA_VERSION, "br-llm-messages/3");
}

#[test]
fn given_schema_three_document_with_ids_and_authors_when_deserialized_then_loaded() {
    let json = serde_json::json!({
        "schema": "br-llm-messages/3",
        "entries": [
            {
                "type": "user_input",
                "id": "i1",
                "source": { "type": "human" },
                "author": { "id": "u1", "name": "Alice" },
                "content": [ { "type": "text", "text": "hi" } ]
            },
            {
                "type": "turn",
                "id": "t1",
                "author": { "id": "a1", "name": "Agent" },
                "items": [
                    {
                        "type": "step",
                        "content": [ { "type": "text", "text": "hello" } ],
                        "stop_reason": { "type": "end_turn" }
                    }
                ]
            }
        ]
    });
    let conversation = serde_json::from_value::<Conversation>(json.clone()).unwrap();
    assert_eq!(conversation.entries().len(), 2);
    assert_eq!(conversation.entry("i1"), Some(&conversation.entries()[0]));
    assert_eq!(serde_json::to_value(&conversation).unwrap(), json);
}

#[test]
fn given_earlier_schema_documents_when_deserialized_then_schema_mismatch_reported() {
    for schema in ["br-llm-messages/1", "br-llm-messages/2"] {
        let json = serde_json::json!({ "schema": schema, "entries": [] });
        let message = serde_json::from_value::<Conversation>(json)
            .unwrap_err()
            .to_string();
        assert!(
            message.contains(&format!("unexpected schema \"{schema}\"")),
            "{message}"
        );
    }
}

#[test]
fn given_schema_two_document_of_inputs_without_ids_when_deserialized_then_refused() {
    let json = serde_json::json!({
        "schema": "br-llm-messages/2",
        "entries": [
            {
                "type": "user_input",
                "source": { "type": "human" },
                "content": [ { "type": "text", "text": "hi" } ]
            }
        ]
    });
    assert!(serde_json::from_value::<Conversation>(json).is_err());
}
