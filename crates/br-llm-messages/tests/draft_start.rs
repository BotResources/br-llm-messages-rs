mod common;
mod stream_support;

use br_llm_messages::{ContentBlock, DeltaOp, DraftPhase, ErrorCode, ResponseDelta, ResponseDraft};
use common::{bid, eid};
use stream_support::{
    draft, feed, finish, fresh, metadata, ok, refuse, run, start_text, start_thinking, text, tid,
};

#[test]
fn given_a_new_draft_when_created_then_it_carries_its_identities_and_has_not_started() {
    let draft = ResponseDraft::new(tid(1), eid(2));

    assert_eq!(draft.turn(), tid(1));
    assert_eq!(draft.entry(), eid(2));
    assert!(matches!(draft.phase(), DraftPhase::Unstarted { .. }));
    assert!(draft.response().is_none());
    assert!(draft.view().slots().is_empty());
}

#[test]
fn given_a_new_draft_when_a_delta_other_than_the_start_arrives_then_stream_phase_and_unchanged() {
    for op in [start_text(10), text(10, "x"), finish()] {
        let mut draft = fresh();

        let error = refuse(&mut draft, op);

        assert_eq!(error.code(), ErrorCode::StreamPhaseError);
        assert!(matches!(draft.phase(), DraftPhase::Unstarted { .. }));
    }
}

#[test]
fn given_a_new_draft_when_the_start_names_other_identities_then_stream_target_and_unchanged() {
    for foreign in [
        ResponseDelta::new(tid(9), eid(2), DeltaOp::response_started(metadata())),
        ResponseDelta::new(tid(1), eid(9), DeltaOp::response_started(metadata())),
    ] {
        let mut draft = fresh();

        let error = draft.apply(&foreign).unwrap_err();

        assert_eq!(error.code(), ErrorCode::StreamTargetError);
        assert!(matches!(draft.phase(), DraftPhase::Unstarted { .. }));
    }
}

#[test]
fn given_a_new_draft_when_the_start_arrives_then_metadata_come_from_the_start() {
    let mut draft = fresh();

    ok(&mut draft, DeltaOp::response_started(metadata()));

    assert_eq!(draft.metadata(), &metadata());
    assert!(matches!(draft.phase(), DraftPhase::Producing { .. }));
}

#[test]
fn given_a_started_draft_when_a_second_start_arrives_then_stream_phase_and_unchanged() {
    let mut draft = draft();

    let error = refuse(&mut draft, DeltaOp::response_started(metadata()));

    assert_eq!(error.code(), ErrorCode::StreamPhaseError);
    assert_eq!(error.object_id(), Some(eid(2).as_uuid()));
}

#[test]
fn given_a_foreign_turn_or_entry_when_applied_then_stream_target_and_unchanged() {
    let mut draft = draft();
    let before = draft.clone();
    let foreign_turn = ResponseDelta::new(tid(9), eid(2), start_text(10));
    let foreign_entry = ResponseDelta::new(tid(1), eid(9), start_text(10));

    for foreign in [foreign_turn, foreign_entry] {
        let error = draft.apply(&foreign).unwrap_err();
        assert_eq!(error.code(), ErrorCode::StreamTargetError);
        assert_eq!(error.object_id(), Some(bid(10).as_uuid()));
        assert_eq!(draft, before);
    }
}

#[test]
fn given_a_declared_block_when_its_id_is_declared_again_then_reused_identity() {
    let mut draft = run(vec![start_text(10)]);

    let error = refuse(&mut draft, start_thinking(10));

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
            br_llm_messages::BlockStart::text(),
        ),
    );

    assert_eq!(error.code(), ErrorCode::ReusedIdentity);
}

#[test]
fn given_blocks_whose_fragments_arrive_in_another_order_when_completed_then_declaration_order_is_the_content_order()
 {
    let mut draft = run(vec![
        start_text(30),
        start_text(10),
        start_thinking(20),
        text(10, "second"),
        DeltaOp::thinking_appended(bid(20), "plan"),
        text(30, "first"),
        stream_support::finalize(10),
        stream_support::finalize(20),
        stream_support::finalize(30),
    ]);

    let view = draft.view();
    let order: Vec<_> = view.slots().iter().map(|s| s.block()).collect();
    assert_eq!(order, vec![bid(30), bid(10), bid(20)]);

    feed(&mut draft, vec![finish()]);
    let ids: Vec<_> = draft
        .response()
        .unwrap()
        .blocks()
        .iter()
        .map(ContentBlock::id)
        .collect();
    assert_eq!(ids, vec![bid(30), bid(10), bid(20)]);
    assert!(matches!(draft.phase(), DraftPhase::Completed { .. }));
}

#[test]
fn given_a_draft_with_no_start_blocks_when_a_block_op_targets_an_unknown_block_then_stream_target()
{
    let mut draft = draft();

    let error = refuse(&mut draft, text(10, "x"));

    assert_eq!(error.code(), ErrorCode::StreamTargetError);
    assert_eq!(error.object_id(), Some(bid(10).as_uuid()));
}
