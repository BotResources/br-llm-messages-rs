mod common;
mod projected_thread_build_support;
mod projected_thread_support;

use br_llm_messages::{AgentOutput, ErrorCode, InputTurn, ProjectedThread, Turn};
use common::turns::{close, delivery, human, human_message, input_turn, tid};
use common::{aid, at, author, cvid, eid, plain_text, refused};
use projected_thread_build_support::*;
use projected_thread_support::{ai, said};

#[test]
fn given_turns_when_built_then_the_value_equals_its_json_reload_and_a_projection() {
    let thread = base();

    let reloaded = ProjectedThread::from_json(&thread.to_json().unwrap()).unwrap();
    let rebuilt = ProjectedThread::from_turns(
        thread.conversation(),
        thread.agent(),
        thread.turns().to_vec(),
    )
    .unwrap();

    assert_eq!(reloaded, thread);
    assert_eq!(rebuilt, thread);
    assert_eq!(thread.conversation(), cvid(900));
    assert_eq!(thread.agent(), aid(1));
}

#[test]
fn given_no_turn_when_built_then_the_empty_thread_is_valid() {
    let thread = ProjectedThread::from_turns(cvid(900), aid(1), vec![]).unwrap();

    assert!(thread.turns().is_empty());
    assert_eq!(thread.entries().count(), 0);
}

#[test]
fn given_a_turn_that_a_thread_refuses_when_built_then_the_reload_refusal_and_location_are_given() {
    let another_agent = ai(30, 3, vec![human(31)]);
    let target_output: Turn = InputTurn::new_projected(tid(30), vec![output(31, 1, 32).into()])
        .unwrap()
        .into();
    let mismatch: Turn = InputTurn::new_projected(
        tid(30),
        vec![
            output(31, 2, 32).into(),
            AgentOutput::new(eid(33), aid(3), author("Codex"), vec![plain_text(34, "x")])
                .unwrap()
                .into(),
        ],
    )
    .unwrap()
    .into();

    let foreign_ai = refused(ProjectedThread::from_turns(
        cvid(900),
        aid(1),
        vec![input_turn(1, 2).into(), another_agent],
    ));
    let own_output = refused(ProjectedThread::from_turns(
        cvid(900),
        aid(1),
        vec![target_output],
    ));
    let two_owners = refused(ProjectedThread::from_turns(
        cvid(900),
        aid(1),
        vec![mismatch],
    ));

    assert_eq!(foreign_ai.code(), ErrorCode::DisallowedKind);
    assert_eq!(foreign_ai.location(), Some("turns[1]"));
    assert_eq!(own_output.code(), ErrorCode::DisallowedKind);
    assert_eq!(own_output.location(), Some("turns[0].entries[0]"));
    assert_eq!(two_owners.code(), ErrorCode::InvalidRecordStructure);
    assert_eq!(two_owners.location(), Some("turns[0].entries[1]"));
}

#[test]
fn given_an_identity_used_twice_in_the_whole_thread_when_built_then_reused_identity() {
    let in_another_turn = [foreign_turn(40, 21, 42), foreign_turn(40, 41, 5002)];
    let as_conversation_id = ProjectedThread::from_turns(
        cvid(900),
        aid(1),
        vec![
            InputTurn::new(tid(900), vec![human_message(2).into()])
                .unwrap()
                .into(),
        ],
    );

    for clash in in_another_turn {
        let mut turns = base_turns();
        turns.push(clash);
        let error = refused(ProjectedThread::from_turns(cvid(900), aid(1), turns));
        assert_eq!(error.code(), ErrorCode::ReusedIdentity);
        assert_eq!(error.location(), Some("turns[3].entries[0]"));
    }
    assert_eq!(
        refused(as_conversation_id).code(),
        ErrorCode::ReusedIdentity
    );
}

