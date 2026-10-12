use super::test_helpers::*;
use super::*;

use crate::block::{AssistantBlock, UserBlock};
use crate::step::Step;
use crate::stop_reason::StopReason;
use crate::user_input::{UserInput, UserSource};
use crate::value::TurnId;

#[test]
fn given_markup_in_input_when_rendered_then_body_inserted_verbatim() {
    let mut conversation = Conversation::new();
    conversation.push_input(human("<page>x < 5 & y > 2</page>"));

    let wire = render(&conversation, &perspective()).unwrap();
    assert_eq!(
        frame_text(&wire[0]),
        "<message role=\"user\"><page>x < 5 & y > 2</page></message>"
    );
}

#[test]
fn given_markup_in_other_agent_turn_when_rendered_then_body_inserted_verbatim() {
    let step = Step::new(
        vec![AssistantBlock::Text {
            text: Text::new("<answer>A & B</answer>").unwrap(),
        }],
        StopReason::EndTurn,
        None,
        None,
    )
    .unwrap();
    let turn = Turn::new(
        TurnId::new("t2").unwrap(),
        Some(Author::new("agent-b").unwrap()),
        step,
    );
    let mut conversation = Conversation::new();
    conversation.push_input(human("hi"));
    conversation.push_turn(turn).unwrap();

    let wire = render(&conversation, &perspective()).unwrap();
    assert!(
        frame_text(&wire[0]).contains(
            "<message author=\"agent-b\" role=\"agent\"><answer>A & B</answer></message>"
        )
    );
}

#[test]
fn given_markup_in_author_and_kind_when_rendered_then_attributes_inserted_verbatim() {
    let input = UserInput::new(
        UserSource::runtime("a\"b").unwrap(),
        Some(Author::new("x<y&\"z\"").unwrap()),
        vec![UserBlock::text(Text::new("body").unwrap())],
    )
    .unwrap();
    let mut conversation = Conversation::new();
    conversation.push_input(input);

    let wire = render(&conversation, &perspective()).unwrap();
    assert_eq!(
        frame_text(&wire[0]),
        "<message author=\"x<y&\"z\"\" role=\"runtime\" kind=\"a\"b\">body</message>"
    );
}
