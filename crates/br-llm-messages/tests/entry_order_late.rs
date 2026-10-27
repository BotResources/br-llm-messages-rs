mod common;
mod stream_support;

use br_llm_messages::{AITurn, ClosureReason, ErrorCode, ResponseEnding};
use common::turns::{
    calling, close, delivery, finished, human, open_turn, pushed, response, tid as turn_id,
};
use common::{aid, author, refused};

fn interrupted_with_two_calls() -> AITurn {
    let mut turn = open_turn(1);
    pushed(
        &mut turn,
        vec![
            calling(10, &[(11, "a"), (12, "b")]),
            close(20, ClosureReason::interrupted()),
        ],
    );
    turn
}

#[test]
fn given_an_abnormal_closure_with_an_outstanding_call_when_its_result_arrives_then_it_is_recorded_and_no_longer_outstanding()
 {
    let mut turn = interrupted_with_two_calls();

    turn.push_entry(delivery(30, &[(31, 11, "a")])).unwrap();

    assert_eq!(turn.entries().len(), 3);
    assert!(turn.is_closed());
    assert_eq!(turn.state().outstanding_runtime_calls().len(), 1);
    assert_eq!(turn.state().outstanding_runtime_calls()[0], common::bid(12));
}

#[test]
fn given_a_closed_turn_when_every_left_call_is_answered_then_a_further_result_is_closed_turn() {
    let mut turn = interrupted_with_two_calls();
    turn.push_entry(delivery(30, &[(31, 11, "a")])).unwrap();
    turn.push_entry(delivery(32, &[(33, 12, "b")])).unwrap();

    let error = refused(turn.push_entry(delivery(34, &[(35, 12, "b")])));

    assert_eq!(error.code(), ErrorCode::ClosedTurn);
    assert!(turn.state().outstanding_runtime_calls().is_empty());
}

#[test]
fn given_a_closed_turn_when_anything_but_late_results_arrives_then_closed_turn_and_nothing_changes()
{
    let mut turn = interrupted_with_two_calls();
    let before = turn.clone();

    for entry in [
        human(40),
        finished(41),
        close(42, ClosureReason::cancelled()),
        close(43, ClosureReason::completed()),
    ] {
        let error = refused(turn.push_entry(entry.clone()));
        assert_eq!(error.code(), ErrorCode::ClosedTurn);
        assert_eq!(error.object_id(), Some(entry.id().as_uuid()));
    }
    assert_eq!(turn, before);
}

#[test]
fn given_a_closed_turn_when_late_results_name_a_wrong_call_then_the_result_rules_still_apply() {
    let mut turn = interrupted_with_two_calls();
    turn.push_entry(delivery(30, &[(31, 11, "a")])).unwrap();
    let before = turn.clone();

    let unknown = refused(turn.push_entry(delivery(32, &[(33, 99, "b")])));
    let duplicate = refused(turn.push_entry(delivery(34, &[(35, 11, "a")])));
    let mismatched = refused(turn.push_entry(delivery(36, &[(37, 12, "a")])));

    assert_eq!(unknown.code(), ErrorCode::UnknownReference);
    assert_eq!(duplicate.code(), ErrorCode::DuplicateResult);
    assert_eq!(mismatched.code(), ErrorCode::MismatchedResult);
    assert_eq!(turn, before);
}

#[test]
fn given_a_late_result_when_the_turn_is_reloaded_from_json_then_it_loads_with_the_same_state() {
    let mut turn = interrupted_with_two_calls();
    turn.push_entry(delivery(30, &[(31, 11, "a")])).unwrap();

    let reloaded = AITurn::from_json(&turn.to_json().unwrap()).unwrap();

    assert_eq!(reloaded, turn);
    assert_eq!(reloaded.state(), turn.state());
}

#[test]
fn given_json_with_a_non_result_entry_after_a_closure_when_loaded_then_closed_turn() {
    let turn = interrupted_with_two_calls();
    let mut value = serde_json::to_value(&turn).unwrap();
    value["entries"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::to_value(human(40)).unwrap());

    let error = refused(AITurn::from_value(common::oj(value)));

    assert_eq!(error.code(), ErrorCode::ClosedTurn);
    assert_eq!(error.location(), Some("entries[2]"));
}

#[test]
fn given_json_with_results_naming_no_outstanding_call_when_loaded_then_entry_out_of_order() {
    let mut turn = open_turn(1);
    pushed(&mut turn, vec![finished(10)]);
    let mut value = serde_json::to_value(&turn).unwrap();
    value["entries"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::to_value(delivery(20, &[(21, 11, "a")])).unwrap());

    let error = refused(AITurn::from_value(common::oj(value)));

    assert_eq!(error.code(), ErrorCode::EntryOutOfOrder);
    assert_eq!(error.location(), Some("entries[1]"));
}

