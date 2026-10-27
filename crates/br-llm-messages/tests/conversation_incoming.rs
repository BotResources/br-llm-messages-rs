mod common;

use br_llm_messages::{
    AgentOutput, ClosureReason, Conversation, ErrorCode, IncomingEntry, InputTurn, Turn,
};
use common::turns::{close, finished, human_message, input_turn, open_turn, tid};
use common::{aid, at, author, cvid, eid, plain_text, refused};

fn empty() -> Conversation {
    Conversation::new(cvid(1))
}

fn ids(conversation: &Conversation) -> Vec<br_llm_messages::TurnId> {
    conversation.turns().iter().map(Turn::id).collect()
}

#[test]
fn given_an_empty_conversation_when_an_incoming_entry_is_pushed_then_a_new_input_turn_takes_the_given_identity()
 {
    let mut conversation = empty();

    conversation
        .push_incoming(human_message(11), tid(10))
        .unwrap();

    assert_eq!(ids(&conversation), [tid(10)]);
    assert_eq!(
        conversation
            .turn(&tid(10))
            .unwrap()
            .as_input()
            .unwrap()
            .entries()
            .len(),
        1
    );
    assert!(conversation.entry(&eid(11)).is_ok());
}

#[test]
fn given_a_last_input_turn_when_incoming_entries_are_pushed_then_they_join_it_in_order_and_the_given_turn_is_unused()
 {
    let mut conversation = empty();
    conversation.push_turn(input_turn(10, 11).into()).unwrap();

    conversation
        .push_incoming(human_message(12), tid(20))
        .unwrap();
    conversation
        .push_incoming(human_message(13), tid(21))
        .unwrap();

    assert_eq!(ids(&conversation), [tid(10)]);
    let entries: Vec<_> = conversation
        .turn(&tid(10))
        .unwrap()
        .as_input()
        .unwrap()
        .entries()
        .iter()
        .map(IncomingEntry::id)
        .collect();
    assert_eq!(entries, [eid(11), eid(12), eid(13)]);
    assert!(conversation.turn(&tid(20)).is_err());
    assert!(conversation.entry(&eid(13)).is_ok());
}

#[test]
fn given_an_input_turn_with_an_observed_time_when_an_entry_is_pushed_then_the_time_is_kept() {
    let mut conversation = empty();
    let turn = InputTurn::new(tid(10), vec![human_message(11).into()])
        .unwrap()
        .with_observed_at(at(4));
    conversation.push_turn(turn.into()).unwrap();

    conversation
        .push_incoming(human_message(12), tid(20))
        .unwrap();

    assert_eq!(
        conversation.turn(&tid(10)).unwrap().observed_at(),
        Some(&at(4))
    );
}

#[test]
fn given_a_last_open_ai_turn_when_an_incoming_entry_is_pushed_then_it_joins_that_turn() {
    let mut conversation = empty();
    conversation.push_turn(open_turn(10).into()).unwrap();
    conversation.push_entry(&tid(10), finished(11)).unwrap();

    conversation
        .push_incoming(human_message(12), tid(20))
        .unwrap();

    assert_eq!(ids(&conversation), [tid(10)]);
    assert_eq!(
        conversation
            .turn(&tid(10))
            .unwrap()
            .as_ai()
            .unwrap()
            .entries()
            .len(),
        2
    );
}

#[test]
fn given_a_last_closed_ai_turn_when_an_incoming_entry_is_pushed_then_a_new_input_turn_is_made() {
    let mut conversation = empty();
    conversation.push_turn(open_turn(10).into()).unwrap();
    conversation
        .push_entry(&tid(10), close(11, ClosureReason::completed()))
        .unwrap();

    conversation
        .push_incoming(human_message(12), tid(20))
        .unwrap();

    assert_eq!(ids(&conversation), [tid(10), tid(20)]);
    assert_eq!(
        conversation
            .turn(&tid(10))
            .unwrap()
            .as_ai()
            .unwrap()
            .entries()
            .len(),
        1
    );
}

#[test]
fn given_refused_incoming_entries_when_pushed_then_nothing_changes() {
    let mut conversation = empty();
    conversation.push_turn(input_turn(10, 11).into()).unwrap();
    let mut closed = empty();
    closed.push_turn(open_turn(10).into()).unwrap();
    closed
        .push_entry(&tid(10), close(11, ClosureReason::completed()))
        .unwrap();
    let output =
        || AgentOutput::new(eid(30), aid(2), author("A"), vec![plain_text(31, "x")]).unwrap();
    let before = (conversation.clone(), closed.clone());

    let reused_entry = conversation.push_incoming(human_message(11), tid(20));
    let reused_in_new_turn = closed.push_incoming(human_message(11), tid(20));
    let reused_turn_id = closed.push_incoming(human_message(12), tid(10));
    let output_on_input = conversation.push_incoming(output(), tid(20));
    let output_on_new_turn = closed.push_incoming(output(), tid(20));

    assert_eq!(refused(reused_entry).code(), ErrorCode::ReusedIdentity);
    assert_eq!(
        refused(reused_in_new_turn).code(),
        ErrorCode::ReusedIdentity
    );
    assert_eq!(refused(reused_turn_id).code(), ErrorCode::ReusedIdentity);
    assert_eq!(refused(output_on_input).code(), ErrorCode::DisallowedKind);
    assert_eq!(
        refused(output_on_new_turn).code(),
        ErrorCode::DisallowedKind
    );
    assert_eq!((conversation, closed), before);
}

#[test]
fn given_an_open_ai_turn_when_an_output_is_pushed_then_disallowed_kind() {
    let mut conversation = empty();
    conversation.push_turn(open_turn(10).into()).unwrap();
    let output = AgentOutput::new(eid(30), aid(2), author("A"), vec![plain_text(31, "x")]).unwrap();

    let error = refused(conversation.push_incoming(output, tid(20)));

    assert_eq!(error.code(), ErrorCode::DisallowedKind);
    assert!(
        conversation
            .turn(&tid(10))
            .unwrap()
            .as_ai()
            .unwrap()
            .entries()
            .is_empty()
    );
}
