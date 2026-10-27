mod common;
use common::Raw as _;

use br_llm_messages::{ClosureReason, Conversation, ErrorCode, RecordSet};
use common::turns::{
    calling, close, delivery, finished, human, input_turn, open_turn, response, tid,
};
use common::{cvid, oj, refused};

fn grown() -> Conversation {
    let mut conversation = Conversation::new(cvid(1));
    conversation.push_turn(input_turn(10, 11).into()).unwrap();
    conversation.push_turn(open_turn(20).into()).unwrap();
    for entry in [
        calling(21, &[(22, "a"), (23, "b")]),
        finished(24),
        human(25),
        response(
            26,
            vec![],
            br_llm_messages::ResponseEnding::provider_continuation(),
        ),
        delivery(27, &[(28, 22, "a")]),
        close(29, ClosureReason::interrupted()),
        delivery(30, &[(31, 23, "b")]),
    ] {
        conversation.push_entry(&tid(20), entry).unwrap();
    }
    conversation.push_entry(&tid(10), human(40)).unwrap();
    conversation
}

#[test]
fn given_free_order_entries_late_results_and_a_grown_input_turn_when_decomposed_and_rebuilt_then_the_conversation_is_equal()
 {
    let conversation = grown();

    let set = RecordSet::from_conversation(&conversation).unwrap();
    let rebuilt = set.clone().into_conversation(cvid(1)).unwrap();
    let reloaded = RecordSet::from_json(&set.to_json().unwrap())
        .unwrap()
        .into_conversation(cvid(1))
        .unwrap();

    assert_eq!(rebuilt, conversation);
    assert_eq!(reloaded, conversation);
    let before = conversation.turns()[1].as_ai().unwrap().state();
    let after = rebuilt.turns()[1].as_ai().unwrap().state();
    assert_eq!(before, after);
    assert!(after.outstanding_runtime_calls().is_empty());
    assert!(after.is_closed());
}

#[test]
fn given_an_incoming_row_after_the_closure_row_when_rebuilt_then_closed_turn_at_its_position() {
    let mut valid = Conversation::new(cvid(1));
    valid.push_turn(open_turn(20).into()).unwrap();
    for entry in [human(21), human(23), close(22, ClosureReason::cancelled())] {
        valid.push_entry(&tid(20), entry).unwrap();
    }
    let set = RecordSet::from_conversation(&valid).unwrap();
    let mut document = set.to_value().unwrap().raw();
    let rows = document["entries"].as_array_mut().unwrap();
    let closure_at = rows
        .iter()
        .position(|row| row["kind"] == "turn_closure")
        .unwrap();
    let human_at = rows
        .iter()
        .position(|row| row["id"] == human(23).id().as_uuid().hyphenated().to_string())
        .unwrap();
    rows.swap(closure_at, human_at);

    let error = refused(
        RecordSet::from_value(oj(document))
            .and_then(|reordered| reordered.into_conversation(cvid(1))),
    );

    assert_eq!(error.code(), ErrorCode::ClosedTurn);
    assert_eq!(error.location(), Some("turns[0].entries[2]"));
    assert_eq!(error.object_id(), Some(human(23).id().as_uuid()));
}

#[test]
fn given_a_result_row_naming_no_outstanding_call_when_rebuilt_then_entry_out_of_order() {
    let mut valid = Conversation::new(cvid(1));
    valid.push_turn(open_turn(20).into()).unwrap();
    for entry in [calling(21, &[(22, "a")]), delivery(23, &[(24, 22, "a")])] {
        valid.push_entry(&tid(20), entry).unwrap();
    }
    let mut document = RecordSet::from_conversation(&valid)
        .unwrap()
        .to_value()
        .unwrap()
        .raw();
    document["entries"].as_array_mut().unwrap().swap(0, 1);

    let error = refused(
        RecordSet::from_value(oj(document))
            .and_then(|reordered| reordered.into_conversation(cvid(1))),
    );

    assert_eq!(error.code(), ErrorCode::EntryOutOfOrder);
    assert_eq!(error.location(), Some("turns[0].entries[0]"));
}
