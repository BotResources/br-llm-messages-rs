mod common;
mod projection_support;

use br_llm_messages::{
    Conversation, EntryRecord, ErrorCode, Perspective, RecordSet, RepresentationState, Turn,
    TurnEntryKind,
};
use common::turns::tid;
use common::{aid, cvid, eid, oj, refused};
use projection_support::*;
use serde_json::{Value, json};

fn with_payload(
    rows: &RecordSet,
    entry: u64,
    state: RepresentationState,
    payload: Value,
) -> RecordSet {
    let entries = rows
        .entries()
        .iter()
        .map(|row| match row.id() == eid(entry) {
            true => EntryRecord::new(row.id(), row.turn(), row.kind(), state, oj(payload.clone())),
            false => row.clone(),
        })
        .collect();
    rows.clone().with_entries(entries)
}

fn foreign() -> Vec<Turn> {
    vec![ai(10, 2, vec![said(11, 12, "foreign answer")])]
}

#[test]
fn given_a_foreign_response_row_with_a_malformed_payload_when_projected_from_records_then_invalid_content()
 {
    let rows = rows_with(&[], &foreign(), 1);
    let cases = [
        (RepresentationState::Complete, json!({"ending": 42})),
        (RepresentationState::Complete, json!({"usage": "lol"})),
        (RepresentationState::Complete, json!({"x": [1, 2]})),
        (RepresentationState::Complete, json!("not an object")),
        (
            RepresentationState::Producing,
            json!({"ending": {"type": "finished"}}),
        ),
        (
            RepresentationState::Producing,
            json!({"interruption": {"type": "cancelled"}}),
        ),
    ];
    for (state, payload) in cases {
        let error = refused(
            with_payload(&rows, 11, state, payload.clone())
                .into_projected_thread(cvid(900), aid(1)),
        );

        assert_eq!(error.code(), ErrorCode::InvalidContent, "{payload}");
        assert_eq!(error.object_id(), Some(eid(11).as_uuid()), "{payload}");
        assert!(
            error
                .location()
                .is_some_and(|at| at.starts_with("entries[0].payload")),
            "{payload}"
        );
    }
}

#[test]
fn given_a_foreign_response_row_built_in_memory_without_payload_when_projected_then_it_is_accepted()
{
    let rows = with_payload(
        &rows_with(&[], &foreign(), 1),
        11,
        RepresentationState::Complete,
        json!({}),
    );
    let expected = Perspective::of(aid(1))
        .project_turns(cvid(900), &foreign())
        .unwrap();

    assert_eq!(
        rows.into_projected_thread(cvid(900), aid(1)).unwrap(),
        expected
    );
}

#[test]
fn given_a_complete_response_row_without_ending_when_serialized_or_loaded_from_json_then_invalid_content()
 {
    let rows = with_payload(
        &rows_with(&[], &foreign(), 1),
        11,
        RepresentationState::Complete,
        json!({}),
    );
    let row = json!({"id": eid(11), "turn": tid(10), "kind": "model_response", "state": "complete", "payload": {}});

    let set = refused(rows.to_json());
    let single = refused(serde_json::from_value::<EntryRecord>(row));

    assert_eq!(set.code(), ErrorCode::InvalidContent);
    assert_eq!(set.location(), Some("entries[0].payload.ending"));
    assert!(single.to_string().starts_with("invalid_content"));
}

#[test]
fn given_any_row_that_loads_from_json_when_checked_locally_then_it_passes() {
    let conversation = Conversation::from_parts(cvid(900), foreign()).unwrap();
    let rows = RecordSet::from_conversation(&conversation).unwrap();

    let reloaded = RecordSet::from_json(&rows.to_json().unwrap()).unwrap();

    assert_eq!(reloaded.check_local(), Ok(()));
}

#[test]
fn given_an_interrupted_foreign_row_with_any_payload_when_projected_then_it_is_ignored() {
    let rows = with_payload(
        &rows_with(&[], &foreign(), 1),
        11,
        RepresentationState::Interrupted,
        json!(7),
    );
    let ignored = rows
        .entries()
        .iter()
        .all(|row| row.kind() == TurnEntryKind::ModelResponse);

    let thread = rows.into_projected_thread(cvid(900), aid(1)).unwrap();

    assert!(ignored);
    assert!(thread.turns().is_empty());
}
