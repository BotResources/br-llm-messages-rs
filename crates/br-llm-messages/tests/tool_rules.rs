mod common;

use br_llm_messages::{AITurn, ErrorCode, ResponseEnding, TurnEntry};
use common::turns::{
    calling, delivery, finished, hosted_call, mount, open_turn, pushed, response, unmount,
};
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
fn given_a_result_to_a_provider_call_when_pushed_then_wrong_kind_reference() {
    let mut turn = open_turn(1);
    pushed(
        &mut turn,
        vec![response(
            10,
            vec![hosted_call(11)],
            ResponseEnding::Finished,
        )],
    );

    let error = refused(turn.push_entry(delivery(20, &[(21, 11, "alpha")])));

    assert_eq!(error.code(), ErrorCode::WrongKindReference);
}

#[test]
fn given_a_result_to_a_non_call_id_when_pushed_then_wrong_kind_reference() {
    let mut turn = open_turn(1);
    pushed(
        &mut turn,
        vec![response(
            10,
            vec![text_block(11, "plain").into()],
            ResponseEnding::Finished,
        )],
    );

    for target in [10, 11] {
        let error = refused(turn.push_entry(delivery(20, &[(21, target, "alpha")])));
        assert_eq!(error.code(), ErrorCode::WrongKindReference);
    }
}

#[test]
fn given_a_call_answered_in_an_earlier_delivery_when_answered_again_then_duplicate_tool_result() {
    let mut turn = turn_with_calls();
    pushed(&mut turn, vec![delivery(20, &[(21, 11, "alpha")])]);
    let before = turn.clone();

    let error = refused(turn.push_entry(delivery(30, &[(31, 12, "beta"), (32, 11, "alpha")])));

    assert_eq!(error.code(), ErrorCode::DuplicateToolResult);
    assert_eq!(error.object_id(), Some(bid(11).as_uuid()));
    assert_eq!(error.location(), Some("results[1]"));
    assert_eq!(turn, before);
}

#[test]
fn given_a_result_naming_another_tool_when_pushed_then_tool_result_mismatch() {
    let mut turn = turn_with_calls();

    let error = refused(turn.push_entry(delivery(20, &[(21, 11, "beta")])));

    assert_eq!(error.code(), ErrorCode::ToolResultMismatch);
    assert_eq!(turn.state().outstanding_runtime_calls().len(), 2);
}

