mod common;

use br_llm_messages::{ErrorCode, ExecutionStatus, ResponseEnding};
use common::order::{accepts, after, judge_row, runtime_call_response};
use common::refused;
use common::turns::{
    calling, delivery, finished, hosted_record, human, incoming_batch, open_turn, response,
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
fn given_a_response_with_an_outstanding_call_when_entries_are_pushed_then_everything_is_accepted_but_a_completed_closure()
 {
    judge_row(
        "row 3",
        &after(vec![calling(10, &[(11, "a")])]),
        accepts(true, true, true, false, true),
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
            accepts(true, true, true, false, true),
        );
    }
}

#[test]
fn given_a_provider_continuation_without_outstanding_call_when_entries_are_pushed_then_everything_is_accepted_but_results_and_a_completed_closure()
 {
    judge_row(
        "row 4, continuation",
        &after(vec![response(
            10,
            vec![],
            ResponseEnding::provider_continuation(),
        )]),
        accepts(true, true, false, false, true),
    );
}

#[test]
fn given_a_pending_provider_execution_without_outstanding_call_when_entries_are_pushed_then_everything_is_accepted_but_results_and_a_completed_closure()
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
            accepts(true, true, false, false, true),
        );
    }
}

#[test]
fn given_a_pending_execution_under_output_exhaustion_when_entries_are_pushed_then_the_continuation_rule_applies()
 {
    let turn = after(vec![response(
        10,
        vec![hosted_record(12, ExecutionStatus::Running)],
        ResponseEnding::output_exhaustion(),
    )]);

    judge_row(
        "row 4 before row 5",
        &turn,
        accepts(true, true, false, false, true),
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
        accepts(true, true, false, true, true),
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
fn given_any_other_response_when_entries_are_pushed_then_incoming_response_and_closure_are_accepted()
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
            accepts(true, true, false, true, true),
        );
    }
}

#[test]
fn given_a_response_last_when_another_response_follows_then_it_is_accepted_whatever_the_ending() {
    let mut turn = after(vec![finished(10)]);

    turn.push_entry(finished(11)).unwrap();
    turn.push_entry(response(
        12,
        vec![],
        ResponseEnding::provider_continuation(),
    ))
    .unwrap();
    turn.push_entry(finished(13)).unwrap();

    assert_eq!(turn.entries().len(), 4);
}

#[test]
fn given_a_results_probe_when_no_call_is_outstanding_then_entry_out_of_order_with_the_entry_identity()
 {
    let mut turn = after(vec![finished(10)]);
    let probe = delivery(20, &[(21, 11, "a")]);

    let error = refused(turn.push_entry(probe.clone()));

    assert_eq!(error.code(), ErrorCode::EntryOutOfOrder);
    assert_eq!(error.object_id(), Some(probe.id().as_uuid()));
    assert_eq!(turn.entries().len(), 1);
}
