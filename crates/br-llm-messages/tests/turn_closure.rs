mod common;

use br_llm_messages::{
    BudgetKind, ClosureReason, ErrorCode, ExecutionStatus, FailureDetail, ResponseEnding, TurnEntry,
};
use common::turns::{
    calling, close, delivery, finished, hosted_call, hosted_record, open_turn, pushed, response,
};
use common::{bid, call, refused};

#[test]
fn given_outstanding_calls_when_completed_closure_pushed_then_unresolved_execution() {
    let mut turn = open_turn(1);
    pushed(&mut turn, vec![calling(10, &[(11, "a")])]);

    let error = refused(turn.push_entry(close(20, ClosureReason::completed())));

    assert_eq!(error.code(), ErrorCode::UnresolvedExecution);
    assert!(!turn.is_closed());
}

#[test]
fn given_a_pending_provider_execution_when_completed_closure_pushed_then_unresolved_execution() {
    for status in [ExecutionStatus::Pending, ExecutionStatus::Running] {
        let mut turn = open_turn(1);
        pushed(
            &mut turn,
            vec![response(
                10,
                vec![hosted_record(11, status)],
                ResponseEnding::finished(),
            )],
        );

        let error = refused(turn.push_entry(close(20, ClosureReason::completed())));

        assert_eq!(error.code(), ErrorCode::UnresolvedExecution);
    }
}

#[test]
fn given_settled_provider_executions_when_completed_closure_pushed_then_accepted() {
    for status in [
        Some(ExecutionStatus::Completed),
        Some(ExecutionStatus::Failed),
        Some(ExecutionStatus::Incomplete),
        None,
    ] {
        let mut turn = open_turn(1);
        let record = match status {
            Some(status) => hosted_record(11, status),
            None => hosted_call(11),
        };
        pushed(
            &mut turn,
            vec![response(10, vec![record], ResponseEnding::finished())],
        );

        turn.push_entry(close(20, ClosureReason::completed()))
            .unwrap();
    }
}

#[test]
fn given_a_last_response_ending_in_provider_continuation_when_completed_then_continuation_pending()
{
    let mut turn = open_turn(1);
    pushed(
        &mut turn,
        vec![response(
            10,
            vec![],
            ResponseEnding::provider_continuation(),
        )],
    );

    let error = refused(turn.push_entry(close(20, ClosureReason::completed())));

    assert_eq!(error.code(), ErrorCode::ContinuationPending);
}

#[test]
fn given_a_continuation_followed_by_a_finished_response_when_completed_then_accepted() {
    let mut turn = open_turn(1);
    pushed(
        &mut turn,
        vec![
            response(10, vec![], ResponseEnding::provider_continuation()),
            finished(11),
            close(12, ClosureReason::completed()),
        ],
    );
}

#[test]
fn given_results_delivered_and_no_further_response_when_completed_then_accepted() {
    let mut turn = open_turn(1);
    pushed(
        &mut turn,
        vec![
            calling(10, &[(11, "terminal")]),
            delivery(20, &[(21, 11, "terminal")]),
            close(30, ClosureReason::completed()),
        ],
    );

    assert!(turn.is_closed());
}

#[test]
fn given_outstanding_calls_when_abnormal_closure_pushed_then_accepted_without_synthetic_results() {
    let reasons = [
        ClosureReason::cancelled(None).unwrap(),
        ClosureReason::interrupted(FailureDetail::new("kill").unwrap()),
        ClosureReason::budget_exhausted(BudgetKind::Tokens),
        ClosureReason::failed(FailureDetail::new("boom").unwrap()),
    ];
    for reason in reasons {
        let mut turn = open_turn(1);
        pushed(&mut turn, vec![calling(10, &[(11, "a"), (12, "b")])]);

        turn.push_entry(close(20, reason.clone())).unwrap();

        assert_eq!(turn.entries().len(), 2);
        assert!(
            !turn
                .entries()
                .iter()
                .any(|entry| matches!(entry, TurnEntry::RuntimeToolResults(_)))
        );
        assert_eq!(turn.state().closure(), Some(&reason));
        assert_eq!(turn.state().outstanding_runtime_calls(), [bid(11), bid(12)]);
    }
}

#[test]
fn given_abnormal_closure_after_a_continuation_when_pushed_then_accepted() {
    let mut turn = open_turn(1);
    pushed(
        &mut turn,
        vec![response(
            10,
            vec![],
            ResponseEnding::provider_continuation(),
        )],
    );

    turn.push_entry(close(20, ClosureReason::cancelled(None).unwrap()))
        .unwrap();
}

#[test]
fn given_an_empty_turn_when_state_read_then_nothing_is_reported() {
    let state = open_turn(1).state();

    assert_eq!(state.closure(), None);
    assert!(state.outstanding_runtime_calls().is_empty());
    assert!(state.pending_provider_executions().is_empty());
    assert_eq!(state.last_ending(), None);
}

#[test]
fn given_runtime_calls_and_provider_executions_when_state_read_then_each_fact_is_distinct() {
    let mut turn = open_turn(1);
    pushed(
        &mut turn,
        vec![response(
            10,
            vec![
                call(11, "a").into(),
                hosted_record(12, ExecutionStatus::Running),
                hosted_record(13, ExecutionStatus::Completed),
                hosted_call(14),
            ],
            ResponseEnding::tool_results_requested(),
        )],
    );

    let state = turn.state();

    assert_eq!(state.outstanding_runtime_calls(), [bid(11)]);
    assert_eq!(state.pending_provider_executions(), [bid(12)]);
    assert_eq!(
        state.last_ending(),
        Some(&ResponseEnding::tool_results_requested())
    );
    assert_eq!(state.closure(), None);
}

#[test]
fn given_a_later_response_when_state_read_then_pending_executions_come_from_the_last_response_only()
{
    let mut turn = open_turn(1);
    pushed(
        &mut turn,
        vec![
            response(
                10,
                vec![hosted_record(11, ExecutionStatus::Pending)],
                ResponseEnding::provider_continuation(),
            ),
            response(
                12,
                vec![hosted_record(13, ExecutionStatus::Completed)],
                ResponseEnding::finished(),
            ),
        ],
    );

    let state = turn.state();

    assert!(state.pending_provider_executions().is_empty());
    assert_eq!(state.last_ending(), Some(&ResponseEnding::finished()));
}

#[test]
fn given_results_for_some_calls_when_state_read_then_only_the_unanswered_remain_outstanding() {
    let mut turn = open_turn(1);
    pushed(
        &mut turn,
        vec![
            calling(10, &[(11, "a"), (12, "b"), (13, "c")]),
            delivery(20, &[(21, 12, "b")]),
        ],
    );

    assert_eq!(turn.state().outstanding_runtime_calls(), [bid(11), bid(13)]);
}

#[test]
fn given_a_provider_call_without_exposed_result_when_state_read_then_it_is_not_outstanding() {
    let mut turn = open_turn(1);
    pushed(
        &mut turn,
        vec![response(
            10,
            vec![
                hosted_call(11),
                hosted_record(12, ExecutionStatus::Completed),
            ],
            ResponseEnding::finished(),
        )],
    );

    assert!(turn.state().outstanding_runtime_calls().is_empty());
    assert!(turn.state().pending_provider_executions().is_empty());
}
