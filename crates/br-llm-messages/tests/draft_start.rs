mod common;
mod stream_support;

use br_llm_messages::{ContentBlock, DeltaOp, DraftPhase, ErrorCode, ResponseDelta, ResponseDraft};
use common::{bid, eid};
use stream_support::{
    delta, draft, feed, finish, metadata, refuse, run, start_text, start_thinking, text, tid,
};

#[test]
fn given_a_first_delta_that_is_not_a_start_when_beginning_then_stream_phase_with_the_entry_id() {
    let first = delta(start_text(10, 0));

    let error = ResponseDraft::begin(&first).unwrap_err();

    assert_eq!(error.code(), ErrorCode::StreamPhase);
    assert_eq!(error.object_id(), Some(eid(2).as_uuid()));
}

#[test]
fn given_a_started_draft_when_beginning_then_identities_and_metadata_come_from_the_start() {
    let first = delta(DeltaOp::response_started(metadata()));

    let draft = ResponseDraft::begin(&first).unwrap();

    assert_eq!(draft.turn(), tid(1));
    assert_eq!(draft.entry(), eid(2));
    assert_eq!(draft.metadata(), &metadata());
    assert!(matches!(draft.phase(), DraftPhase::Producing { .. }));
    assert!(draft.response().is_none());
}

#[test]
fn given_a_started_draft_when_a_second_start_arrives_then_stream_phase_and_unchanged() {
    let mut draft = draft();

    let error = refuse(&mut draft, DeltaOp::response_started(metadata()));

    assert_eq!(error.code(), ErrorCode::StreamPhase);
    assert_eq!(error.object_id(), Some(eid(2).as_uuid()));
}

#[test]
fn given_a_foreign_turn_or_entry_when_applied_then_stream_target_and_unchanged() {
    let mut draft = draft();
    let before = draft.clone();
    let foreign_turn = ResponseDelta::new(tid(9), eid(2), start_text(10, 0));
    let foreign_entry = ResponseDelta::new(tid(1), eid(9), start_text(10, 0));

    for foreign in [foreign_turn, foreign_entry] {
        let error = draft.apply(&foreign).unwrap_err();
        assert_eq!(error.code(), ErrorCode::StreamTarget);
        assert_eq!(error.object_id(), Some(bid(10).as_uuid()));
        assert_eq!(draft, before);
    }
}

#[test]
fn given_a_declared_block_when_its_id_is_declared_again_then_reused_identity() {
    let mut draft = run(vec![start_text(10, 0)]);

    let error = refuse(&mut draft, start_thinking(10, 1));

    assert_eq!(error.code(), ErrorCode::ReusedIdentity);
    assert_eq!(error.object_id(), Some(bid(10).as_uuid()));
}

#[test]
fn given_the_response_entry_id_when_used_for_a_block_then_reused_identity() {
    let mut draft = draft();

    let error = refuse(
        &mut draft,
        DeltaOp::block_started(
            br_llm_messages::BlockId::from_uuid(eid(2).as_uuid()).unwrap(),
            0,
            br_llm_messages::BlockStart::text(),
        ),
    );

    assert_eq!(error.code(), ErrorCode::ReusedIdentity);
}

#[test]
fn given_an_occupied_position_when_another_block_starts_there_then_invalid_position() {
    let mut draft = run(vec![start_text(10, 4)]);

    let error = refuse(&mut draft, start_thinking(11, 4));

    assert_eq!(error.code(), ErrorCode::InvalidPosition);
    assert_eq!(error.object_id(), Some(bid(11).as_uuid()));
}

#[test]
fn given_blocks_declared_out_of_order_when_completed_then_caller_ids_and_positions_are_kept() {
    let mut draft = run(vec![
        start_text(30, 2),
        start_text(10, 0),
        start_thinking(20, 1),
        text(30, "third"),
        text(10, "first"),
        stream_support::finalize(30),
        stream_support::finalize(10),
        stream_support::finalize(20),
    ]);

    let view = draft.view();
    let order: Vec<_> = view
        .slots()
        .iter()
        .map(|s| (s.position(), s.block()))
        .collect();
    assert_eq!(order, vec![(0, bid(10)), (1, bid(20)), (2, bid(30))]);

    feed(&mut draft, vec![finish()]);
    let ids: Vec<_> = draft
        .response()
        .unwrap()
        .blocks()
        .iter()
        .map(ContentBlock::id)
        .collect();
    assert_eq!(ids, vec![bid(10), bid(20), bid(30)]);
    assert!(matches!(draft.phase(), DraftPhase::Completed { .. }));
}

#[test]
fn given_a_draft_with_no_start_blocks_when_a_block_op_targets_an_unknown_block_then_stream_target()
{
    let mut draft = draft();

    let error = refuse(&mut draft, text(10, "x"));

    assert_eq!(error.code(), ErrorCode::StreamTarget);
    assert_eq!(error.object_id(), Some(bid(10).as_uuid()));
}
