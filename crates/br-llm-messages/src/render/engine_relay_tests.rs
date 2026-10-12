use super::test_helpers::*;
use super::*;

use crate::user_input::{UserInput, UserSource};
use crate::value::{Base64Data, ImageMime, TurnId};

#[test]
fn given_trailing_input_after_open_turn_when_rendered_then_relay_after_results() {
    let mut turn = Turn::new(TurnId::new("t1").unwrap(), Some(agent()), call_step("c1"));
    turn.push_result(result("c1")).unwrap();

    let mut conversation = Conversation::new();
    conversation.push_input(human("question"));
    conversation.push_turn(turn).unwrap();
    conversation.push_input(human("meanwhile"));

    let wire = render(&conversation, &perspective()).unwrap();
    assert_eq!(wire.len(), 4);
    assert!(matches!(wire[0], WireMessage::User { .. }));
    assert!(matches!(wire[1], WireMessage::Assistant { .. }));
    match &wire[2] {
        WireMessage::User { content } => {
            assert!(matches!(content.as_slice(), [WireUserBlock::ToolResult(_)]));
        }
        WireMessage::Assistant { .. } | WireMessage::Relay { .. } => {
            panic!("expected the tool-results user message")
        }
    }
    match &wire[3] {
        WireMessage::Relay { content } => {
            assert_eq!(content.len(), 1);
            assert!(content[0].as_str().contains("meanwhile"));
        }
        WireMessage::User { .. } | WireMessage::Assistant { .. } => {
            panic!("expected a relay message")
        }
    }
}

#[test]
fn given_several_trailing_inputs_after_open_turn_when_rendered_then_single_relay() {
    let mut turn = Turn::new(TurnId::new("t1").unwrap(), Some(agent()), call_step("c1"));
    turn.push_result(result("c1")).unwrap();

    let mut conversation = Conversation::new();
    conversation.push_input(human("question"));
    conversation.push_turn(turn).unwrap();
    conversation.push_input(human("first meanwhile"));
    conversation.push_input(human("second meanwhile"));

    let wire = render(&conversation, &perspective()).unwrap();
    let relays: Vec<&WireMessage> = wire
        .iter()
        .filter(|message| matches!(message, WireMessage::Relay { .. }))
        .collect();
    assert_eq!(relays.len(), 1);
    match relays[0] {
        WireMessage::Relay { content } => {
            assert_eq!(content.len(), 2);
            assert!(content[0].as_str().contains("first meanwhile"));
            assert!(content[1].as_str().contains("second meanwhile"));
        }
        WireMessage::User { .. } | WireMessage::Assistant { .. } => unreachable!(),
    }
}

#[test]
fn given_relayed_input_with_image_when_rendered_then_image_on_preceding_user() {
    let mut turn = Turn::new(TurnId::new("t1").unwrap(), Some(agent()), call_step("c1"));
    turn.push_result(result("c1")).unwrap();

    let relayed = UserInput::new(
        UserSource::Human,
        None,
        vec![
            crate::block::UserBlock::text(Text::new("look here").unwrap()),
            crate::block::UserBlock::image(ImageMime::Png, Base64Data::new("aGVsbG8=").unwrap()),
        ],
    )
    .unwrap();

    let mut conversation = Conversation::new();
    conversation.push_input(human("question"));
    conversation.push_turn(turn).unwrap();
    conversation.push_input(relayed);

    let wire = render(&conversation, &perspective()).unwrap();
    assert_eq!(wire.len(), 4);
    match &wire[2] {
        WireMessage::User { content } => {
            assert!(matches!(content[0], WireUserBlock::ToolResult(_)));
            assert!(matches!(content[1], WireUserBlock::Image(_)));
        }
        WireMessage::Assistant { .. } | WireMessage::Relay { .. } => {
            panic!("expected the tool-results user message with the appended image")
        }
    }
    match &wire[3] {
        WireMessage::Relay { content } => {
            assert_eq!(content.len(), 1);
            assert!(content[0].as_str().contains("look here"));
        }
        WireMessage::User { .. } | WireMessage::Assistant { .. } => {
            panic!("expected a relay message")
        }
    }
}

#[test]
fn given_other_agent_turn_arriving_during_own_open_turn_when_rendered_then_folded_into_relay() {
    let mut own = Turn::new(TurnId::new("t1").unwrap(), Some(agent()), call_step("c1"));
    own.push_result(result("c1")).unwrap();

    let mut other = Turn::new(
        TurnId::new("t2").unwrap(),
        Some(Author::new("agent-b").unwrap()),
        call_step("c9"),
    );
    other.push_result(result("c9")).unwrap();
    other.push_step(end_step()).unwrap();

    let mut conversation = Conversation::new();
    conversation.push_input(human("q"));
    conversation.push_turn(own).unwrap();
    conversation.push_turn(other).unwrap();

    let wire = render(&conversation, &perspective()).unwrap();
    assert!(matches!(wire.first(), Some(WireMessage::User { .. })));
    match wire.last().unwrap() {
        WireMessage::Relay { content } => {
            assert_eq!(content.len(), 1);
            let framed = content[0].as_str();
            assert!(framed.contains("role=\"agent\""));
            assert!(framed.contains("author=\"agent-b\""));
            assert!(framed.contains("the answer"));
        }
        WireMessage::User { .. } | WireMessage::Assistant { .. } => {
            panic!("expected the intervening agent turn folded into a relay")
        }
    }
}

#[test]
fn given_other_agent_mid_flight_during_own_open_turn_when_rendered_then_calls_dropped_from_relay() {
    let mut own = Turn::new(TurnId::new("t1").unwrap(), Some(agent()), call_step("c1"));
    own.push_result(result("c1")).unwrap();

    let other = Turn::new(
        TurnId::new("t2").unwrap(),
        Some(Author::new("agent-b").unwrap()),
        call_step("c9"),
    );
    assert!(matches!(
        other.state(),
        TurnState::AwaitingToolResults { .. }
    ));

    let mut conversation = Conversation::new();
    conversation.push_input(human("q"));
    conversation.push_turn(own).unwrap();
    conversation.push_turn(other).unwrap();

    let wire = render(&conversation, &perspective()).unwrap();
    match wire.last().unwrap() {
        WireMessage::Relay { content } => {
            assert_eq!(content.len(), 1);
            let framed = content[0].as_str();
            assert!(framed.contains("role=\"agent\""));
            assert!(framed.contains("author=\"agent-b\""));
            assert!(framed.contains("calling a tool"));
            assert!(!framed.contains("reasoning"));
            assert!(!framed.contains("search"));
        }
        WireMessage::User { .. } | WireMessage::Assistant { .. } => {
            panic!("expected the intervening mid-flight agent turn folded into a relay")
        }
    }
}

#[test]
fn given_trailing_input_after_finished_own_turn_when_rendered_then_opens_user_not_relay() {
    let mut conversation = Conversation::new();
    conversation.push_input(human("question"));
    conversation.push_turn(own_turn_finished()).unwrap();
    conversation.push_input(human("afterwards"));

    let wire = render(&conversation, &perspective()).unwrap();
    assert!(
        wire.iter()
            .all(|message| !matches!(message, WireMessage::Relay { .. }))
    );
    let last = wire.last().unwrap();
    assert!(matches!(last, WireMessage::User { .. }));
    assert!(frame_text(last).contains("afterwards"));
}
