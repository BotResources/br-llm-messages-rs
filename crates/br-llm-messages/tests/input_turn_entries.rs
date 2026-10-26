mod common;
use common::Raw as _;

use br_llm_messages::{
    AgentMail, Conversation, ErrorCode, HumanMessage, IncomingEntry, InputTurn, RecordSet,
    RequestAccepted, Turn, TurnRecord,
};
use common::turns::tid;
use common::{aid, author, bid, cvid, eid, plain_text, refused, rid};
use serde_json::json;

fn message(entry: u64, block: u64, text: &str) -> IncomingEntry {
    HumanMessage::new(eid(entry), common::hid(1), vec![plain_text(block, text)])
        .unwrap()
        .into()
}

fn mail(entry: u64, block: u64) -> IncomingEntry {
    AgentMail::new(
        eid(entry),
        aid(7),
        author("Codex GPT-6.1-Sol high"),
        cvid(70),
        vec![plain_text(block, "mail")],
    )
    .unwrap()
    .into()
}

fn notice(entry: u64) -> IncomingEntry {
    RequestAccepted::new(eid(entry), rid(1)).into()
}

fn mixed_turn() -> InputTurn {
    InputTurn::new(
        tid(1),
        vec![
            message(2, 3, "first"),
            mail(4, 5),
            notice(6),
            message(7, 8, "last"),
        ],
    )
    .unwrap()
}

#[test]
fn given_several_entries_of_mixed_kinds_when_an_input_turn_is_built_then_they_are_kept_in_order() {
    let turn = mixed_turn();

    let ids: Vec<_> = turn.entries().iter().map(IncomingEntry::id).collect();
    assert_eq!(ids, [eid(2), eid(4), eid(6), eid(7)]);
}

#[test]
fn given_a_multi_entry_input_turn_when_serialized_and_reloaded_then_nothing_is_lost() {
    let turn = mixed_turn();

    let json = turn.to_json().unwrap();
    let reloaded = InputTurn::from_json(&json).unwrap();
    let value = turn.to_value().unwrap().raw();

    assert_eq!(reloaded, turn);
    assert_eq!(value["entries"].as_array().map(Vec::len), Some(4));
    assert!(value.get("entry").is_none());
    assert_eq!(Turn::from_json(&json).unwrap(), Turn::from(turn));
}

#[test]
fn given_no_entry_when_an_input_turn_is_built_then_invalid_record_structure() {
    let error = refused(InputTurn::new(tid(1), Vec::new()));

    assert_eq!(error.code(), ErrorCode::InvalidRecordStructure);
    assert_eq!(error.location(), Some("entries"));
}

#[test]
fn given_an_empty_entry_list_in_json_when_loaded_then_invalid_record_structure() {
    let json = json!({"type": "input_turn", "id": tid(1).to_string(), "entries": []});

    let error = refused(Turn::from_value(common::oj(json)));

    assert_eq!(error.code(), ErrorCode::InvalidRecordStructure);
    assert_eq!(error.location(), Some("entries"));
}

#[test]
fn given_a_1x_single_entry_input_turn_when_loaded_then_it_is_refused() {
    let mut json = mixed_turn().to_value().unwrap().raw();
    let first = json["entries"][0].clone();
    json.as_object_mut().unwrap().remove("entries");
    json["entry"] = first;

    let error = refused(Turn::from_value(common::oj(json)));

    assert_eq!(error.code(), ErrorCode::InvalidContent);
}

#[test]
fn given_a_1x_conversation_document_when_loaded_then_schema_mismatch() {
    let mut document = Conversation::new(cvid(900)).to_value().unwrap().raw();
    document["schema"] = json!("br-llm-messages/4");

    let error = refused(Conversation::from_value(common::oj(document)));

    assert_eq!(error.code(), ErrorCode::SchemaMismatch);
    assert_eq!(error.location(), Some("schema"));
}

#[test]
fn given_the_same_entry_id_twice_in_one_input_turn_when_built_then_reused_identity_at_the_second() {
    let error = refused(InputTurn::new(
        tid(1),
        vec![message(2, 3, "a"), message(2, 5, "b")],
    ));

    assert_eq!(error.code(), ErrorCode::ReusedIdentity);
    assert_eq!(error.object_id(), Some(eid(2).as_uuid()));
    assert_eq!(error.location(), Some("entries[1]"));
}

#[test]
fn given_a_block_id_shared_by_two_entries_when_built_then_reused_identity_at_the_second() {
    let error = refused(InputTurn::new(
        tid(1),
        vec![message(2, 3, "a"), message(4, 3, "b")],
    ));

    assert_eq!(error.code(), ErrorCode::ReusedIdentity);
    assert_eq!(error.object_id(), Some(bid(3).as_uuid()));
    assert_eq!(error.location(), Some("entries[1]"));
}

#[test]
fn given_the_turn_id_reused_by_a_later_entry_when_built_then_reused_identity_at_that_entry() {
    let error = refused(InputTurn::new(
        tid(1),
        vec![message(2, 3, "a"), message(1, 5, "b")],
    ));

    assert_eq!(error.code(), ErrorCode::ReusedIdentity);
    assert_eq!(error.location(), Some("entries[1]"));
}

