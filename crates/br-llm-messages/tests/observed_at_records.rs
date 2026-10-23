mod common;
use common::Raw as _;

use br_llm_messages::{
    BlockRecord, ContentBlockKind, EntryRecord, ErrorCode, RecordSet, RepresentationState,
    TurnEntryKind, TurnKind, TurnRecord,
};
use common::conversations::rich;
use common::records::{rich_set, row, try_edit};
use common::turns::tid;
use common::{at, bid, eid, oj, v7};
use serde_json::{Value, json};

fn column_value(document: &Value, collection: &str, id: u64) -> Value {
    let wanted = v7(id).hyphenated().to_string();
    document[collection]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["id"] == wanted)
        .unwrap()["observed_at"]
        .clone()
}

#[test]
fn given_a_stamped_conversation_when_decomposed_then_all_three_row_kinds_carry_the_time_as_a_column()
 {
    let set = rich_set();

    let input = &set.turns()[0];
    let ai = &set.turns()[1];
    assert_eq!(input.observed_at(), Some(&at(3)));
    assert_eq!(ai.observed_at(), Some(&at(19)));
    let entry = |id: u64| {
        set.entries()
            .iter()
            .find(|row| row.id() == eid(id))
            .unwrap()
    };
    assert_eq!(entry(2).observed_at(), Some(&at(2)));
    assert_eq!(entry(12).observed_at(), Some(&at(10)));
    assert_eq!(entry(30).observed_at(), Some(&at(18)));
    let block = |id: u64| set.blocks().iter().find(|row| row.id() == bid(id)).unwrap();
    assert_eq!(block(3).observed_at(), Some(&at(1)));
    assert_eq!(block(15).observed_at(), Some(&at(6)));
    assert_eq!(block(22).observed_at(), Some(&at(11)));
    assert_eq!(block(24).observed_at(), Some(&at(12)));
    assert_eq!(block(24).parent(), Some(bid(22)));
}

#[test]
fn given_decomposed_rows_when_their_payloads_are_read_then_the_time_lives_only_in_the_column() {
    let document = rich_set().to_value().unwrap().raw();

    for collection in ["entries", "blocks"] {
        for row in document[collection].as_array().unwrap() {
            assert!(row["payload"].get("observed_at").is_none(), "{row}");
        }
    }
    assert_eq!(column_value(&document, "turns", 1), json!(at(3).as_str()));
    assert_eq!(column_value(&document, "entries", 2), json!(at(2).as_str()));
    assert_eq!(
        column_value(&document, "blocks", 24),
        json!(at(12).as_str())
    );
}

#[test]
fn given_rows_with_times_when_reconstructed_then_the_conversation_is_equal_including_every_time() {
    let set = rich_set();

    let rebuilt = set.clone().into_conversation(rich().id()).unwrap();
    let again = RecordSet::from_json(&set.to_json().unwrap()).unwrap();

    assert_eq!(rebuilt, rich());
    assert_eq!(again, set);
    assert_eq!(
        again
            .into_conversation(rich().id())
            .unwrap()
            .to_json()
            .unwrap(),
        rich().to_json().unwrap()
    );
}

#[test]
fn given_a_row_without_a_time_when_reconstructed_then_the_object_has_none() {
    let mut document = rich_set().to_value().unwrap().raw();
    row(&mut document, "entries", v7(2))
        .as_object_mut()
        .unwrap()
        .remove("observed_at");
    row(&mut document, "blocks", v7(24))
        .as_object_mut()
        .unwrap()
        .remove("observed_at");
    row(&mut document, "turns", v7(1))
        .as_object_mut()
        .unwrap()
        .remove("observed_at");

    let rebuilt = RecordSet::from_value(oj(document))
        .unwrap()
        .into_conversation(rich().id())
        .unwrap();

    let first = rebuilt.turns()[0].as_input().unwrap();
    assert_eq!(first.observed_at(), None);
    assert_eq!(first.entry().observed_at(), None);
    assert_ne!(rebuilt, rich());
}

#[test]
fn given_a_bad_time_in_a_row_column_when_loaded_then_invalid_content_for_each_row_kind() {
    for (collection, id) in [("turns", 1), ("entries", 2), ("blocks", 24)] {
        for bad in [
            json!("2026-10-23T08:00:00+01:00"),
            json!("2026-10-23t08:00:00Z"),
            json!("2026-02-29T08:00:00Z"),
            json!(5),
        ] {
            let error = try_edit(&rich_set(), |d| {
                row(d, collection, v7(id))["observed_at"] = bad.clone()
            })
            .unwrap_err();

            assert_eq!(
                error.code(),
                ErrorCode::InvalidContent,
                "{collection} {bad}"
            );
        }
    }
}

#[test]
fn given_a_time_inside_a_payload_when_loaded_then_it_is_refused_as_a_second_copy() {
    let entry = try_edit(&rich_set(), |d| {
        row(d, "entries", v7(12))["payload"]["observed_at"] = json!(at(1).as_str());
    })
    .unwrap_err();
    let block = try_edit(&rich_set(), |d| {
        row(d, "blocks", v7(15))["payload"]["observed_at"] = json!(at(1).as_str());
    })
    .unwrap_err();

    assert_eq!(entry.code(), ErrorCode::InvalidContent);
    assert_eq!(block.code(), ErrorCode::InvalidContent);
}

#[test]
fn given_rows_built_in_code_when_given_times_then_the_getters_and_local_checks_accept_them() {
    let turn = TurnRecord::new(tid(1), TurnKind::InputTurn, 0).with_observed_at(at(1));
    let entry = EntryRecord::new(
        eid(2),
        tid(1),
        TurnEntryKind::TaskCompleted,
        0,
        RepresentationState::Complete,
        oj(json!({"task": common::kid(1).to_string()})),
    )
    .with_observed_at(at(2));
    let block = BlockRecord::new(
        bid(3),
        eid(2),
        ContentBlockKind::Image,
        0,
        RepresentationState::Complete,
        oj(json!({"source": {"type": "inline", "media_type": "image/png", "data": "QUJD"}})),
    )
    .with_observed_at(at(3));

    assert_eq!(turn.observed_at(), Some(&at(1)));
    assert_eq!(entry.observed_at(), Some(&at(2)));
    assert_eq!(block.observed_at(), Some(&at(3)));
    assert!(turn.check_local().is_ok());
    assert!(entry.check_local().is_ok());
    assert!(block.check_local().is_ok(), "{:?}", block.check_local());
}

#[test]
fn given_a_discarded_row_with_a_time_when_checked_then_it_is_disallowed() {
    let discarded = BlockRecord::new(
        bid(3),
        eid(2),
        ContentBlockKind::Text,
        0,
        RepresentationState::Discarded,
        oj(json!({})),
    );

    assert!(discarded.check_local().is_ok());
    let error = discarded.with_observed_at(at(3)).check_local().unwrap_err();
    assert_eq!(error.code(), ErrorCode::DisallowedKind);
    assert_eq!(error.location(), Some("observed_at"));
}
