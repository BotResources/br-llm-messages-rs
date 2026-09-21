use super::test_helpers::*;
use super::*;

use crate::block::{AssistantBlock, RedactedThinking, Thinking};
use crate::step::Step;
use crate::stop_reason::StopReason;
use crate::value::TurnId;

#[test]
fn given_two_own_turns_back_to_back_when_rendered_then_each_step_verbatim_assistant() {
    let first = end_step();
    let second = call_step("c1");
    let turn_one = Turn::new(TurnId::new("t1").unwrap(), Some(agent()), first.clone());
    let mut turn_two = Turn::new(TurnId::new("t2").unwrap(), Some(agent()), second.clone());
    turn_two.push_result(result("c1")).unwrap();
    turn_two.push_step(end_step()).unwrap();

    let mut conversation = Conversation::new();
    conversation.push_input(human("question"));
    conversation.push_turn(turn_one).unwrap();
    conversation.push_turn(turn_two).unwrap();

    let wire = render(&conversation, &perspective()).unwrap();
    assert!(matches!(wire[0], WireMessage::User { .. }));
    assert_eq!(
        wire[1],
        WireMessage::Assistant {
            content: first.content().to_vec()
        }
    );
    assert_eq!(
        wire[2],
        WireMessage::Assistant {
            content: second.content().to_vec()
        }
    );
    assert!(matches!(
        (&wire[1], &wire[2]),
        (WireMessage::Assistant { .. }, WireMessage::Assistant { .. })
    ));
}

#[test]
fn given_own_step_with_redacted_and_structured_when_rendered_then_content_verbatim() {
    let step = Step::new(
        vec![
            AssistantBlock::Thinking(Thinking {
                text: "reasoning".to_owned(),
                signature: None,
            }),
            AssistantBlock::RedactedThinking(RedactedThinking::new("enc==").unwrap()),
            AssistantBlock::Text {
                text: Text::new("the answer").unwrap(),
            },
            AssistantBlock::Structured {
                value: serde_json::json!({ "answer": 42 }),
            },
        ],
        StopReason::EndTurn,
        None,
        None,
    )
    .unwrap();
    let turn = Turn::new(TurnId::new("t1").unwrap(), Some(agent()), step.clone());

    let mut conversation = Conversation::new();
    conversation.push_input(human("question"));
    conversation.push_turn(turn).unwrap();

    let wire = render(&conversation, &perspective()).unwrap();
    assert_eq!(
        wire[1],
        WireMessage::Assistant {
            content: step.content().to_vec()
        }
    );
    match &wire[1] {
        WireMessage::Assistant { content } => {
            assert!(matches!(content[1], AssistantBlock::RedactedThinking(_)));
            assert!(matches!(content[3], AssistantBlock::Structured { .. }));
        }
        WireMessage::User { .. } | WireMessage::Relay { .. } => {
            panic!("expected the own step as a verbatim assistant message")
        }
    }
}
