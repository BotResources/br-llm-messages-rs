mod common;

use br_llm_messages::{
    AgentOutput, Citation, CitationSource, ClosureReason, Conversation, ErrorCode, HumanMessage,
    IncomingEntry, InputTurn, PlainBlock, TextBlock, Turn,
};
use common::turns::{close, finished, human_message, input_turn, open_turn, tid};
use common::{aid, at, author, bid, cid, cvid, eid, hid, plain_text, refused};

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

fn cited_message(entry: u64, block: u64, citation: u64) -> HumanMessage {
    let citation = Citation::new(cid(citation), CitationSource::new("doc").unwrap());
    let block = PlainBlock::try_from(
        TextBlock::new(bid(block), "cited")
            .unwrap()
            .with_citations(vec![citation])
            .unwrap(),
    )
    .unwrap();
    HumanMessage::new(eid(entry), hid(1), vec![block]).unwrap()
}

fn two_inputs_around_a_closed_ai_turn() -> Conversation {
    let mut conversation = empty();
    let first = InputTurn::new(tid(10), vec![cited_message(11, 12, 13).into()]).unwrap();
    conversation.push_turn(first.into()).unwrap();
    conversation.push_turn(open_turn(20).into()).unwrap();
    conversation
        .push_entry(&tid(20), close(21, ClosureReason::completed()))
        .unwrap();
    conversation.push_turn(input_turn(30, 31).into()).unwrap();
    conversation
}

#[test]
fn given_a_last_input_turn_when_an_incoming_entry_reuses_an_identity_of_an_earlier_turn_then_reused_identity_and_nothing_changes()
 {
    let mut conversation = two_inputs_around_a_closed_ai_turn();
    let before = conversation.clone();
    let cases = [
        (human_message_with_ids(11, 41), eid(11).as_uuid()),
        (cited_message(40, 12, 42), bid(12).as_uuid()),
        (cited_message(40, 41, 13), cid(13).as_uuid()),
        (cited_message(21, 41, 42), eid(21).as_uuid()),
        (cited_message(40, 41, 10), tid(10).as_uuid()),
    ];

    for (entry, reused) in cases {
        let dry = conversation.would_refuse_incoming(&entry.clone().into(), tid(50));
        let error = refused(conversation.push_incoming(entry, tid(50)));

        assert_eq!(error.code(), ErrorCode::ReusedIdentity);
        assert_eq!(error.object_id(), Some(reused));
        assert_eq!(dry.unwrap().code(), ErrorCode::ReusedIdentity);
        assert_eq!(conversation, before);
    }
}

fn human_message_with_ids(entry: u64, block: u64) -> HumanMessage {
    HumanMessage::new(eid(entry), hid(1), vec![plain_text(block, "x")]).unwrap()
}
