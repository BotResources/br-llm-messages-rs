mod common;

use br_llm_messages::{ClosureReason, Conversation, ErrorCode, IncomingEntry, InputTurn, TurnId};
use common::turns::{
    calling, close, delivery, finished, human, human_message, input_turn, open_turn, tid,
};
use common::{at, bid, cvid, eid, refused};

fn history() -> Conversation {
    let mut conversation = Conversation::new(cvid(1));
    let first = InputTurn::new(
        tid(10),
        vec![human_message(11).into(), human_message(12).into()],
    )
    .unwrap()
    .with_observed_at(at(2));
    conversation.push_turn(first.into()).unwrap();
    conversation
        .push_turn(open_turn(20).with_observed_at(at(3)).into())
        .unwrap();
    for entry in [
        human(21),
        calling(22, &[(23, "lookup")]),
        delivery(24, &[(25, 23, "lookup")]),
        finished(26),
        close(27, ClosureReason::completed()),
    ] {
        conversation.push_entry(&tid(20), entry).unwrap();
    }
    conversation.push_turn(input_turn(30, 31).into()).unwrap();
    conversation
}

fn turn_ids(conversation: &Conversation) -> Vec<TurnId> {
    conversation.turns().iter().map(|turn| turn.id()).collect()
}

fn entry_ids(conversation: &Conversation, turn: u64) -> Vec<br_llm_messages::EntryId> {
    let turn = conversation.turn(&tid(turn)).unwrap();
    match turn {
        br_llm_messages::Turn::InputTurn(input) => {
            input.entries().iter().map(IncomingEntry::id).collect()
        }
        br_llm_messages::Turn::AITurn(ai) => ai.entries().iter().map(|entry| entry.id()).collect(),
        _ => unreachable!(),
    }
}

#[test]
fn given_a_turn_when_cut_before_it_then_it_and_all_that_follows_are_lost_and_their_identities_are_free()
 {
    let mut conversation = history();

    conversation.cut_before(tid(20)).unwrap();

    assert_eq!(turn_ids(&conversation), [tid(10)]);
    assert!(conversation.entry(&eid(21)).is_err());
    assert!(conversation.block(&bid(23)).is_err());
    conversation.push_turn(open_turn(20).into()).unwrap();
    conversation.push_entry(&tid(20), human(21)).unwrap();
    conversation.push_turn(input_turn(30, 31).into()).unwrap();
    assert_eq!(conversation.check(), Ok(()));
}

#[test]
fn given_an_entry_inside_an_input_turn_when_cut_before_it_then_the_turn_keeps_the_entries_before_it()
 {
    let mut conversation = history();

    conversation.cut_before(eid(12)).unwrap();

    assert_eq!(turn_ids(&conversation), [tid(10)]);
    assert_eq!(entry_ids(&conversation, 10), [eid(11)]);
    assert_eq!(
        conversation.turn(&tid(10)).unwrap().observed_at(),
        Some(&at(2))
    );
    conversation
        .push_incoming(human_message(12), tid(99))
        .unwrap();
}

#[test]
fn given_the_first_entry_of_an_input_turn_when_cut_before_it_then_the_empty_input_turn_is_dropped()
{
    let mut conversation = history();

    conversation.cut_before(eid(31)).unwrap();

    assert_eq!(turn_ids(&conversation), [tid(10), tid(20)]);
    conversation.cut_before(eid(11)).unwrap();
    assert!(conversation.turns().is_empty());
    assert!(conversation.turn(&tid(10)).is_err());
}

#[test]
fn given_the_first_entry_of_an_ai_turn_when_cut_before_it_then_the_turn_stays_open_and_empty() {
    let mut conversation = history();

    conversation.cut_before(eid(21)).unwrap();

    assert_eq!(turn_ids(&conversation), [tid(10), tid(20)]);
    let turn = conversation.turn(&tid(20)).unwrap().as_ai().unwrap();
    assert!(turn.entries().is_empty());
    assert!(!turn.is_closed());
    assert_eq!(turn.observed_at(), Some(&at(3)));
    conversation.push_entry(&tid(20), human(21)).unwrap();
}

