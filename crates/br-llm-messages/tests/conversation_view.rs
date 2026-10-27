mod common;

use br_llm_messages::{ErrorCode, IncomingEntry, Perspective, Turn};
use common::turns::{finished, human, human_message, input_turn, open_turn, tid};
use common::{aid, cvid, eid};

fn empty() -> br_llm_messages::Conversation {
    br_llm_messages::Conversation::new(cvid(1))
}

#[test]
fn given_a_conversation_when_projected_for_an_agent_then_the_result_is_the_perspective_projection_without_selection()
 {
    let mut conversation = empty();
    conversation.push_turn(input_turn(10, 11).into()).unwrap();
    conversation.push_turn(open_turn(20).into()).unwrap();
    conversation.push_entry(&tid(20), human(21)).unwrap();
    conversation.push_entry(&tid(20), finished(22)).unwrap();

    for agent in [aid(1), aid(2)] {
        let projected = conversation.project_for(agent);
        assert_eq!(projected, Perspective::of(agent).project(&conversation));
        assert_eq!(projected.conversation(), conversation.id());
        assert_eq!(projected.agent(), agent);
    }
    assert_eq!(conversation.project_for(aid(2)).turns().len(), 2);
}

#[test]
fn given_a_conversation_when_its_entries_are_listed_then_they_follow_turn_order_then_entry_order_with_their_turn()
 {
    let mut conversation = empty();
    conversation.push_turn(input_turn(10, 11).into()).unwrap();
    conversation
        .push_incoming(human_message(12), tid(99))
        .unwrap();
    conversation.push_turn(open_turn(20).into()).unwrap();
    conversation.push_entry(&tid(20), human(21)).unwrap();
    conversation.push_entry(&tid(20), finished(22)).unwrap();
    conversation.push_turn(input_turn(30, 31).into()).unwrap();

    let flat: Vec<_> = conversation
        .entries()
        .map(|placed| (placed.turn(), placed.entry().id()))
        .collect();

    assert_eq!(
        flat,
        [
            (tid(10), eid(11)),
            (tid(10), eid(12)),
            (tid(20), eid(21)),
            (tid(20), eid(22)),
            (tid(30), eid(31)),
        ]
    );
    assert_eq!(empty().entries().count(), 0);
}

#[test]
fn given_any_addition_when_asked_as_a_dry_run_then_the_answer_is_the_refusal_of_the_push_and_nothing_changes()
 {
    let mut conversation = empty();
    conversation.push_turn(input_turn(10, 11).into()).unwrap();
    conversation.push_turn(open_turn(20).into()).unwrap();
    let before = conversation.clone();

    let fresh_turn: Turn = input_turn(30, 31).into();
    let clashing_turn: Turn = input_turn(30, 11).into();
    let fresh_incoming: IncomingEntry = human_message(40).into();
    let clashing_incoming: IncomingEntry = human_message(11).into();

    assert_eq!(conversation.would_refuse_turn(&fresh_turn), None);
    assert_eq!(
        conversation
            .would_refuse_turn(&clashing_turn)
            .unwrap()
            .code(),
        ErrorCode::ReusedIdentity
    );
    assert_eq!(conversation.would_refuse_entry(&tid(20), &human(41)), None);
    assert_eq!(
        conversation
            .would_refuse_entry(&tid(20), &human(11))
            .unwrap()
            .code(),
        ErrorCode::ReusedIdentity
    );
    assert_eq!(
        conversation
            .would_refuse_entry(&tid(10), &human(42))
            .unwrap()
            .code(),
        ErrorCode::UnknownReference
    );
    assert_eq!(
        conversation.would_refuse_incoming(&fresh_incoming, tid(50)),
        None
    );
    assert_eq!(
        conversation
            .would_refuse_incoming(&clashing_incoming, tid(50))
            .unwrap()
            .code(),
        ErrorCode::ReusedIdentity
    );
    assert_eq!(conversation, before);
    conversation.push_turn(fresh_turn).unwrap();
    conversation.push_entry(&tid(20), human(41)).unwrap();
    conversation.push_incoming(fresh_incoming, tid(50)).unwrap();
}

#[test]
fn given_a_conversation_when_checked_as_a_whole_then_it_passes() {
    let mut conversation = empty();

    assert_eq!(conversation.check(), Ok(()));
    conversation.push_turn(input_turn(10, 11).into()).unwrap();
    conversation
        .push_incoming(human_message(12), tid(20))
        .unwrap();
    conversation.push_turn(open_turn(30).into()).unwrap();
    conversation.push_entry(&tid(30), finished(31)).unwrap();

    assert_eq!(conversation.check(), Ok(()));
}
