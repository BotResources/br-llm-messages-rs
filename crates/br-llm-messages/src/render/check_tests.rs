use super::*;
use crate::block::{Thinking, ToolResult};
use crate::value::{Text, ToolName};

fn user_text(body: &str) -> WireUserBlock {
    WireUserBlock::Text {
        text: Text::new(body).unwrap(),
    }
}

fn answer(id: &str) -> WireUserBlock {
    WireUserBlock::ToolResult(ToolResult::new(
        ToolCallId::new(id).unwrap(),
        ToolName::new("search").unwrap(),
        Vec::new(),
        false,
    ))
}

fn call(id: &str) -> AssistantBlock {
    AssistantBlock::ToolCall(ToolCall {
        id: ToolCallId::new(id).unwrap(),
        name: ToolName::new("search").unwrap(),
        arguments: serde_json::json!({}),
    })
}

fn said(body: &str) -> AssistantBlock {
    AssistantBlock::Text {
        text: Text::new(body).unwrap(),
    }
}

fn user(content: Vec<WireUserBlock>) -> WireMessage {
    WireMessage::User { content }
}

fn assistant(content: Vec<AssistantBlock>) -> WireMessage {
    WireMessage::Assistant { content }
}

fn relay(body: &str) -> WireMessage {
    WireMessage::Relay {
        content: vec![Text::new(body).unwrap()],
    }
}

fn id(value: &str) -> ToolCallId {
    ToolCallId::new(value).unwrap()
}

#[test]
fn given_question_calls_answers_and_reply_when_checked_then_accepted() {
    let list = vec![
        user(vec![user_text("question")]),
        assistant(vec![
            AssistantBlock::Thinking(Thinking {
                text: String::new(),
                signature: None,
            }),
            said("calling"),
            call("c1"),
            call("c2"),
        ]),
        user(vec![answer("c2"), user_text("more"), answer("c1")]),
        assistant(vec![said("the answer")]),
        assistant(vec![said("and more")]),
        user(vec![user_text("thanks")]),
        relay("a framed entry"),
    ];
    assert_eq!(check_wire_list(&list), Ok(()));
}

#[test]
fn given_empty_list_when_checked_then_wire_list_empty() {
    assert_eq!(check_wire_list(&[]), Err(MessageError::WireListEmpty));
}

#[test]
fn given_list_starting_with_assistant_or_relay_when_checked_then_not_starting_with_user() {
    for first in [assistant(vec![said("hi")]), relay("hi")] {
        let list = vec![first, user(vec![user_text("question")])];
        assert_eq!(
            check_wire_list(&list),
            Err(MessageError::WireListNotStartingWithUser { message: 0 })
        );
    }
}

#[test]
fn given_call_without_result_in_the_next_user_message_when_checked_then_unanswered() {
    let list = vec![
        user(vec![user_text("question")]),
        assistant(vec![said("calling"), call("c1"), call("c2")]),
        user(vec![answer("c1"), user_text("no answer for c2")]),
    ];
    assert_eq!(
        check_wire_list(&list),
        Err(MessageError::WireToolCallUnanswered {
            message: 1,
            block: 2,
            id: id("c2"),
        })
    );
}

#[test]
fn given_call_followed_by_no_user_message_when_checked_then_unanswered() {
    let calling = || assistant(vec![call("c1")]);
    let cases = [
        vec![user(vec![user_text("q")]), calling()],
        vec![user(vec![user_text("q")]), calling(), relay("late")],
        vec![
            user(vec![user_text("q")]),
            calling(),
            assistant(vec![said("x")]),
        ],
    ];
    for list in cases {
        assert_eq!(
            check_wire_list(&list),
            Err(MessageError::WireToolCallUnanswered {
                message: 1,
                block: 0,
                id: id("c1"),
            })
        );
    }
}

#[test]
fn given_call_answered_two_messages_later_when_checked_then_unanswered() {
    let list = vec![
        user(vec![user_text("q")]),
        assistant(vec![call("c1")]),
        relay("in between"),
        user(vec![answer("c1")]),
    ];
    assert_eq!(
        check_wire_list(&list),
        Err(MessageError::WireToolCallUnanswered {
            message: 1,
            block: 0,
            id: id("c1"),
        })
    );
}