#[test]
fn given_a_closure_when_cut_before_it_then_the_turn_is_open_again() {
    let mut conversation = history();
    assert!(
        conversation
            .turn(&tid(20))
            .unwrap()
            .as_ai()
            .unwrap()
            .is_closed()
    );

    conversation.cut_before(eid(27)).unwrap();

    let turn = conversation.turn(&tid(20)).unwrap().as_ai().unwrap();
    assert!(!turn.is_closed());
    assert_eq!(turn.entries().len(), 4);
    conversation
        .push_entry(&tid(20), close(27, ClosureReason::completed()))
        .unwrap();
}

#[test]
fn given_a_result_delivery_when_cut_before_it_then_its_call_is_outstanding_again() {
    let mut conversation = history();

    conversation.cut_before(eid(24)).unwrap();

    let state = conversation
        .turn(&tid(20))
        .unwrap()
        .as_ai()
        .unwrap()
        .state();
    assert_eq!(state.outstanding_runtime_calls(), [bid(23)]);
    conversation
        .push_entry(&tid(20), delivery(24, &[(25, 23, "lookup")]))
        .unwrap();
}

#[test]
fn given_an_edit_when_it_is_a_cut_followed_by_an_addition_then_the_replaced_identity_is_reusable() {
    let mut conversation = history();

    conversation.cut_before(eid(26)).unwrap();
    conversation.push_entry(&tid(20), finished(26)).unwrap();

    assert_eq!(
        entry_ids(&conversation, 20),
        [eid(21), eid(22), eid(24), eid(26)]
    );
}

#[test]
fn given_a_fork_when_cut_then_the_original_is_unchanged_and_the_copy_has_the_given_or_the_original_identity()
 {
    let conversation = history();
    let before = conversation.clone();

    let same_id = conversation.fork_before(tid(20), None).unwrap();
    let new_id = conversation.fork_before(eid(12), Some(cvid(2))).unwrap();

    assert_eq!(conversation, before);
    assert_eq!(same_id.id(), cvid(1));
    assert_eq!(turn_ids(&same_id), [tid(10)]);
    assert_eq!(new_id.id(), cvid(2));
    assert_eq!(entry_ids(&new_id, 10), [eid(11)]);
    assert_eq!(same_id.check(), Ok(()));
    assert_eq!(new_id.check(), Ok(()));
}

#[test]
fn given_a_fork_identity_that_clashes_with_what_remains_when_forked_then_reused_identity() {
    let conversation = history();
    let kept_turn_as_id = br_llm_messages::ConversationId::from_uuid(tid(10).as_uuid()).unwrap();
    let cut_off_as_id = br_llm_messages::ConversationId::from_uuid(tid(30).as_uuid()).unwrap();

    let clash = conversation.fork_before(tid(20), Some(kept_turn_as_id));
    let free_again = conversation.fork_before(tid(20), Some(cut_off_as_id));

    assert_eq!(refused(clash).code(), ErrorCode::ReusedIdentity);
    assert_eq!(free_again.unwrap().id(), cut_off_as_id);
}

#[test]
fn given_a_position_that_is_not_a_turn_or_an_entry_when_cut_then_unknown_reference_and_nothing_changes()
 {
    let mut conversation = history();
    let before = conversation.clone();

    let block = conversation.cut_before(eid(23));
    let conversation_id = conversation.cut_before(tid(1));
    let unknown_turn = conversation.cut_before(tid(404));
    let unknown_entry = conversation.cut_before(eid(404));
    let forked = conversation.fork_before(eid(404), None);

    for error in [block, conversation_id, unknown_turn, unknown_entry] {
        assert_eq!(refused(error).code(), ErrorCode::UnknownReference);
    }
    assert_eq!(refused(forked).code(), ErrorCode::UnknownReference);
    assert_eq!(conversation, before);
}
