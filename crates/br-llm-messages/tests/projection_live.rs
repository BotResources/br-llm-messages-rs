mod common;
mod projection_support;

use br_llm_messages::{BlockStart, DeltaOp, ErrorCode, IncomingEntry, Perspective, Turn};
use common::turns::{human, tid};
use common::{aid, bid, cvid, eid, refused};
use projection_support::*;

#[test]
fn given_complete_producing_and_interrupted_responses_when_projected_then_output_partial_and_nothing()
 {
    let drafts = [producing(30, 32, 33, "so far"), interrupted(50, 52, 53)];

    let thread = Perspective::of(aid(1))
        .project_live(cvid(900), &history(), &drafts)
        .unwrap();

    assert_eq!(thread.len(), 2);
    assert_eq!(kinds(&thread.turns()[0]), ["output"]);
    assert_eq!(thread.turns()[1].id(), tid(30));
    assert_eq!(kinds(&thread.turns()[1]), ["partial"]);
}

#[test]
fn given_a_producing_response_after_a_completed_one_when_projected_then_the_partial_is_last() {
    let turns = vec![ai(10, 2, vec![said(11, 12, "complete")])];
    let draft = producing(10, 20, 21, "so far");

    let thread = Perspective::of(aid(1))
        .project_live(cvid(900), &turns, &[draft])
        .unwrap();

    assert_eq!(kinds(&thread.turns()[0]), ["output", "partial"]);
    let Turn::InputTurn(input) = &thread.turns()[0] else {
        panic!("input turn expected");
    };
    let IncomingEntry::PartialAgentOutput(partial) = &input.entries()[1] else {
        panic!("partial expected");
    };
    assert_eq!(partial.id(), eid(20));
    assert_eq!(partial.agent(), aid(2));
    assert_eq!(partial.content().len(), 1);
    assert_eq!(partial.content()[0].id(), bid(22));
    assert!(!format!("{thread:?}").contains("private plan"));
}

#[test]
fn given_a_producing_response_when_text_arrives_afterwards_then_the_built_thread_is_unchanged() {
    let turns = vec![ai(30, 2, vec![human(31)])];
    let mut draft = producing(30, 32, 33, "so far");
    let thread = Perspective::of(aid(1))
        .project_live(cvid(900), &turns, std::slice::from_ref(&draft))
        .unwrap();
    let before = thread.clone();

    feed(
        &mut draft,
        30,
        32,
        vec![DeltaOp::text_appended(bid(34), " and more")],
    );

    assert_eq!(thread, before);
    let again = Perspective::of(aid(1))
        .project_live(cvid(900), &turns, &[draft])
        .unwrap();
    assert_ne!(again, thread);
}

#[test]
fn given_a_producing_response_without_text_yet_when_projected_then_no_entry_is_made() {
    let mut draft = started(30, 32);
    feed(
        &mut draft,
        30,
        32,
        vec![DeltaOp::block_started(bid(33), BlockStart::text())],
    );

    let thread = Perspective::of(aid(1))
        .project_live(cvid(900), &[ai(30, 2, vec![human(31)])], &[draft])
        .unwrap();

    assert!(thread.is_empty());
}

#[test]
fn given_a_draft_of_an_unknown_turn_or_of_the_target_agent_when_projected_then_it_is_refused() {
    let turns = history();

    let unknown = refused(Perspective::of(aid(1)).project_live(
        cvid(900),
        &turns,
        &[producing(99, 90, 91, "x")],
    ));
    let own = refused(Perspective::of(aid(2)).project_live(
        cvid(900),
        &turns,
        &[producing(30, 32, 33, "x")],
    ));

    assert_eq!(unknown.code(), ErrorCode::UnknownReference);
    assert_eq!(own.code(), ErrorCode::DisallowedKind);
}

#[test]
fn given_two_producing_responses_in_one_turn_when_projected_then_invalid_record_structure() {
    let drafts = [producing(30, 32, 33, "a"), producing(30, 42, 43, "b")];

    let error = refused(Perspective::of(aid(1)).project_live(cvid(900), &history(), &drafts));

    assert_eq!(error.code(), ErrorCode::InvalidRecordStructure);
}
