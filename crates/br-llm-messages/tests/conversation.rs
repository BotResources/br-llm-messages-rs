mod common;

use br_llm_messages::{AITurn, ClosureReason, Conversation, ErrorCode, Turn, TurnId};
use common::turns::{calling, close, delivery, finished, human, input_turn, open_turn, tid};
use common::{aid, author, cvid, eid, refused};

fn empty() -> Conversation {
    Conversation::new(cvid(1))
}

#[test]
fn given_consecutive_input_turns_when_pushed_then_no_alternation_is_demanded() {
    let mut conversation = empty();

    conversation.push_turn(input_turn(10, 11).into()).unwrap();
    conversation.push_turn(input_turn(20, 21).into()).unwrap();
    conversation.push_turn(input_turn(30, 31).into()).unwrap();

    assert_eq!(conversation.turns().len(), 3);
}

#[test]
fn given_several_open_ai_turns_when_entries_are_pushed_by_turn_id_then_each_gets_its_own() {
    let mut conversation = empty();
    let first = AITurn::open(tid(10), aid(1), author("one"));
    let second = AITurn::open(tid(20), aid(2), author("two"));
    conversation.push_turn(first.into()).unwrap();
    conversation.push_turn(second.into()).unwrap();

    conversation.push_entry(&tid(20), human(21)).unwrap();
    conversation.push_entry(&tid(10), human(11)).unwrap();
    conversation.push_entry(&tid(20), finished(22)).unwrap();

    let sizes: Vec<_> = conversation
        .turns()
        .iter()
        .map(|turn| turn.as_ai().unwrap().entries().len())
        .collect();
    assert_eq!(sizes, [1, 2]);
}

#[test]
fn given_an_ai_turn_between_input_turns_when_pushed_then_the_order_is_recorded_as_given() {
    let mut conversation = empty();

    conversation.push_turn(open_turn(10).into()).unwrap();
    conversation.push_turn(input_turn(20, 21).into()).unwrap();
    conversation.push_turn(open_turn(30).into()).unwrap();

    let ids: Vec<TurnId> = conversation.turns().iter().map(Turn::id).collect();
    assert_eq!(ids, [tid(10), tid(20), tid(30)]);
}

#[test]
fn given_an_unknown_turn_when_an_entry_is_pushed_then_unknown_reference() {
    let mut conversation = empty();

    let error = refused(conversation.push_entry(&tid(99), human(11)));

    assert_eq!(error.code(), ErrorCode::UnknownReference);
    assert_eq!(error.object_id(), Some(tid(99).as_uuid()));
}

#[test]
fn given_an_input_turn_target_when_an_incoming_entry_is_pushed_then_it_is_appended_and_indexed() {
    let mut conversation = empty();
    conversation.push_turn(input_turn(10, 11).into()).unwrap();

    conversation.push_entry(&tid(10), human(12)).unwrap();

    let turn = conversation.turns()[0].as_input().unwrap();
    assert_eq!(turn.entries().len(), 2);
    assert_eq!(
        refused(conversation.push_entry(&tid(20), human(12))).code(),
        ErrorCode::UnknownReference
    );
    conversation.push_turn(open_turn(20).into()).unwrap();
    let before = conversation.clone();
    assert_eq!(
        refused(conversation.push_entry(&tid(20), human(12))).code(),
        ErrorCode::ReusedIdentity
    );
    assert_eq!(conversation, before);
}

#[test]
fn given_an_input_turn_target_when_a_non_incoming_entry_is_pushed_then_disallowed_kind_and_nothing_changes()
 {
    let mut conversation = empty();
    conversation.push_turn(input_turn(10, 11).into()).unwrap();
    let before = conversation.clone();

    let refusals = [
        finished(12),
        delivery(13, &[(14, 15, "t")]),
        close(16, ClosureReason::completed()),
    ];

    for entry in refusals {
        let error = refused(conversation.push_entry(&tid(10), entry.clone()));
        assert_eq!(error.code(), ErrorCode::DisallowedKind);
        assert_eq!(error.object_id(), Some(entry.id().as_uuid()));
    }
    assert_eq!(conversation, before);
}

#[test]
fn given_an_input_turn_entry_reusing_a_whole_conversation_identity_when_pushed_then_reused_identity_and_nothing_changes()
 {
    let mut conversation = empty();
    conversation.push_turn(input_turn(10, 11).into()).unwrap();
    conversation.push_turn(open_turn(20).into()).unwrap();
    conversation.push_entry(&tid(20), human(21)).unwrap();
    let before = conversation.clone();

    for reused in [11, 21, 10, 20] {
        let error = refused(conversation.push_entry(&tid(10), human(reused)));
        assert_eq!(error.code(), ErrorCode::ReusedIdentity, "{reused}");
        assert_eq!(error.location(), Some("entries[1]"), "{reused}");
        assert_eq!(conversation, before);
    }
}