#[test]
fn given_a_thread_when_grown_by_turns_and_entries_then_it_equals_the_thread_built_at_once() {
    let mut thread = base();
    thread.push_turn(foreign_turn(40, 41, 42)).unwrap();
    thread.push_incoming(&tid(40), partial(43, 2, 44)).unwrap();
    thread.push_incoming(&tid(40), human_message(45)).unwrap();
    thread.push_incoming(&tid(1), output(46, 5, 47)).unwrap();
    thread.push_incoming(&tid(10), human_message(48)).unwrap();
    thread.push_entry(&tid(10), said(49, 50, "again")).unwrap();
    thread.push_entry(&tid(1), human(51)).unwrap();

    let mut turns = base_turns();
    turns[0] = InputTurn::new_projected(
        tid(1),
        vec![
            human_message(2).into(),
            output(46, 5, 47).into(),
            human_message(51).into(),
        ],
    )
    .unwrap()
    .into();
    turns[1] = ai(
        10,
        1,
        vec![
            human(14),
            said(15, 16, "mine"),
            human(48),
            said(49, 50, "again"),
        ],
    );
    turns.push(
        InputTurn::new_projected(
            tid(40),
            vec![
                output(41, 2, 42).into(),
                partial(43, 2, 44).into(),
                human_message(45).into(),
            ],
        )
        .unwrap()
        .into(),
    );
    let at_once = ProjectedThread::from_turns(cvid(900), aid(1), turns).unwrap();
    assert_eq!(thread, at_once);
    assert_eq!(
        ProjectedThread::from_json(&thread.to_json().unwrap()).unwrap(),
        thread
    );
}

#[test]
fn given_an_input_turn_with_an_observed_time_when_grown_then_the_time_is_kept() {
    let observed = InputTurn::new_projected(tid(60), vec![output(61, 2, 62).into()])
        .unwrap()
        .with_observed_at(at(3));
    let mut thread = base();
    thread.push_turn(observed.into()).unwrap();

    thread.push_incoming(&tid(60), partial(63, 2, 64)).unwrap();

    assert_eq!(thread.turn(&tid(60)).unwrap().observed_at(), Some(&at(3)));
}

#[test]
fn given_refused_additions_when_pushed_then_the_thread_is_unchanged_and_the_refusal_is_precise() {
    let before = base();
    let mut thread = before.clone();

    let refusals = [
        (
            thread.push_turn(foreign_turn(70, 21, 72)),
            ErrorCode::ReusedIdentity,
        ),
        (
            thread.push_turn(foreign_turn(70, 71, 22)),
            ErrorCode::ReusedIdentity,
        ),
        (
            thread.push_turn(foreign_turn(900, 71, 72)),
            ErrorCode::ReusedIdentity,
        ),
        (
            thread.push_turn(ai(80, 3, vec![])),
            ErrorCode::DisallowedKind,
        ),
        (
            thread.push_incoming(&tid(20), human_message(21)),
            ErrorCode::ReusedIdentity,
        ),
        (
            thread.push_incoming(&tid(1), output(73, 1, 74)),
            ErrorCode::DisallowedKind,
        ),
        (
            thread.push_incoming(&tid(20), output(73, 3, 74)),
            ErrorCode::InvalidRecordStructure,
        ),
        (
            thread.push_incoming(&tid(20), human_message(900)),
            ErrorCode::ReusedIdentity,
        ),
        (
            thread.push_incoming(&tid(404), human_message(75)),
            ErrorCode::UnknownReference,
        ),
        (
            thread.push_incoming(&tid(10), output(76, 2, 77)),
            ErrorCode::DisallowedKind,
        ),
        (
            thread.push_entry(&tid(10), delivery(78, &[(79, 80, "t")])),
            ErrorCode::EntryOutOfOrder,
        ),
        (
            thread.push_entry(&tid(10), human(15)),
            ErrorCode::ReusedIdentity,
        ),
        (
            thread.push_entry(&tid(10), human_with_block(82, 5002)),
            ErrorCode::ReusedIdentity,
        ),
        (
            thread.push_entry(&tid(20), said(83, 84, "x")),
            ErrorCode::DisallowedKind,
        ),
    ];

    for (index, (result, code)) in refusals.into_iter().enumerate() {
        assert_eq!(refused(result).code(), code, "refusal {index}");
    }
    assert_eq!(thread, before);
}

#[test]
fn given_a_closed_target_turn_when_an_entry_is_pushed_then_the_ai_turn_rules_apply() {
    use br_llm_messages::ClosureReason;
    let mut thread = base();
    thread
        .push_entry(&tid(10), close(90, ClosureReason::completed()))
        .unwrap();
    let before = thread.clone();

    let refused_entry = thread.push_entry(&tid(10), said(91, 92, "late"));

    assert_eq!(refused(refused_entry).code(), ErrorCode::ClosedTurn);
    assert_eq!(thread, before);
}
