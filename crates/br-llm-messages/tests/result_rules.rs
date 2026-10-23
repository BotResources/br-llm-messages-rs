mod common;

use br_llm_messages::{AITurn, ErrorCode, ResponseEnding, TurnEntry};
use common::turns::{calling, delivery, finished, hosted_call, open_turn, pushed, response};
use common::{bid, eid, refused, text_block};

fn turn_with_calls() -> AITurn {
    let mut turn = open_turn(1);
    pushed(&mut turn, vec![calling(10, &[(11, "alpha"), (12, "beta")])]);
    turn
}

#[test]
fn given_a_result_to_an_unknown_call_when_pushed_then_unknown_reference_and_nothing_changes() {
    let mut turn = turn_with_calls();
    let before = turn.clone();

    let error = refused(turn.push_entry(delivery(20, &[(21, 99, "alpha")])));

    assert_eq!(error.code(), ErrorCode::UnknownReference);
    assert_eq!(error.object_id(), Some(bid(99).as_uuid()));
    assert_eq!(error.location(), Some("results[0]"));
    assert_eq!(turn, before);
}

#[test]
fn given_a_result_to_a_provider_call_when_pushed_then_unknown_reference() {
    let mut turn = open_turn(1);
    pushed(
        &mut turn,
        vec![response(
            10,
            vec![hosted_call(11)],
            ResponseEnding::finished(),
        )],
    );

    let error = refused(turn.push_entry(delivery(20, &[(21, 11, "alpha")])));

    assert_eq!(error.code(), ErrorCode::UnknownReference);
}

#[test]
fn given_a_result_to_a_non_call_id_when_pushed_then_unknown_reference() {
    let mut turn = open_turn(1);
    pushed(
        &mut turn,
        vec![response(
            10,
            vec![text_block(11, "plain").into()],
            ResponseEnding::finished(),
        )],
    );

    for target in [10, 11] {
        let error = refused(turn.push_entry(delivery(20, &[(21, target, "alpha")])));
        assert_eq!(error.code(), ErrorCode::UnknownReference);
    }
}

#[test]
fn given_a_call_answered_in_an_earlier_delivery_when_answered_again_then_duplicate_result() {
    let mut turn = turn_with_calls();
    pushed(&mut turn, vec![delivery(20, &[(21, 11, "alpha")])]);
    let before = turn.clone();

    let error = refused(turn.push_entry(delivery(30, &[(31, 12, "beta"), (32, 11, "alpha")])));

    assert_eq!(error.code(), ErrorCode::DuplicateResult);
    assert_eq!(error.object_id(), Some(bid(11).as_uuid()));
    assert_eq!(error.location(), Some("results[1]"));
    assert_eq!(turn, before);
}

#[test]
fn given_a_result_naming_another_tool_when_pushed_then_mismatched_result() {
    let mut turn = turn_with_calls();

    let error = refused(turn.push_entry(delivery(20, &[(21, 11, "beta")])));

    assert_eq!(error.code(), ErrorCode::MismatchedResult);
    assert_eq!(turn.state().outstanding_runtime_calls().len(), 2);
}

#[test]
fn given_a_valid_and_an_invalid_result_in_one_delivery_when_pushed_then_none_is_applied() {
    let mut turn = turn_with_calls();
    let before = turn.clone();

    let error = refused(turn.push_entry(delivery(20, &[(21, 11, "alpha"), (22, 12, "alpha")])));

    assert_eq!(error.code(), ErrorCode::MismatchedResult);
    assert_eq!(turn, before);
    assert_eq!(turn.state().outstanding_runtime_calls(), [bid(11), bid(12)]);
}

#[test]
fn given_a_result_id_already_used_in_the_turn_when_pushed_then_reused_identity() {
    let mut turn = turn_with_calls();

    let error = refused(turn.push_entry(delivery(20, &[(11, 12, "beta")])));

    assert_eq!(error.code(), ErrorCode::ReusedIdentity);
    assert_eq!(error.object_id(), Some(bid(11).as_uuid()));
}

#[test]
fn given_results_in_reverse_call_order_when_pushed_then_the_recorded_order_is_kept() {
    let mut turn = turn_with_calls();

    pushed(
        &mut turn,
        vec![delivery(20, &[(21, 12, "beta"), (22, 11, "alpha")])],
    );

    let TurnEntry::RuntimeToolResults(delivery) = &turn.entries()[1] else {
        panic!("a delivery follows the response");
    };
    let calls: Vec<_> = delivery.results().iter().map(|r| r.call()).collect();
    assert_eq!(calls, [bid(12), bid(11)]);
}

#[test]
fn given_two_deliveries_when_the_turn_is_reloaded_then_they_stay_two_entries_in_order() {
    let mut turn = turn_with_calls();
    pushed(
        &mut turn,
        vec![
            delivery(20, &[(21, 12, "beta")]),
            delivery(30, &[(31, 11, "alpha")]),
            finished(40),
        ],
    );

    let reloaded = AITurn::from_json(&turn.to_json().unwrap()).unwrap();

    assert_eq!(reloaded, turn);
    let ids: Vec<_> = reloaded.entries().iter().map(TurnEntry::id).collect();
    assert_eq!(ids, [eid(10), eid(20), eid(30), eid(40)]);
}