#[test]
fn given_a_non_turn_id_as_target_when_an_entry_is_pushed_then_unknown_reference() {
    let mut conversation = empty();
    conversation.push_turn(open_turn(10).into()).unwrap();
    conversation.push_entry(&tid(10), human(11)).unwrap();
    let entry_as_turn = TurnId::from_uuid(eid(11).as_uuid()).unwrap();
    let conversation_as_turn = TurnId::from_uuid(cvid(1).as_uuid()).unwrap();

    for target in [entry_as_turn, conversation_as_turn] {
        let error = refused(conversation.push_entry(&target, human(12)));
        assert_eq!(error.code(), ErrorCode::UnknownReference);
    }
}

#[test]
fn given_a_refused_turn_when_the_conversation_is_compared_then_it_equals_its_pre_push_clone() {
    let mut conversation = empty();
    conversation.push_turn(input_turn(10, 11).into()).unwrap();
    let before = conversation.clone();

    let clash = refused(conversation.push_turn(input_turn(20, 11).into()));
    let turn_clash = refused(conversation.push_turn(input_turn(10, 21).into()));

    assert_eq!(clash.code(), ErrorCode::ReusedIdentity);
    assert_eq!(turn_clash.code(), ErrorCode::ReusedIdentity);
    assert_eq!(conversation, before);
}

#[test]
fn given_a_refused_turn_when_its_free_ids_are_reused_afterwards_then_they_are_still_free() {
    let mut conversation = empty();
    conversation.push_turn(input_turn(10, 11).into()).unwrap();

    refused(conversation.push_turn(input_turn(20, 11).into()));

    conversation.push_turn(input_turn(20, 21).into()).unwrap();
    assert_eq!(conversation.turns().len(), 2);
}

#[test]
fn given_refused_entries_when_pushed_then_history_and_index_are_unchanged() {
    let mut conversation = empty();
    conversation.push_turn(open_turn(10).into()).unwrap();
    conversation
        .push_entry(&tid(10), calling(11, &[(12, "t")]))
        .unwrap();
    let before = conversation.clone();

    let reasons = [
        refused(conversation.push_entry(&tid(10), delivery(11, &[(21, 12, "t")]))).code(),
        refused(conversation.push_entry(&tid(10), delivery(20, &[(21, 99, "t")]))).code(),
    ];

    assert_eq!(
        reasons,
        [ErrorCode::ReusedIdentity, ErrorCode::UnknownReference]
    );
    assert_eq!(conversation, before);
    conversation
        .push_entry(&tid(10), delivery(20, &[(21, 12, "t")]))
        .unwrap();
    conversation.push_entry(&tid(10), human(30)).unwrap();
}

#[test]
fn given_an_entry_clashing_with_another_turn_when_pushed_then_the_ai_turn_is_untouched() {
    let mut conversation = empty();
    conversation.push_turn(input_turn(10, 11).into()).unwrap();
    conversation.push_turn(open_turn(20).into()).unwrap();
    let before = conversation.clone();

    let error = refused(conversation.push_entry(&tid(20), human(11)));

    assert_eq!(error.code(), ErrorCode::ReusedIdentity);
    assert_eq!(conversation, before);
}

#[test]
fn given_a_closed_turn_in_a_conversation_when_a_late_result_and_an_extended_input_turn_are_saved_then_json_reload_gives_the_same_conversation()
 {
    let mut conversation = empty();
    conversation.push_turn(input_turn(10, 11).into()).unwrap();
    conversation.push_turn(open_turn(20).into()).unwrap();
    conversation
        .push_entry(&tid(20), calling(21, &[(22, "t")]))
        .unwrap();
    conversation
        .push_entry(&tid(20), close(23, ClosureReason::interrupted()))
        .unwrap();
    conversation.push_entry(&tid(10), human(30)).unwrap();

    conversation
        .push_entry(&tid(20), delivery(40, &[(41, 22, "t")]))
        .unwrap();

    let before = conversation.clone();
    assert_eq!(
        refused(conversation.push_entry(&tid(20), delivery(42, &[(43, 22, "t")]))).code(),
        ErrorCode::ClosedTurn
    );
    assert_eq!(conversation, before);
    let reloaded = Conversation::from_json(&conversation.to_json().unwrap()).unwrap();
    assert_eq!(reloaded, conversation);
    assert_eq!(
        refused(reloaded.clone().push_entry(&tid(10), human(40))).code(),
        ErrorCode::ReusedIdentity
    );
}