#[test]
fn given_call_answered_twice_when_checked_then_answered_twice_at_the_second_result() {
    let list = vec![
        user(vec![user_text("q")]),
        assistant(vec![call("c1")]),
        user(vec![answer("c1"), user_text("x"), answer("c1")]),
    ];
    assert_eq!(
        check_wire_list(&list),
        Err(MessageError::WireToolCallAnsweredTwice {
            message: 2,
            block: 2,
            id: id("c1"),
        })
    );
}

#[test]
fn given_result_answering_no_call_of_the_previous_assistant_when_checked_then_unmatched() {
    let list = vec![
        user(vec![user_text("q")]),
        assistant(vec![call("c1")]),
        user(vec![answer("c1"), answer("c9")]),
    ];
    assert_eq!(
        check_wire_list(&list),
        Err(MessageError::WireToolResultUnmatched {
            message: 2,
            block: 1,
            id: id("c9"),
        })
    );
}

#[test]
fn given_result_in_the_first_message_when_checked_then_unmatched() {
    let list = vec![user(vec![user_text("q"), answer("c1")])];
    assert_eq!(
        check_wire_list(&list),
        Err(MessageError::WireToolResultUnmatched {
            message: 0,
            block: 1,
            id: id("c1"),
        })
    );
}

#[test]
fn given_result_after_a_user_or_relay_message_when_checked_then_unmatched() {
    for before in [user(vec![user_text("x")]), relay("x")] {
        let list = vec![
            user(vec![user_text("q")]),
            assistant(vec![call("c1")]),
            user(vec![answer("c1")]),
            before,
            user(vec![answer("c1")]),
        ];
        assert_eq!(
            check_wire_list(&list),
            Err(MessageError::WireToolResultUnmatched {
                message: 4,
                block: 0,
                id: id("c1"),
            })
        );
    }
}

#[test]
fn given_call_id_repeated_in_one_message_when_checked_then_repeated_at_the_second_call() {
    let list = vec![
        user(vec![user_text("q")]),
        assistant(vec![said("x"), call("c1"), call("c1")]),
        user(vec![answer("c1")]),
    ];
    assert_eq!(
        check_wire_list(&list),
        Err(MessageError::WireToolCallIdRepeated {
            message: 1,
            block: 2,
            id: id("c1"),
        })
    );
}

#[test]
fn given_call_id_repeated_across_messages_when_checked_then_repeated_at_the_later_call() {
    let list = vec![
        user(vec![user_text("q")]),
        assistant(vec![call("c1")]),
        user(vec![answer("c1")]),
        assistant(vec![said("again"), call("c1")]),
        user(vec![answer("c1")]),
    ];
    assert_eq!(
        check_wire_list(&list),
        Err(MessageError::WireToolCallIdRepeated {
            message: 3,
            block: 1,
            id: id("c1"),
        })
    );
}

#[test]
fn given_invalid_list_in_json_when_loaded_then_loads_and_only_the_check_refuses_it() {
    let json = serde_json::json!([
        { "role": "assistant", "content": [
            { "type": "tool_call", "id": "c1", "name": "search", "arguments": {} }
        ] },
        { "role": "assistant", "content": [] },
        { "role": "user", "content": [
            { "type": "tool_result", "tool_call_id": "c9", "tool_name": "search", "is_error": false }
        ] }
    ]);
    let list: Vec<WireMessage> = serde_json::from_value(json).unwrap();
    assert_eq!(list.len(), 3);
    assert_eq!(
        check_wire_list(&list),
        Err(MessageError::WireListNotStartingWithUser { message: 0 })
    );
}

#[test]
fn given_a_refusal_about_a_block_when_displayed_then_names_the_message_and_the_block() {
    let text = MessageError::WireToolCallUnanswered {
        message: 3,
        block: 1,
        id: id("c7"),
    }
    .to_string();
    assert!(text.contains("c7") && text.contains("message 3") && text.contains("block 1"));
}