#[test]
fn given_an_entry_id_already_used_by_another_turn_when_pushed_then_reused_identity_at_that_entry() {
    let mut conversation = Conversation::new(cvid(900));
    conversation
        .push_turn(
            InputTurn::new(tid(10), vec![message(2, 3, "a")])
                .unwrap()
                .into(),
        )
        .unwrap();
    let second = InputTurn::new(tid(11), vec![message(20, 21, "b"), message(2, 23, "c")]).unwrap();

    let error = refused(conversation.push_turn(second.into()));

    assert_eq!(error.code(), ErrorCode::ReusedIdentity);
    assert_eq!(error.object_id(), Some(eid(2).as_uuid()));
    assert_eq!(error.location(), Some("entries[1]"));
    assert_eq!(conversation.turns().len(), 1);
}

#[test]
fn given_a_multi_entry_input_turn_when_looked_up_then_every_entry_and_block_resolves_to_its_own_object()
 {
    let mut conversation = Conversation::new(cvid(900));
    conversation.push_turn(mixed_turn().into()).unwrap();

    for (entry, kind) in [
        (2, "human_message"),
        (4, "agent_mail"),
        (6, "request_accepted"),
        (7, "human_message"),
    ] {
        assert_eq!(
            conversation.entry(&eid(entry)).unwrap().kind().as_str(),
            kind
        );
    }
    let last = conversation.block(&bid(8)).unwrap();
    let first = conversation.block(&bid(3)).unwrap();
    assert_ne!(format!("{last:?}"), format!("{first:?}"));
    assert!(format!("{last:?}").contains("last"));
    assert!(format!("{first:?}").contains("first"));
}

#[test]
fn given_a_conversation_with_a_multi_entry_input_turn_when_decomposed_and_rebuilt_then_it_is_equal()
{
    let mut conversation = Conversation::new(cvid(900));
    conversation.push_turn(mixed_turn().into()).unwrap();
    conversation
        .push_turn(
            InputTurn::new(tid(20), vec![message(21, 22, "alone")])
                .unwrap()
                .into(),
        )
        .unwrap();

    let set = RecordSet::from_conversation(&conversation).unwrap();
    let reloaded = RecordSet::from_json(&set.to_json().unwrap()).unwrap();

    assert_eq!(set.turns().len(), 2);
    assert_eq!(set.entries().len(), 5);
    assert_eq!(reloaded.into_conversation(cvid(900)).unwrap(), conversation);
}

#[test]
fn given_entry_rows_of_one_input_turn_when_rebuilt_then_the_row_order_is_the_entry_order() {
    let set = RecordSet::from_turn(&mixed_turn().into()).unwrap();
    let mut rows = set.entries().to_vec();
    rows.swap(0, 3);
    let reordered = RecordSet::new()
        .with_turns(set.turns().to_vec())
        .with_entries(rows)
        .with_blocks(set.blocks().to_vec());

    let turns = reordered.into_turns().unwrap();

    let Turn::InputTurn(turn) = &turns[0] else {
        panic!("input turn expected");
    };
    let ids: Vec<_> = turn.entries().iter().map(IncomingEntry::id).collect();
    assert_eq!(ids, [eid(7), eid(4), eid(6), eid(2)]);
}

#[test]
fn given_an_input_turn_row_without_entry_rows_when_rebuilt_then_invalid_record_structure() {
    let set = RecordSet::new().with_turns(vec![TurnRecord::new(
        tid(1),
        br_llm_messages::TurnKind::InputTurn,
    )]);

    let error = refused(set.into_turns());

    assert_eq!(error.code(), ErrorCode::InvalidRecordStructure);
    assert_eq!(error.object_id(), Some(tid(1).as_uuid()));
    assert_eq!(error.location(), Some("turns[0].entries"));
}

#[test]
fn given_a_non_incoming_row_among_input_turn_rows_when_rebuilt_then_disallowed_kind_at_its_index() {
    let set = RecordSet::from_turn(&mixed_turn().into()).unwrap();
    let mut document = set.to_value().unwrap().raw();
    let row = &mut document["entries"][2];
    row["kind"] = json!("turn_closure");
    row["payload"] = json!({"reason": {"type": "completed"}});
    let set = RecordSet::from_value(common::oj(document)).unwrap();

    let error = refused(set.into_turns());

    assert_eq!(error.code(), ErrorCode::DisallowedKind);
    assert_eq!(error.location(), Some("turns[0].entries[2]"));
}

#[test]
fn given_a_multi_entry_input_turn_when_projected_then_it_is_left_unchanged_with_no_new_identity() {
    use br_llm_messages::Perspective;

    let turn = mixed_turn();
    let mut conversation = Conversation::new(cvid(900));
    conversation.push_turn(turn.clone().into()).unwrap();

    let projection = Perspective::of(aid(1)).project(&conversation);

    assert_eq!(projection.turns().len(), 1);
    assert_eq!(projection.turns()[0], Turn::from(turn));
}
