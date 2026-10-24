mod common;

use br_llm_messages::{ClosureReason, ExecutionStatus, ResponseEnding};
use common::call;
use common::order::{NOTHING, accepts, after, judge_row, runtime_call_response};
use common::turns::{calling, close, delivery, hosted_record, response};

#[test]
fn given_results_with_a_call_still_outstanding_when_entries_are_pushed_then_results_or_abnormal_closure()
 {
    let turn = after(vec![
        calling(10, &[(13, "z"), (11, "a")]),
        delivery(20, &[(21, 13, "z")]),
    ]);

    judge_row("row 7", &turn, accepts(false, false, true, false, true));
}

#[test]
fn given_answered_results_after_a_provider_continuation_when_entries_are_pushed_then_response_or_abnormal_closure()
 {
    let turn = after(vec![
        runtime_call_response(ResponseEnding::provider_continuation()),
        delivery(20, &[(21, 11, "a")]),
    ]);

    judge_row(
        "row 8, continuation",
        &turn,
        accepts(false, true, false, false, true),
    );
}

#[test]
fn given_answered_results_after_a_pending_execution_when_entries_are_pushed_then_response_or_abnormal_closure()
 {
    let turn = after(vec![
        response(
            10,
            vec![
                call(11, "a").into(),
                hosted_record(12, ExecutionStatus::Running),
            ],
            ResponseEnding::tool_use(),
        ),
        delivery(20, &[(21, 11, "a")]),
    ]);

    judge_row(
        "row 8, pending",
        &turn,
        accepts(false, true, false, false, true),
    );
}

#[test]
fn given_any_other_answered_results_when_entries_are_pushed_then_incoming_response_or_closure() {
    for ending in [
        ResponseEnding::tool_use(),
        ResponseEnding::finished(),
        ResponseEnding::output_exhaustion(),
    ] {
        let turn = after(vec![
            runtime_call_response(ending),
            delivery(20, &[(21, 11, "a")]),
        ]);

        judge_row("row 9", &turn, accepts(true, true, false, true, true));
    }
}

#[test]
fn given_a_closure_last_when_any_entry_is_pushed_then_closed_turn() {
    for closure in [
        ClosureReason::completed(),
        ClosureReason::cancelled(),
        ClosureReason::interrupted(),
    ] {
        let turn = after(vec![close(10, closure)]);

        judge_row("row 10", &turn, NOTHING);
    }
}

#[test]
fn given_an_abnormal_closure_after_an_unresolved_call_when_state_is_read_then_the_call_stays_unresolved()
 {
    let turn = after(vec![
        calling(10, &[(11, "a")]),
        close(20, ClosureReason::interrupted()),
    ]);

    assert_eq!(turn.state().outstanding_runtime_calls().len(), 1);
    assert!(turn.is_closed());
}
