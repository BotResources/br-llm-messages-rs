use super::test_helpers::*;
use super::*;

use crate::user_input::{UserInput, UserSource};
use crate::value::{Author, TurnId};

#[test]
fn given_own_turn_written_under_an_old_name_when_rendered_then_still_own_by_id() {
    let before_rename = Author::new("id-agent-a", "Old name").unwrap();
    let mut conversation = Conversation::new();
    conversation.push_input(human("question"));
    conversation
        .push_turn(Turn::new(
            TurnId::new("t1").unwrap(),
            Some(before_rename),
            end_step(),
        ))
        .unwrap();

    let renamed = Perspective::new(Author::new("id-agent-a", "New name").unwrap());
    let wire = render(&conversation, &renamed).unwrap();
    assert_eq!(wire.len(), 2);
    assert_eq!(
        wire[1],
        WireMessage::Assistant {
            content: end_step().content().to_vec()
        }
    );
}

#[test]
fn given_other_agent_with_the_perspective_name_when_rendered_then_framed() {
    let namesake = Author::new("id-agent-z", "agent-a").unwrap();
    let mut conversation = Conversation::new();
    conversation.push_input(human("question"));
    conversation
        .push_turn(Turn::new(
            TurnId::new("t1").unwrap(),
            Some(namesake),
            end_step(),
        ))
        .unwrap();

    let wire = render(&conversation, &perspective()).unwrap();
    assert_eq!(wire.len(), 1);
    let framed = frame_text(&wire[0]);
    assert!(framed.contains("<message author=\"agent-a\" role=\"agent\">the answer</message>"));
}

#[test]
fn given_other_agent_turn_when_rendered_then_frame_shows_name_not_id() {
    let mut conversation = Conversation::new();
    conversation.push_input(human("question"));
    conversation
        .push_turn(Turn::new(
            TurnId::new("t1").unwrap(),
            Some(Author::new("0192-agent-b", "Agent B").unwrap()),
            end_step(),
        ))
        .unwrap();

    let wire = render(&conversation, &perspective()).unwrap();
    let framed = frame_text(&wire[0]);
    assert!(framed.contains("author=\"Agent B\""));
    assert!(!framed.contains("0192-agent-b"));
}

#[test]
fn given_input_with_author_when_rendered_then_frame_shows_name_not_id() {
    let input = UserInput::new(
        UserSource::Human,
        Some(Author::new("0192-user-1", "Alice").unwrap()),
        vec![crate::block::UserBlock::text(Text::new("hi").unwrap())],
    )
    .unwrap();
    let mut conversation = Conversation::new();
    conversation.push_input(input);

    let wire = render(&conversation, &perspective()).unwrap();
    assert_eq!(
        frame_text(&wire[0]),
        "<message author=\"Alice\" role=\"user\">hi</message>"
    );
}