#[test]
fn given_json_with_free_order_when_loaded_then_it_is_accepted_like_a_direct_build() {
    let entries = vec![
        calling(10, &[(11, "a")]),
        human(12),
        finished(13),
        response(14, vec![], ResponseEnding::provider_continuation()),
        delivery(20, &[(21, 11, "a")]),
        finished(15),
    ];
    let built = AITurn::from_parts(turn_id(1), aid(1), author("Codex"), entries).unwrap();

    let reloaded = AITurn::from_json(&built.to_json().unwrap()).unwrap();

    assert_eq!(reloaded, built);
    assert!(reloaded.state().outstanding_runtime_calls().is_empty());
}

#[test]
fn given_a_response_built_from_a_draft_when_added_while_a_call_is_outstanding_then_it_is_accepted()
{
    use stream_support::{finalize, finish, run, start_text, text};
    let mut turn = open_turn(1);
    pushed(&mut turn, vec![calling(10, &[(11, "a")])]);
    let draft = run(vec![start_text(30), text(30, "x"), finalize(30), finish()]);

    turn.push_entry(draft.into_response().unwrap().into())
        .unwrap();

    assert_eq!(turn.entries().len(), 2);
    assert_eq!(turn.state().outstanding_runtime_calls().len(), 1);
}

#[test]
fn given_a_response_built_from_a_draft_when_added_to_a_closed_turn_then_closed_turn() {
    use stream_support::{finalize, finish, run, start_text, text};
    let mut turn = interrupted_with_two_calls();
    let draft = run(vec![start_text(30), text(30, "x"), finalize(30), finish()]);

    let error = refused(turn.push_entry(draft.into_response().unwrap().into()));

    assert_eq!(error.code(), ErrorCode::ClosedTurn);
}

#[test]
fn given_a_completed_closure_when_a_call_is_outstanding_or_a_continuation_trails_then_unresolved_execution()
 {
    let mut with_call = open_turn(1);
    pushed(
        &mut with_call,
        vec![calling(10, &[(11, "a")]), human(12), finished(13)],
    );
    let mut continuing = open_turn(2);
    pushed(
        &mut continuing,
        vec![
            response(10, vec![], ResponseEnding::provider_continuation()),
            human(12),
        ],
    );

    for turn in [&mut with_call, &mut continuing] {
        let error = refused(turn.push_entry(close(50, ClosureReason::completed())));
        assert_eq!(error.code(), ErrorCode::UnresolvedExecution);
        assert!(!turn.is_closed());
        turn.push_entry(close(51, ClosureReason::failed("boom").unwrap()))
            .unwrap();
    }
}

#[test]
fn given_a_dry_run_when_checking_entries_then_it_gives_the_refusal_push_would_give_and_mutates_nothing()
 {
    let mut turn = open_turn(1);
    pushed(&mut turn, vec![calling(10, &[(11, "a")])]);
    let before = turn.clone();
    let entries = [
        human(20),
        finished(21),
        delivery(22, &[(23, 11, "a")]),
        delivery(24, &[(25, 99, "a")]),
        delivery(26, &[(27, 11, "b")]),
        close(28, ClosureReason::completed()),
        close(29, ClosureReason::cancelled()),
        human(10),
    ];

    for entry in entries {
        let dry = turn.would_refuse(&entry);
        assert_eq!(turn, before);
        let mut attempt = turn.clone();
        let real = attempt.push_entry(entry.clone());
        assert_eq!(dry, real.err());
    }
}

#[test]
fn given_a_closed_turn_when_dry_running_then_late_results_pass_and_the_rest_is_closed_turn() {
    let turn = interrupted_with_two_calls();

    assert_eq!(turn.would_refuse(&delivery(30, &[(31, 11, "a")])), None);
    assert_eq!(
        turn.would_refuse(&human(32)).unwrap().code(),
        ErrorCode::ClosedTurn
    );
}

#[test]
fn given_a_response_calling_tools_while_calls_are_outstanding_when_results_follow_then_both_batches_resolve()
 {
    let mut turn = open_turn(1);
    pushed(
        &mut turn,
        vec![calling(10, &[(11, "a")]), calling(12, &[(13, "b")])],
    );

    turn.push_entry(delivery(20, &[(21, 13, "b"), (22, 11, "a")]))
        .unwrap();

    assert!(turn.state().outstanding_runtime_calls().is_empty());
}
