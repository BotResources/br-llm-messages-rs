mod common;
use common::Raw as _;
mod snapshot_support;
mod stream_support;

use br_llm_messages::{
    DRAFT_SCHEMA, DeltaOp, DraftSnapshot, ErrorCode, Finalization, RepresentationState,
    ResponseDraft, Uuid,
};
use common::bid;
use serde_json::{Value, json};
use snapshot_support::{deltas, draft_after, script};
use stream_support::delta;

fn snapshot_at(count: usize) -> DraftSnapshot {
    draft_after(&deltas(&script()), count)
        .to_snapshot()
        .unwrap()
}

fn block_row(document: &mut Value, id: Uuid) -> &mut Value {
    let wanted = id.hyphenated().to_string();
    document["blocks"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|row| row["id"] == wanted)
        .expect("row present")
}

fn load(
    change: impl FnOnce(&mut Value),
    count: usize,
) -> Result<DraftSnapshot, br_llm_messages::Error> {
    let mut document = snapshot_at(count).to_value().unwrap().raw();
    change(&mut document);
    DraftSnapshot::from_value(common::oj(document))
}

#[test]
fn given_a_producing_text_row_when_finalized_then_the_same_row_becomes_complete_in_place() {
    let before = snapshot_at(26);
    let mut draft = draft_after(&deltas(&script()), 26);
    draft
        .apply(&delta(DeltaOp::block_finalized(
            bid(10),
            Finalization::accumulated(),
        )))
        .unwrap();

    let after = draft.to_snapshot().unwrap();

    let find = |snapshot: &DraftSnapshot| {
        snapshot
            .blocks()
            .iter()
            .find(|row| row.id() == bid(10))
            .unwrap()
            .clone()
    };
    let (open, done) = (find(&before), find(&after));
    assert_eq!(open.state(), RepresentationState::Producing);
    assert_eq!(done.state(), RepresentationState::Complete);
    assert_eq!(
        (
            open.id(),
            open.entry(),
            open.kind(),
            open.position(),
            open.parent()
        ),
        (
            done.id(),
            done.entry(),
            done.kind(),
            done.position(),
            done.parent()
        )
    );
}

#[test]
fn given_a_restored_complete_row_when_finalized_or_discarded_again_then_it_is_a_stream_phase_error()
{
    let snapshot = snapshot_at(28);
    let mut restored = ResponseDraft::from_snapshot(&snapshot).unwrap();

    let refinalize = restored
        .apply(&delta(DeltaOp::block_finalized(
            bid(10),
            Finalization::accumulated(),
        )))
        .unwrap_err();
    let discard = restored
        .apply(&delta(DeltaOp::block_discarded(bid(10))))
        .unwrap_err();
    let append = restored
        .apply(&delta(DeltaOp::text_appended(bid(10), "late")))
        .unwrap_err();

    for error in [refinalize, discard, append] {
        assert_eq!(error.code(), ErrorCode::StreamPhaseError);
        assert_eq!(error.object_id(), Some(bid(10).as_uuid()));
    }
}

#[test]
fn given_a_checkpoint_when_serialized_then_it_names_its_schema_and_round_trips() {
    let snapshot = snapshot_at(24);

    let json = snapshot.to_json().unwrap();
    let reloaded = DraftSnapshot::from_json(&json).unwrap();

    assert_eq!(DraftSnapshot::schema(), "br-llm-messages-draft/1");
    assert_eq!(snapshot.to_value().unwrap().raw()["schema"], DRAFT_SCHEMA);
    assert_eq!(reloaded, snapshot);
    assert_eq!(reloaded.to_json().unwrap(), json);
}

#[test]
fn given_another_schema_when_the_checkpoint_is_loaded_then_it_is_schema_mismatch() {
    for schema in [
        json!("br-llm-messages-draft/2"),
        json!("br-llm-messages-records/1"),
        json!(1),
    ] {
        let error = load(|document| document["schema"] = schema, 24).unwrap_err();
        assert_eq!(error.code(), ErrorCode::SchemaMismatch);
    }
    let missing = load(
        |document| {
            document.as_object_mut().unwrap().remove("schema");
        },
        24,
    )
    .unwrap_err();
    assert_eq!(missing.code(), ErrorCode::SchemaMismatch);
}

