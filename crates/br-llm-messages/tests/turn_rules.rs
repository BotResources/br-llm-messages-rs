mod common;

use br_llm_messages::{ClosureReason, ErrorCode, ResponseEnding};
use common::turns::{
    calling, close, delivery, finished, hosted_call, hosted_result, open_turn, pushed, response,
};
use common::{bid, call, refused, text_block};

#[test]
fn given_outstanding_calls_when_a_response_is_pushed_then_it_is_accepted_and_the_calls_stay_outstanding()
 {
    let mut turn = open_turn(1);
    pushed(&mut turn, vec![calling(10, &[(11, "a"), (12, "b")])]);

    turn.push_entry(finished(13)).unwrap();

    assert_eq!(turn.entries().len(), 2);
    assert_eq!(turn.state().outstanding_runtime_calls().len(), 2);
}

#[test]
fn given_results_over_several_deliveries_when_all_collected_then_no_call_stays_outstanding() {
    let mut turn = open_turn(1);
    pushed(
        &mut turn,
        vec![calling(10, &[(11, "a"), (12, "b"), (13, "c")])],
    );

    pushed(&mut turn, vec![delivery(20, &[(21, 11, "a")])]);
    turn.push_entry(finished(30)).unwrap();
    pushed(
        &mut turn,
        vec![delivery(22, &[(23, 13, "c"), (24, 12, "b")])],
    );
    turn.push_entry(finished(31)).unwrap();

    assert_eq!(turn.entries().len(), 5);
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
fn given_a_provider_call_already_answered_when_a_second_result_is_pushed_then_duplicate_result() {
    let mut turn = open_turn(1);
    pushed(
        &mut turn,
        vec![response(
            10,
            vec![hosted_call(11), hosted_result(12, 11)],
            ResponseEnding::provider_continuation(),
        )],
    );

    let error = refused(turn.push_entry(response(
        20,
        vec![hosted_result(21, 11)],
        ResponseEnding::finished(),
    )));

    assert_eq!(error.code(), ErrorCode::DuplicateResult);
    assert_eq!(error.object_id(), Some(bid(11).as_uuid()));
    assert_eq!(error.location(), Some("content[0].call"));
    assert_eq!(turn.entries().len(), 1);
}

#[test]
fn given_a_provider_call_answered_in_a_later_response_when_a_third_result_is_pushed_then_it_is_refused()
 {
    let mut turn = open_turn(1);
    pushed(
        &mut turn,
        vec![
            response(
                10,
                vec![hosted_call(11)],
                ResponseEnding::provider_continuation(),
            ),
            response(
                20,
                vec![hosted_result(21, 11)],
                ResponseEnding::provider_continuation(),
            ),
        ],
    );

    let error = refused(turn.push_entry(response(
        30,
        vec![hosted_result(31, 11)],
        ResponseEnding::finished(),
    )));

    assert_eq!(error.code(), ErrorCode::DuplicateResult);
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
fn given_a_provider_result_linking_a_wrong_kind_when_pushed_then_unknown_reference() {
    let mut turn = open_turn(1);
    pushed(
        &mut turn,
        vec![response(
            10,
            vec![call(11, "t").into(), text_block(12, "x").into()],
            ResponseEnding::tool_use(),
        )],
    );
    pushed(&mut turn, vec![delivery(20, &[(21, 11, "t")])]);

    for target in [11, 12] {
        let error = refused(turn.push_entry(response(
            30 + target,
            vec![hosted_result(40 + target, target)],
            ResponseEnding::finished(),
        )));
        assert_eq!(error.code(), ErrorCode::UnknownReference);
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
            close(14, ClosureReason::completed()),
        ],
    );

    assert!(turn.state().outstanding_runtime_calls().is_empty());
}

#[test]
fn given_an_earlier_response_with_a_provider_call_when_a_later_response_answers_it_twice_then_duplicate_result()
 {
    let mut turn = open_turn(1);
    pushed(
        &mut turn,
        vec![response(
            10,
            vec![hosted_call(11)],
            ResponseEnding::provider_continuation(),
        )],
    );

    let error = refused(turn.push_entry(response(
        20,
        vec![hosted_result(21, 11), hosted_result(22, 11)],
        ResponseEnding::finished(),
    )));

    assert_eq!(error.code(), ErrorCode::DuplicateResult);
    assert_eq!(error.object_id(), Some(bid(11).as_uuid()));
    assert_eq!(error.location(), Some("content[1].call"));
    assert_eq!(turn.entries().len(), 1);
}

#[test]
fn given_a_document_answering_a_provider_call_twice_in_one_response_when_loaded_then_duplicate_result()
 {
    let call = "ab000000-0000-7000-8000-00000000000b";
    let result = |id: &str, inner: &str| {
        format!(
            r#"{{"type":"provider_tool_result","id":"ab000000-0000-7000-8000-{id}","call":"{call}","content":[{{"type":"text","id":"ab000000-0000-7000-8000-{inner}","text":"found"}}]}}"#
        )
    };
    let document = format!(
        r#"{{"type":"ai_turn","id":"ab000000-0000-7000-8000-000000000001","agent":"ab000000-0000-7000-8000-000000000001","author":"Codex GPT-6.1-Sol high","entries":[{{"type":"model_response","id":"ab000000-0000-7000-8000-00000000000a","ending":{{"type":"provider_continuation"}},"content":[{{"type":"provider_tool_call","id":"{call}"}}]}},{{"type":"model_response","id":"ab000000-0000-7000-8000-000000000014","ending":{{"type":"finished"}},"content":[{},{}]}}]}}"#,
        result("000000000015", "000000000bc1"),
        result("000000000016", "000000000bc2"),
    );

    let error = refused(br_llm_messages::AITurn::from_json(&document));

    assert_eq!(error.code(), ErrorCode::DuplicateResult);
}
