use super::*;
use crate::block::{AssistantBlock, ToolCall, UserBlock};
use crate::stop_reason::StopReason;
use crate::user_input::UserSource;
use crate::value::{Author, Text, ToolCallId, ToolName};

fn input(body: &str) -> UserInput {
    UserInput::new(
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
        Some(Author::new(agent).unwrap()),
        await_step(call),
    )
}

#[test]
fn given_duplicate_turn_id_when_push_turn_then_refused() {
    let mut conversation = Conversation::new();
    conversation
        .push_turn(open_turn("t1", "agent-a", "c1"))
        .unwrap();
    assert!(matches!(
        conversation.push_turn(open_turn("t1", "agent-b", "c2")),
        Err(MessageError::DuplicateTurnId { .. })
    ));
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
    conversation.push_input(input("first"));
    conversation.push_input(input("second"));
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
    conversation.push_input(input("hi"));
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
    assert!(serde_json::from_value::<Conversation>(json).is_err());
}

#[test]
fn given_conversation_when_round_tripped_then_identical() {
    let mut conversation = Conversation::new();
    conversation.push_input(input("hello"));
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
