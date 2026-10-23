mod common;

use br_llm_messages::{ClosureReason, ErrorCode, ResponseEnding, TurnEntry, TurnEntryKind};
use common::turns::{
    calling, close, delivery, finished, hosted_call, hosted_result, human, incoming_batch,
    open_turn, pushed, response,
};
use common::{bid, call, refused, text_block};

#[test]
fn given_outstanding_calls_when_a_response_is_pushed_then_unresolved_execution() {
    let mut turn = open_turn(1);
    pushed(&mut turn, vec![calling(10, &[(11, "a"), (12, "b")])]);

    let error = refused(turn.push_entry(finished(13)));

    assert_eq!(error.code(), ErrorCode::UnresolvedExecution);
    assert_eq!(turn.entries().len(), 1);
}

#[test]
fn given_results_over_several_deliveries_when_all_collected_then_the_next_response_is_accepted() {
    let mut turn = open_turn(1);
    pushed(
        &mut turn,
        vec![calling(10, &[(11, "a"), (12, "b"), (13, "c")])],
    );

    pushed(&mut turn, vec![delivery(20, &[(21, 11, "a")])]);
    assert_eq!(
        refused(turn.push_entry(finished(30))).code(),
        ErrorCode::UnresolvedExecution
    );
    pushed(
        &mut turn,
        vec![delivery(22, &[(23, 13, "c"), (24, 12, "b")])],
    );
    turn.push_entry(finished(31)).unwrap();

    assert_eq!(turn.entries().len(), 4);
    assert!(turn.state().outstanding_runtime_calls().is_empty());
}

#[test]
fn given_a_provider_result_in_a_later_response_when_pushed_then_the_link_by_identity_holds() {
    let mut turn = open_turn(1);
    pushed(
        &mut turn,
        vec![response(
            10,
            vec![hosted_call(11), text_block(12, "between").into()],
            ResponseEnding::provider_continuation(),
        )],
    );

    turn.push_entry(response(
        13,
        vec![text_block(14, "other").into(), hosted_result(15, 11)],
        ResponseEnding::finished(),
    ))
    .unwrap();

    assert_eq!(turn.entries().len(), 2);
}

#[test]
fn given_a_provider_result_before_its_call_in_one_response_when_pushed_then_accepted() {
    let mut turn = open_turn(1);

    turn.push_entry(response(
        10,
        vec![hosted_result(11, 12), hosted_call(12)],
        ResponseEnding::finished(),
    ))
    .unwrap();
}

#[test]
fn given_a_provider_result_linking_nothing_when_pushed_then_unknown_reference() {
    let mut turn = open_turn(1);

    let error = refused(turn.push_entry(response(
        10,
        vec![hosted_result(11, 99)],
        ResponseEnding::finished(),
    )));

    assert_eq!(error.code(), ErrorCode::UnknownReference);
    assert_eq!(error.object_id(), Some(bid(99).as_uuid()));
    assert_eq!(error.location(), Some("content[0].call"));
}

#[test]
fn given_a_provider_result_linking_a_wrong_kind_when_pushed_then_wrong_kind_reference() {
    let mut turn = open_turn(1);
    pushed(
        &mut turn,
        vec![response(
            10,
            vec![call(11, "t").into(), text_block(12, "x").into()],
            ResponseEnding::tool_results_requested(),
        )],
    );
    pushed(&mut turn, vec![delivery(20, &[(21, 11, "t")])]);

    for target in [11, 12] {
        let error = refused(turn.push_entry(response(
            30 + target,
            vec![hosted_result(40 + target, target)],
            ResponseEnding::finished(),
        )));
        assert_eq!(error.code(), ErrorCode::WrongKindReference);
    }
}

#[test]
fn given_provider_calls_and_results_when_the_turn_runs_then_no_runtime_result_is_required() {
    let mut turn = open_turn(1);
    pushed(
        &mut turn,
        vec![
            response(
                10,
                vec![hosted_call(11), hosted_result(12, 11)],
                ResponseEnding::finished(),
            ),
            finished(13),
            close(14, ClosureReason::completed()),
        ],
    );

    assert!(turn.state().outstanding_runtime_calls().is_empty());
}

#[test]
fn given_incoming_entries_at_any_point_when_pushed_then_accepted_with_kinds_preserved() {
    let mut turn = open_turn(1);
    let batch = incoming_batch(100);
    assert!(batch.iter().all(TurnEntry::is_incoming));

    pushed(&mut turn, batch[..3].to_vec());
    pushed(&mut turn, vec![calling(300, &[(301, "a")])]);
    pushed(&mut turn, batch[3..].to_vec());
    pushed(
        &mut turn,
        vec![delivery(310, &[(311, 301, "a")]), human(320)],
    );
    pushed(&mut turn, vec![finished(330)]);
    pushed(&mut turn, incoming_batch(400));

    let kinds: Vec<_> = turn.entries().iter().map(TurnEntry::kind).collect();
    let mut expected: Vec<_> = batch.iter().map(TurnEntry::kind).collect();
    expected.insert(3, TurnEntryKind::ModelResponse);
    expected.extend([
        TurnEntryKind::RuntimeToolResults,
        TurnEntryKind::HumanMessage,
        TurnEntryKind::ModelResponse,
    ]);
    expected.extend(incoming_batch(400).iter().map(TurnEntry::kind));
    assert_eq!(kinds, expected);
    assert!(!turn.is_closed());
}

fn call_ending(entry: u64, call_id: u64, ending: ResponseEnding) -> TurnEntry {
    response(entry, vec![call(call_id, "a").into()], ending)
}

#[test]
fn given_a_call_under_a_non_requesting_ending_when_more_is_pushed_then_no_result_is_required() {
    for ending in [
        ResponseEnding::finished(),
        ResponseEnding::provider_continuation(),
        ResponseEnding::output_exhausted(),
        ResponseEnding::stop_sequence(),
    ] {
        let mut turn = open_turn(1);
        pushed(&mut turn, vec![call_ending(10, 11, ending)]);

        assert!(turn.state().outstanding_runtime_calls().is_empty());
        turn.push_entry(finished(12)).unwrap();
        turn.push_entry(close(13, ClosureReason::completed()))
            .unwrap();
        assert!(turn.is_closed());
    }
}

#[test]
fn given_a_call_under_a_requesting_ending_when_more_is_pushed_then_both_are_blocked() {
    let mut turn = open_turn(1);
    pushed(
        &mut turn,
        vec![call_ending(
            10,
            11,
            ResponseEnding::tool_results_requested(),
        )],
    );

    assert_eq!(turn.state().outstanding_runtime_calls(), &[bid(11)]);
    assert_eq!(
        refused(turn.push_entry(finished(12))).code(),
        ErrorCode::UnresolvedExecution
    );
    assert_eq!(
        refused(turn.push_entry(close(13, ClosureReason::completed()))).code(),
        ErrorCode::UnresolvedExecution
    );
}

#[test]
fn given_a_call_under_a_non_requesting_ending_when_a_result_arrives_then_it_is_checked_by_identity()
{
    let mut turn = open_turn(1);
    pushed(
        &mut turn,
        vec![call_ending(10, 11, ResponseEnding::finished())],
    );

    assert_eq!(
        refused(turn.push_entry(delivery(20, &[(21, 11, "wrong")]))).code(),
        ErrorCode::ToolResultMismatch
    );
    turn.push_entry(delivery(22, &[(23, 11, "a")])).unwrap();
    assert_eq!(
        refused(turn.push_entry(delivery(24, &[(25, 11, "a")]))).code(),
        ErrorCode::DuplicateToolResult
    );
}
