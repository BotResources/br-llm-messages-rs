mod common;
mod stream_support;

use br_llm_messages::{
    AITurn, ContentBlock, DeltaOp, DeltaOutcome, DraftPhase, ErrorCode, FailureDetail,
    InterruptionReason, ResponseEnding, SlotLifecycle, TurnEntry,
};
use common::{aid, author, bid, eid, text_block};
use stream_support::{
    complete, finalize, finish, json, ok, refuse, run, start_call, start_json, start_text,
    start_thinking, text, tid,
};

#[test]
fn given_an_open_block_when_completing_then_refused_naming_the_block_and_the_draft_keeps_producing()
{
    let mut draft = run(vec![
        start_text(10, 0),
        text(10, "x"),
        finalize(10),
        start_text(11, 1),
    ]);

    let error = refuse(&mut draft, finish());

    assert_eq!(error.code(), ErrorCode::InconsistentFinalization);
    assert_eq!(error.location(), Some("content[1]"));
    assert!(matches!(draft.phase(), DraftPhase::Producing { .. }));
    ok(&mut draft, DeltaOp::block_discarded(bid(11)));
    assert!(matches!(
        ok(&mut draft, finish()),
        DeltaOutcome::Completed { .. }
    ));
}

#[test]
fn given_a_gap_in_positions_when_completing_then_invalid_position() {
    let mut draft = run(vec![
        start_text(10, 0),
        text(10, "x"),
        finalize(10),
        start_text(11, 2),
        text(11, "y"),
        finalize(11),
    ]);

    let error = refuse(&mut draft, finish());

    assert_eq!(error.code(), ErrorCode::InvalidPosition);
    assert_eq!(error.object_id(), Some(eid(2).as_uuid()));
}

#[test]
fn given_positions_not_starting_at_zero_when_completing_then_invalid_position() {
    let mut draft = run(vec![start_text(10, 1), text(10, "x"), finalize(10)]);

    assert_eq!(
        refuse(&mut draft, finish()).code(),
        ErrorCode::InvalidPosition
    );
}

#[test]
fn given_an_ending_the_blocks_cannot_support_when_completing_then_direct_construction_rules_apply_and_the_draft_stays_producing()
 {
    let mut draft = run(vec![start_text(10, 0), text(10, "x"), finalize(10)]);

    let error = refuse(
        &mut draft,
        complete(ResponseEnding::tool_results_requested()),
    );

    assert_eq!(error.code(), ErrorCode::MissingContent);
    assert!(matches!(draft.phase(), DraftPhase::Producing { .. }));

    ok(&mut draft, start_call(11, 1, Some("t"), None));
    ok(&mut draft, json(11, "{}"));
    ok(&mut draft, finalize(11));
    ok(
        &mut draft,
        complete(ResponseEnding::tool_results_requested()),
    );
    assert!(draft.response().is_some());
}

#[test]
fn given_an_empty_response_when_completed_then_allowed_for_any_ending_but_a_tool_request() {
    let mut draft = run(vec![]);
    let error = refuse(
        &mut draft,
        complete(ResponseEnding::tool_results_requested()),
    );
    assert_eq!(error.code(), ErrorCode::MissingContent);

    ok(
        &mut draft,
        complete(ResponseEnding::provider_continuation()),
    );

    assert!(draft.response().unwrap().blocks().is_empty());
    assert_eq!(
        draft.response().unwrap().ending(),
        &ResponseEnding::provider_continuation()
    );
}

#[test]
fn given_a_completed_draft_when_any_delta_arrives_then_stream_phase_and_the_response_is_sealed() {
    let mut draft = run(vec![
        start_text(10, 0),
        text(10, "x"),
        finalize(10),
        finish(),
    ]);
    let sealed = draft.response().unwrap().clone();

    for op in [
        text(10, "y"),
        finish(),
        start_text(11, 1),
        DeltaOp::response_interrupted(InterruptionReason::cancelled()),
    ] {
        assert_eq!(refuse(&mut draft, op).code(), ErrorCode::StreamPhase);
    }

    assert_eq!(draft.response(), Some(&sealed));
    assert!(matches!(draft.phase(), DraftPhase::Completed { .. }));
}

#[test]
fn given_a_completed_response_when_added_to_an_open_ai_turn_then_the_turn_is_still_open() {
    let draft = run(vec![
        start_text(10, 0),
        text(10, "x"),
        finalize(10),
        finish(),
    ]);
    let mut turn = AITurn::open(tid(1), aid(1), author("agent"));

    turn.push_entry(TurnEntry::ModelResponse(draft.response().unwrap().clone()))
        .unwrap();

    assert!(turn.state().closure().is_none());
    assert_eq!(draft.into_response().unwrap().id(), eid(2));
}

#[test]
fn given_each_interruption_reason_when_applied_then_the_draft_is_interrupted_and_no_response_exists()
 {
    let failed = InterruptionReason::failed(FailureDetail::new("overloaded").unwrap());
    for reason in [
        InterruptionReason::cancelled(),
        InterruptionReason::truncated(),
        InterruptionReason::interrupted(),
        failed,
    ] {
        let mut draft = run(vec![start_text(10, 0)]);

        let outcome = ok(&mut draft, DeltaOp::response_interrupted(reason.clone()));

        assert!(matches!(outcome, DeltaOutcome::Interrupted { .. }));
        assert_eq!(draft.phase(), &DraftPhase::Interrupted(reason));
        assert!(draft.response().is_none());
    }
}

#[test]
fn given_an_interrupted_draft_when_inspected_then_complete_blocks_stay_complete_and_partials_are_untouched()
 {
    let mut draft = run(vec![
        start_text(10, 0),
        text(10, "done"),
        finalize(10),
        start_thinking(11, 1),
        DeltaOp::thinking_appended(bid(11), "half a thou"),
        start_json(12, 2),
        json(12, "{\"unfinished\":"),
        start_call(13, 3, Some("t"), None),
    ]);

    ok(
        &mut draft,
        DeltaOp::response_interrupted(InterruptionReason::truncated()),
    );

    let view = draft.view();
    let slots = view.slots();
    assert_eq!(slots[0].lifecycle(), SlotLifecycle::Final);
    assert_eq!(
        slots[0].final_block(),
        Some(&ContentBlock::from(text_block(10, "done")))
    );
    assert_eq!(slots[1].lifecycle(), SlotLifecycle::Open);
    assert_eq!(slots[1].text(), Some("half a thou"));
    assert_eq!(slots[2].raw_json(), Some("{\"unfinished\":"));
    assert_eq!(slots[3].raw_json(), Some(""));
    assert!(slots[3].final_block().is_none());
}

#[test]
fn given_an_interrupted_draft_when_more_deltas_arrive_then_stream_phase() {
    let mut draft = run(vec![start_text(10, 0)]);
    ok(
        &mut draft,
        DeltaOp::response_interrupted(InterruptionReason::cancelled()),
    );

    for op in [text(10, "x"), finalize(10), finish(), start_text(11, 1)] {
        assert_eq!(refuse(&mut draft, op).code(), ErrorCode::StreamPhase);
    }
    assert!(draft.response().is_none());
}