#[test]
fn given_a_valid_and_an_invalid_result_in_one_delivery_when_pushed_then_none_is_applied() {
    let mut turn = turn_with_calls();
    let before = turn.clone();

    let error = refused(turn.push_entry(delivery(20, &[(21, 11, "alpha"), (22, 12, "alpha")])));

    assert_eq!(error.code(), ErrorCode::ToolResultMismatch);
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

#[test]
fn given_a_mounting_when_unmounted_then_the_reference_resolves_to_the_earlier_entry() {
    let mut turn = open_turn(1);

    pushed(&mut turn, vec![mount(10, "alpha"), unmount(11, 10)]);

    assert_eq!(turn.entries().len(), 2);
}

#[test]
fn given_an_unknown_or_wrong_kind_mounting_when_unmounted_then_refused() {
    let mut turn = open_turn(1);
    pushed(&mut turn, vec![calling(10, &[(11, "alpha")])]);

    let unknown = refused(turn.push_entry(unmount(20, 99)));
    let wrong = refused(turn.push_entry(unmount(21, 10)));

    assert_eq!(unknown.code(), ErrorCode::UnknownReference);
    assert_eq!(unknown.location(), Some("mounting"));
    assert_eq!(wrong.code(), ErrorCode::WrongKindReference);
}

#[test]
fn given_a_mounting_from_another_turn_when_unmounted_then_unknown_reference() {
    let mut other = open_turn(2);
    pushed(&mut other, vec![mount(10, "alpha")]);
    let mut turn = open_turn(1);

    let error = refused(turn.push_entry(unmount(20, 10)));

    assert_eq!(error.code(), ErrorCode::UnknownReference);
}

#[test]
fn given_a_withdrawn_mounting_when_unmounted_again_then_refused() {
    let mut turn = open_turn(1);
    pushed(&mut turn, vec![mount(10, "alpha"), unmount(11, 10)]);

    let error = refused(turn.push_entry(unmount(12, 10)));

    assert_eq!(error.code(), ErrorCode::InvalidContent);
    assert_eq!(error.location(), Some("mounting"));
}

#[test]
fn given_a_name_already_mounted_when_mounted_again_then_both_mountings_stand_independently() {
    let mut turn = open_turn(1);
    pushed(&mut turn, vec![mount(10, "alpha"), mount(11, "alpha")]);

    pushed(&mut turn, vec![unmount(12, 10)]);

    assert_eq!(turn.entries().len(), 3);
    assert_eq!(
        refused(turn.push_entry(unmount(13, 10))).code(),
        ErrorCode::InvalidContent
    );
    turn.push_entry(unmount(14, 11)).unwrap();
}

#[test]
fn given_a_withdrawn_name_when_mounted_again_then_the_earlier_mounting_is_untouched() {
    let mut turn = open_turn(1);
    pushed(&mut turn, vec![mount(10, "alpha"), unmount(11, 10)]);

    pushed(&mut turn, vec![mount(12, "alpha"), unmount(13, 12)]);

    assert_eq!(turn.entries().len(), 4);
    assert_eq!(
        refused(turn.push_entry(unmount(14, 10))).code(),
        ErrorCode::InvalidContent
    );
}

#[test]
fn given_an_unmount_when_a_pending_result_arrives_then_it_is_still_accepted() {
    let mut turn = open_turn(1);
    pushed(
        &mut turn,
        vec![
            mount(5, "alpha"),
            calling(10, &[(11, "alpha")]),
            unmount(12, 5),
        ],
    );

    assert_eq!(turn.state().outstanding_runtime_calls(), [bid(11)]);
    pushed(
        &mut turn,
        vec![delivery(20, &[(21, 11, "alpha")]), finished(30)],
    );

    assert!(turn.state().outstanding_runtime_calls().is_empty());
}

#[test]
fn given_a_mounting_in_a_closed_turn_when_a_later_turn_unmounts_it_then_the_conversation_refuses() {
    let mut conversation = br_llm_messages::Conversation::new(common::cvid(1000));
    conversation.push_turn(open_turn(1).into()).unwrap();
    conversation
        .push_entry(&common::turns::tid(1), mount(10, "alpha"))
        .unwrap();
    conversation
        .push_entry(
            &common::turns::tid(1),
            common::turns::close(11, br_llm_messages::ClosureReason::completed()),
        )
        .unwrap();
    conversation.push_turn(open_turn(2).into()).unwrap();
    let before = conversation.clone();

    let error = refused(conversation.push_entry(&common::turns::tid(2), unmount(20, 10)));

    assert_eq!(error.code(), ErrorCode::UnknownReference);
    assert_eq!(error.object_id(), Some(eid(10).as_uuid()));
    assert_eq!(conversation, before);
}

#[test]
fn given_a_later_turn_that_restates_the_offering_when_it_unmounts_then_the_new_mounting_is_withdrawn()
 {
    let mut conversation = br_llm_messages::Conversation::new(common::cvid(1000));
    conversation.push_turn(open_turn(1).into()).unwrap();
    conversation
        .push_entry(&common::turns::tid(1), mount(10, "alpha"))
        .unwrap();
    conversation
        .push_entry(
            &common::turns::tid(1),
            common::turns::close(11, br_llm_messages::ClosureReason::completed()),
        )
        .unwrap();
    conversation.push_turn(open_turn(2).into()).unwrap();

    conversation
        .push_entry(&common::turns::tid(2), mount(20, "alpha"))
        .unwrap();
    conversation
        .push_entry(&common::turns::tid(2), unmount(21, 20))
        .unwrap();

    assert_eq!(conversation.turns()[1].as_ai().unwrap().entries().len(), 2);
}
