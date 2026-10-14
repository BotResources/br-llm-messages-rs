//! Tool call ids are unique in the conversation: a step is refused when one
//! of its call ids is already used by a call of any turn.

use super::*;
use crate::block::{AssistantBlock, ToolCall};
use crate::render::Perspective;
use crate::stop_reason::StopReason;
use crate::value::{Author, Text, ToolCallId, ToolName};

fn agent(name: &str) -> Author {
    Author::new(format!("id-{name}"), name).unwrap()
}

fn await_step(ids: &[&str]) -> Step {
    let calls = ids
        .iter()
        .map(|id| {
            AssistantBlock::ToolCall(ToolCall {
                id: ToolCallId::new(*id).unwrap(),
                name: ToolName::new("search").unwrap(),
                arguments: serde_json::json!({}),
            })
        })
        .collect();
    Step::new(calls, StopReason::AwaitingToolResults, None, None).unwrap()
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

fn result(id: &str) -> ToolResult {
    ToolResult::new(
        ToolCallId::new(id).unwrap(),
        ToolName::new("search").unwrap(),
        Vec::new(),
        false,
    )
}

fn turn_id(id: &str) -> TurnId {
    TurnId::new(id).unwrap()
}

fn open_turn(id: &str, who: &str, calls: &[&str]) -> Turn {
    Turn::new(turn_id(id), Some(agent(who)), await_step(calls))
}

fn duplicate(id: &str) -> MessageError {
    MessageError::DuplicateToolCallIdInConversation {
        id: ToolCallId::new(id).unwrap(),
    }
}

/// Turn t1 of agent-a with calls c1 and c2, both answered, awaiting a step.
fn conversation_with_answered_turn() -> Conversation {
    let mut conversation = Conversation::new();
    conversation
        .push_turn(open_turn("t1", "agent-a", &["c1", "c2"]))
        .unwrap();
    conversation
        .push_result(&turn_id("t1"), result("c1"))
        .unwrap();
    conversation
        .push_result(&turn_id("t1"), result("c2"))
        .unwrap();
    conversation
}

#[test]
fn given_call_id_of_another_turn_when_push_turn_then_duplicate_in_conversation() {
    let mut conversation = conversation_with_answered_turn();
    let before = conversation.clone();
    assert_eq!(
        conversation.push_turn(open_turn("t2", "agent-b", &["c9", "c2"])),
        Err(duplicate("c2"))
    );
    assert_eq!(conversation, before);
    assert_eq!(json(&conversation), json(&before));
}

#[test]
fn given_multi_step_turn_whose_later_step_reuses_an_id_when_push_turn_then_refused() {
    let mut conversation = conversation_with_answered_turn();
    let mut turn = open_turn("t2", "agent-b", &["c3"]);
    turn.push_result(result("c3")).unwrap();
    turn.push_step(await_step(&["c1"])).unwrap();
    assert_eq!(conversation.push_turn(turn), Err(duplicate("c1")));
    assert_eq!(conversation.entries().len(), 1);
}

#[test]
fn given_call_id_of_another_turn_when_push_step_then_duplicate_in_conversation() {
    let mut conversation = conversation_with_answered_turn();
    conversation
        .push_turn(open_turn("t2", "agent-b", &["c3"]))
        .unwrap();
    conversation
        .push_result(&turn_id("t2"), result("c3"))
        .unwrap();
    let before = conversation.clone();
    assert_eq!(
        conversation.push_step(&turn_id("t2"), await_step(&["c4", "c1"])),
        Err(duplicate("c1"))
    );
    assert_eq!(conversation, before);
}

#[test]
fn given_call_id_of_the_same_turn_when_push_step_then_the_turn_rule_is_reported() {
    let mut conversation = conversation_with_answered_turn();
    assert_eq!(
        conversation.push_step(&turn_id("t1"), await_step(&["c1"])),
        Err(MessageError::DuplicateToolCallIdInTurn {
            id: ToolCallId::new("c1").unwrap()
        })
    );
}

#[test]
fn given_turn_not_awaiting_a_step_when_push_step_reusing_an_id_then_the_state_is_reported() {
    let mut conversation = conversation_with_answered_turn();
    conversation
        .push_turn(open_turn("t2", "agent-b", &["c3"]))
        .unwrap();
    assert!(matches!(
        conversation.push_step(&turn_id("t2"), await_step(&["c1"])),
        Err(MessageError::TurnNotAwaitingStep { .. })
    ));
}

#[test]
fn given_refused_step_when_a_fresh_step_follows_then_accepted() {
    let mut conversation = conversation_with_answered_turn();
    conversation
        .push_step(&turn_id("t1"), await_step(&["c1"]))
        .unwrap_err();
    conversation
        .push_step(&turn_id("t1"), await_step(&["c5"]))
        .unwrap();
    conversation
        .push_result(&turn_id("t1"), result("c5"))
        .unwrap();
    conversation.push_step(&turn_id("t1"), end_step()).unwrap();
    assert_eq!(conversation.open_turns().count(), 0);
}

#[test]
fn given_unknown_turn_when_push_step_then_turn_not_found() {
    let mut conversation = conversation_with_answered_turn();
    assert!(matches!(
        conversation.push_step(&turn_id("ghost"), await_step(&["c1"])),
        Err(MessageError::TurnNotFound { .. })
    ));
}

#[test]
fn given_two_turns_sharing_a_call_id_when_loaded_then_refused_with_the_rule() {
    let turn = |id: &str| {
        serde_json::json!({
            "type": "turn",
            "id": id,
            "items": [
                {
                    "type": "step",
                    "content": [ { "type": "tool_call", "id": "c1", "name": "search", "arguments": {} } ],
                    "stop_reason": { "type": "awaiting_tool_results" }
                }
            ]
        })
    };
    let document = serde_json::json!({
        "schema": crate::SCHEMA_VERSION,
        "entries": [ turn("t1"), turn("t2") ]
    });
    let error = serde_json::from_value::<Conversation>(document).unwrap_err();
    assert!(
        error.to_string().contains(&duplicate("c1").to_string()),
        "{error}"
    );
}

#[test]
fn given_conversation_holding_a_tool_name_a_provider_refuses_when_loaded_then_refused() {
    let document = serde_json::json!({
        "schema": crate::SCHEMA_VERSION,
        "entries": [
            {
                "type": "turn",
                "id": "t1",
                "items": [
                    {
                        "type": "step",
                        "content": [ { "type": "tool_call", "id": "c1", "name": "mcp.search", "arguments": {} } ],
                        "stop_reason": { "type": "awaiting_tool_results" }
                    }
                ]
            }
        ]
    });
    let error = serde_json::from_value::<Conversation>(document).unwrap_err();
    assert!(
        error
            .to_string()
            .contains(&MessageError::InvalidToolName.to_string()),
        "{error}"
    );
}

#[test]
fn given_lightened_conversation_when_pushing_an_id_of_an_own_turn_then_still_refused() {
    let mut conversation = conversation_with_answered_turn();
    conversation
        .push_turn(open_turn("t2", "agent-b", &["c3"]))
        .unwrap();
    let mut lightened = crate::lighten(&conversation, &Perspective::new(agent("agent-a")));

    assert_eq!(
        lightened.push_turn(open_turn("t3", "agent-c", &["c2"])),
        Err(duplicate("c2"))
    );
    lightened
        .push_turn(open_turn("t4", "agent-c", &["c3"]))
        .unwrap();
}

fn json(conversation: &Conversation) -> String {
    serde_json::to_string(conversation).unwrap()
}
