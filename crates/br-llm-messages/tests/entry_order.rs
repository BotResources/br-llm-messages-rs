mod common;

use br_llm_messages::{ErrorCode, ExecutionStatus, ResponseEnding};
use common::order::{accepts, after, judge_row, runtime_call_response};
use common::refused;
use common::turns::{
    calling, finished, hosted_record, human, incoming_batch, open_turn, pushed, response,
};

#[test]
fn given_a_just_opened_turn_when_entries_are_pushed_then_incoming_response_and_closure_are_accepted()
 {
    judge_row(
        "row 1",
        &open_turn(1),
        accepts(true, true, false, true, true),
    );
}

#[test]
fn given_an_incoming_entry_last_when_entries_are_pushed_then_incoming_response_and_closure_are_accepted()
 {
    judge_row(
        "row 2",
        &after(vec![human(10)]),
        accepts(true, true, false, true, true),
    );
}

#[test]
fn given_every_incoming_kind_last_when_entries_are_pushed_then_the_row_2_outcomes_hold() {
    for (index, entry) in incoming_batch(100).into_iter().enumerate() {
        judge_row(
            &format!("incoming {index}"),
            &after(vec![entry]),
            accepts(true, true, false, true, true),
        );
    }
}

#[test]
fn given_a_response_with_an_outstanding_call_when_entries_are_pushed_then_only_results_or_abnormal_closure()
 {
    judge_row(
        "row 3",
        &after(vec![calling(10, &[(11, "a")])]),
        accepts(false, false, true, false, true),
    );
}

#[test]
fn given_a_runtime_call_under_any_ending_when_entries_are_pushed_then_the_call_is_outstanding() {
    let endings = [
        ResponseEnding::finished(),
        ResponseEnding::tool_use(),
        ResponseEnding::provider_continuation(),
        ResponseEnding::output_exhaustion(),
        ResponseEnding::stop_sequence(),
        ResponseEnding::refusal(),
        ResponseEnding::filtering(),
        ResponseEnding::unclassified("odd").unwrap(),
    ];
    for ending in endings {
        let turn = after(vec![runtime_call_response(ending)]);

        assert_eq!(turn.state().outstanding_runtime_calls().len(), 1);
        judge_row(
            "row 3, any ending",
            &turn,
            accepts(false, false, true, false, true),
        );
    }
}

#[test]
fn given_a_provider_continuation_without_outstanding_call_when_entries_are_pushed_then_response_or_abnormal_closure()
 {
    judge_row(
        "row 4, continuation",
        &after(vec![response(
            10,
            vec![],
            ResponseEnding::provider_continuation(),
        )]),
        accepts(false, true, false, false, true),
    );
}

#[test]
fn given_a_pending_provider_execution_without_outstanding_call_when_entries_are_pushed_then_response_or_abnormal_closure()
 {
    for status in [ExecutionStatus::Pending, ExecutionStatus::Running] {
        let turn = after(vec![response(
            10,
            vec![hosted_record(12, status)],
            ResponseEnding::finished(),
        )]);

        judge_row(
            "row 4, pending",
            &turn,
            accepts(false, true, false, false, true),
        );
    }
}

#[test]
fn given_a_pending_execution_under_output_exhaustion_when_entries_are_pushed_then_the_continuation_row_applies()
 {
    let turn = after(vec![response(
        10,
        vec![hosted_record(12, ExecutionStatus::Running)],
        ResponseEnding::output_exhaustion(),
    )]);

    judge_row(
        "row 4 before row 5",
        &turn,
        accepts(false, true, false, false, true),
    );
}

#[test]
fn given_an_execution_that_is_not_pending_when_entries_are_pushed_then_it_does_not_block() {
    let turn = after(vec![response(
        10,
        vec![hosted_record(12, ExecutionStatus::Completed)],
        ResponseEnding::finished(),
    )]);

    judge_row(
        "finished record",
        &turn,
        accepts(true, false, false, true, true),
    );
}

#[test]
fn given_output_exhaustion_without_outstanding_call_when_entries_are_pushed_then_incoming_response_or_closure()
 {
    judge_row(
        "row 5",
        &after(vec![response(
            10,
            vec![],
            ResponseEnding::output_exhaustion(),
        )]),
        accepts(true, true, false, true, true),
    );
}

#[test]
fn given_any_other_response_when_entries_are_pushed_then_incoming_or_closure_and_no_second_response()
 {
    let endings = [
        ResponseEnding::finished(),
        ResponseEnding::stop_sequence(),
        ResponseEnding::matched_stop_sequence("END").unwrap(),
        ResponseEnding::refusal(),
        ResponseEnding::filtering(),
        ResponseEnding::unclassified("odd").unwrap(),
    ];
    for ending in endings {
        judge_row(
            "row 6",
            &after(vec![response(10, vec![], ending)]),
            accepts(true, false, false, true, true),
        );
    }
}

#[test]
fn given_an_incoming_entry_after_a_response_when_a_response_follows_then_the_second_response_rule_is_reset()
 {
    let mut turn = after(vec![finished(10)]);
    assert_eq!(
        refused(turn.push_entry(finished(11))).code(),
        ErrorCode::EntryOutOfOrder
    );

    pushed(&mut turn, vec![human(12), finished(13)]);

    assert_eq!(turn.entries().len(), 3);
}
