mod common;
mod projected_thread_build_support;
mod projected_thread_support;

use br_llm_messages::{ErrorCode, IncomingEntry, TurnEntry};
use common::turns::{human, tid};
use common::{bid, eid, refused};
use projected_thread_build_support::*;

#[test]
fn given_any_addition_when_asked_as_a_dry_run_then_the_answer_is_the_refusal_of_the_push_and_nothing_changes()
 {
    let thread = base();
    let before = thread.clone();
    let accepted_turn = foreign_turn(40, 41, 42);
    let refused_turn = foreign_turn(40, 21, 42);
    let accepted_entry: IncomingEntry = output(43, 2, 44).into();
    let refused_entry: IncomingEntry = output(43, 1, 44).into();
    let refused_entry_turn = tid(1);
    let accepted_recorded: TurnEntry = human(45);
    let refused_recorded: TurnEntry = human(15);

    assert_eq!(thread.would_refuse_turn(&accepted_turn), None);
    assert_eq!(
        thread.would_refuse_turn(&refused_turn).unwrap().code(),
        ErrorCode::ReusedIdentity
    );
    assert_eq!(
        thread.would_refuse_incoming_into(&tid(20), &accepted_entry),
        None
    );
    assert_eq!(
        thread
            .would_refuse_incoming_into(&refused_entry_turn, &refused_entry)
            .unwrap()
            .code(),
        ErrorCode::DisallowedKind
    );
    assert_eq!(
        thread.would_refuse_entry(&tid(10), &accepted_recorded),
        None
    );
    assert_eq!(
        thread
            .would_refuse_entry(&tid(10), &refused_recorded)
            .unwrap()
            .code(),
        ErrorCode::ReusedIdentity
    );
    assert_eq!(thread, before);
    let mut grown = thread.clone();
    grown.push_turn(accepted_turn).unwrap();
    grown.push_incoming_into(&tid(20), accepted_entry).unwrap();
    grown.push_entry(&tid(10), accepted_recorded).unwrap();
}

#[test]
fn given_a_thread_when_checked_as_a_whole_then_it_passes_for_built_grown_and_projected_threads() {
    let mut grown = base();
    grown
        .push_incoming_into(&tid(20), partial(43, 2, 44))
        .unwrap();

    assert_eq!(base().check(), Ok(()));
    assert_eq!(grown.check(), Ok(()));
    assert_eq!(projected_thread_support::thread().check(), Ok(()));
}

#[test]
fn given_identities_when_looked_up_in_a_thread_then_turns_entries_and_blocks_resolve_to_the_right_kind()
 {
    let thread = base();

    let turn = thread.turn(&tid(20)).unwrap();
    let projected_entry = thread.entry(&eid(21)).unwrap();
    let recorded_entry = thread.entry(&eid(15)).unwrap();
    let projected_block = thread.block(&bid(22)).unwrap();
    let own_block = thread.block(&bid(16)).unwrap();

    assert_eq!(turn.id(), tid(20));
    assert_eq!(projected_entry.id(), eid(21));
    assert_eq!(projected_entry.kind(), None);
    assert_eq!(
        recorded_entry.kind(),
        Some(br_llm_messages::TurnEntryKind::ModelResponse)
    );
    assert_eq!(projected_block.id(), bid(22));
    assert_eq!(own_block.id(), bid(16));
}

#[test]
fn given_an_identity_of_another_kind_or_unknown_when_looked_up_then_unknown_reference() {
    let thread = base();

    for error in [
        refused(thread.turn(&tid(404)).map(|_| ())),
        refused(thread.turn(&tid(900)).map(|_| ())),
        refused(thread.entry(&eid(20)).map(|_| ())),
        refused(thread.entry(&eid(22)).map(|_| ())),
        refused(thread.block(&bid(21)).map(|_| ())),
        refused(thread.block(&bid(404)).map(|_| ())),
    ] {
        assert_eq!(error.code(), ErrorCode::UnknownReference);
    }
}

#[test]
fn given_a_grown_thread_when_looked_up_then_added_identities_resolve() {
    let mut thread = base();
    thread
        .push_incoming_into(&tid(20), partial(43, 2, 44))
        .unwrap();
    thread.push_entry(&tid(10), human(45)).unwrap();

    assert!(thread.entry(&eid(43)).is_ok());
    assert!(thread.block(&bid(44)).is_ok());
    assert!(thread.entry(&eid(45)).is_ok());
}

#[test]
fn given_a_thread_when_its_entries_are_listed_then_they_follow_turn_order_then_entry_order_with_their_turn()
 {
    let thread = base();

    let flat: Vec<_> = thread
        .entries()
        .map(|placed| (placed.turn(), placed.entry().id()))
        .collect();

    assert_eq!(
        flat,
        [
            (tid(1), eid(2)),
            (tid(10), eid(14)),
            (tid(10), eid(15)),
            (tid(20), eid(21)),
        ]
    );
}
