mod common;
mod projected_thread_support;

use br_llm_messages::{
    AgentMail, ErrorCode, HumanMessage, IncomingEntry, InputTurn, RequestAccepted, TaskCompleted,
};
use common::turns::tid;
use common::{aid, author, bid, cvid, eid, hid, kid, plain_text, refused, rid};

fn message(entry: u64, block: u64) -> IncomingEntry {
    HumanMessage::new(eid(entry), hid(1), vec![plain_text(block, "hi")])
        .unwrap()
        .into()
}

fn base() -> InputTurn {
    InputTurn::new(tid(1), vec![message(2, 3)]).unwrap()
}

#[test]
fn given_an_input_turn_when_entries_of_every_allowed_kind_are_added_then_they_follow_in_order() {
    let mut turn = base();
    let mail: IncomingEntry = AgentMail::new(
        eid(4),
        aid(7),
        author("Codex"),
        cvid(70),
        vec![plain_text(5, "mail")],
    )
    .unwrap()
    .into();

    turn.push_entry(mail).unwrap();
    turn.push_entry(RequestAccepted::new(eid(6), rid(1)).into())
        .unwrap();
    turn.push_entry(TaskCompleted::new(eid(7), kid(1)).into())
        .unwrap();
    turn.push_entry(message(8, 9)).unwrap();

    let ids: Vec<_> = turn.entries().iter().map(IncomingEntry::id).collect();
    assert_eq!(ids, [eid(2), eid(4), eid(6), eid(7), eid(8)]);
    assert_eq!(
        InputTurn::from_json(&turn.to_json().unwrap()).unwrap(),
        turn
    );
}

#[test]
fn given_an_extended_input_turn_when_compared_with_one_built_at_once_then_they_are_equal() {
    let mut grown = base();
    grown.push_entry(message(4, 5)).unwrap();

    let built = InputTurn::new(tid(1), vec![message(2, 3), message(4, 5)]).unwrap();

    assert_eq!(grown, built);
}

#[test]
fn given_a_reused_identity_when_an_entry_is_added_then_reused_identity_located_and_nothing_changes()
{
    let mut turn = base();
    turn.push_entry(message(4, 5)).unwrap();
    let before = turn.clone();

    let cases = [
        (message(2, 11), eid(2).as_uuid()),
        (message(12, 3), bid(3).as_uuid()),
        (message(1, 13), tid(1).as_uuid()),
        (message(14, 5), bid(5).as_uuid()),
    ];

    for (entry, reused) in cases {
        let error = refused(turn.push_entry(entry));
        assert_eq!(error.code(), ErrorCode::ReusedIdentity);
        assert_eq!(error.object_id(), Some(reused));
        assert_eq!(error.location(), Some("entries[2]"));
        assert_eq!(turn, before);
    }
}

#[test]
fn given_a_projected_output_when_added_to_an_input_turn_then_disallowed_kind_and_nothing_changes() {
    let thread = projected_thread_support::thread();
    let outputs: Vec<IncomingEntry> = thread
        .turns()
        .iter()
        .filter_map(|turn| turn.as_input())
        .flat_map(|turn| turn.entries().iter().cloned())
        .filter(|entry| entry.kind().is_none())
        .collect();
    assert!(!outputs.is_empty());
    let mut turn = base();
    let before = turn.clone();

    for output in outputs {
        let error = refused(turn.push_entry(output.clone()));
        assert_eq!(error.code(), ErrorCode::DisallowedKind);
        assert_eq!(error.location(), Some("entries[1]"));
        assert_eq!(error.object_id(), Some(output.id().as_uuid()));
        assert_eq!(turn, before);
    }
}

#[test]
fn given_a_dry_run_when_checking_an_addition_then_it_matches_the_real_addition_and_mutates_nothing()
{
    let mut turn = base();
    let before = turn.clone();

    assert!(turn.check_push(&message(4, 5)).is_ok());
    let refusal = refused(turn.check_push(&message(2, 9)));
    assert_eq!(turn, before);

    assert_eq!(refused(turn.push_entry(message(2, 9))), refusal);
    turn.push_entry(message(4, 5)).unwrap();
    assert_eq!(turn.entries().len(), 2);
}