#[test]
fn given_a_complete_row_with_blank_text_when_loaded_then_completed_invariants_refuse_it() {
    let error = load(
        |document| block_row(document, common::v7(10))["payload"]["text"] = json!("  "),
        28,
    )
    .unwrap_err();

    assert_eq!(error.code(), ErrorCode::InvalidContent);
}

#[test]
fn given_a_partial_row_with_blank_text_when_loaded_then_draft_invariants_accept_it() {
    let loaded = load(
        |document| block_row(document, common::v7(10))["payload"]["text"] = json!("  "),
        24,
    )
    .unwrap();

    let restored = ResponseDraft::from_snapshot(&loaded).unwrap();
    assert_eq!(restored.view().slots()[3].text(), Some("  "));
}

#[test]
fn given_rows_that_break_snapshot_structure_when_loaded_then_the_rule_code_is_reported() {
    let foreign_entry = load(
        |document| block_row(document, common::v7(10))["entry"] = json!(common::v7(99).to_string()),
        24,
    )
    .unwrap_err();
    let duplicate_position = load(
        |document| block_row(document, common::v7(10))["position"] = json!(0),
        24,
    )
    .unwrap_err();
    let reused_id = load(
        |document| block_row(document, common::v7(10))["id"] = json!(common::v7(11).to_string()),
        24,
    )
    .unwrap_err();
    let unknown_field = load(|document| document["cursor"] = json!(7), 24).unwrap_err();

    assert_eq!(foreign_entry.code(), ErrorCode::UnknownReference);
    assert_eq!(duplicate_position.code(), ErrorCode::InvalidRecordStructure);
    assert_eq!(reused_id.code(), ErrorCode::ReusedIdentity);
    assert_eq!(unknown_field.code(), ErrorCode::InvalidContent);
}

#[test]
fn given_row_states_that_contradict_the_header_when_loaded_then_they_are_refused() {
    let interrupted_row = load(
        |document| block_row(document, common::v7(10))["state"] = json!("interrupted"),
        24,
    )
    .unwrap_err();
    let complete_header = load(
        |document| {
            document["entry"]["state"] = json!("complete");
            document["entry"]["payload"] = json!({"ending": {"type": "finished"}});
        },
        24,
    )
    .unwrap_err();

    assert_eq!(interrupted_row.code(), ErrorCode::DisallowedKind);
    assert_eq!(complete_header.code(), ErrorCode::StreamPhaseError);
}

#[test]
fn given_a_runtime_result_row_when_loaded_as_a_draft_then_it_is_disallowed() {
    let error = load(
        |document| {
            let row = block_row(document, common::v7(17));
            row["kind"] = json!("runtime_tool_result");
            row["call"] = json!(common::v7(12).to_string());
            row["payload"] = json!({"tool_name": "lookup", "is_error": false});
        },
        28,
    )
    .unwrap_err();

    assert_eq!(error.code(), ErrorCode::DisallowedKind);
}

#[test]
fn given_the_entry_position_when_chosen_then_it_is_carried_by_the_header_row() {
    let snapshot = snapshot_at(24).with_entry_position(6);

    assert_eq!(snapshot.entry().position(), 6);
    assert_eq!(snapshot.turn(), snapshot.entry().turn());
    assert!(ResponseDraft::from_snapshot(&snapshot).is_ok());
}

#[test]
fn given_a_producing_row_inside_an_interrupted_snapshot_when_loaded_then_it_is_refused() {
    let mut draft = draft_after(&deltas(&script()), 24);
    draft
        .apply(&delta(DeltaOp::response_interrupted(
            br_llm_messages::InterruptionReason::cancelled(),
        )))
        .unwrap();
    let mut document = draft.to_snapshot().unwrap().to_value().unwrap().raw();
    block_row(&mut document, common::v7(10))["state"] = json!("producing");

    let error = DraftSnapshot::from_value(common::oj(document)).unwrap_err();

    assert_eq!(error.code(), ErrorCode::DisallowedKind);
}

#[test]
fn given_an_entry_row_of_another_kind_when_loaded_as_a_draft_then_it_is_refused() {
    let error = load(
        |document| {
            let entry = &mut document["entry"];
            entry["kind"] = json!("turn_closure");
            entry["payload"] = json!({"reason": {"type": "completed"}});
            entry["state"] = json!("complete");
        },
        24,
    )
    .unwrap_err();

    assert_eq!(error.code(), ErrorCode::DisallowedKind);
}
