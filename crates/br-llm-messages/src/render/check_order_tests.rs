//! The first fault in message order, then in block order, is the one
//! reported.

use super::test_helpers::*;
use super::*;

#[test]
fn given_unanswered_call_before_a_twice_answered_one_when_checked_then_the_earlier_fault_wins() {
    let list = vec![
        user(vec![user_text("q")]),
        assistant(vec![call("a"), call("b")]),
        user(vec![answer("a"), answer("a")]),
    ];
    assert_eq!(
        check_wire_list(&list),
        Err(MessageError::WireToolCallUnanswered {
            message: 1,
            block: 1,
            id: id("b"),
        })
    );
}

#[test]
fn given_a_second_answer_before_an_unmatched_result_when_checked_then_block_order_wins() {
    let list = vec![
        user(vec![user_text("q")]),
        assistant(vec![call("a")]),
        user(vec![answer("a"), answer("a"), answer("z")]),
    ];
    assert_eq!(
        check_wire_list(&list),
        Err(MessageError::WireToolCallAnsweredTwice {
            message: 2,
            block: 1,
            id: id("a"),
        })
    );
}
