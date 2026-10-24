mod common;
mod snapshot_support;
mod stream_support;
use common::Raw as _;

use br_llm_messages::{
    DELTA_SCHEMA, DeltaOp, DeltaStream, ErrorCode, ResponseDelta, ResponseDraft,
};
use common::bid;
use serde_json::{Value, json};
use snapshot_support::{deltas, script};
use stream_support::delta;

fn stream() -> DeltaStream {
    DeltaStream::new(deltas(&script()))
}

fn document() -> Value {
    stream().to_value().unwrap().raw()
}

fn load(value: Value) -> br_llm_messages::Error {
    DeltaStream::from_value(common::oj(value)).unwrap_err()
}

#[test]
fn given_a_script_when_stored_as_a_stream_then_one_document_holds_the_schema_and_the_flat_deltas() {
    let original = stream();

    let text = original.to_json().unwrap();
    let reloaded = DeltaStream::from_json(&text).unwrap();

    assert_eq!(reloaded, original);
    assert_eq!(reloaded.to_json().unwrap(), text);
    assert_eq!(DeltaStream::schema(), DELTA_SCHEMA);
    let value = document();
    assert_eq!(value["schema"], DELTA_SCHEMA);
    let items = value["deltas"].as_array().unwrap();
    assert_eq!(items.len(), original.deltas().len());
    assert!(items.iter().all(|item| item.get("schema").is_none()
        && item.get("op").is_none()
        && item.get("target").is_none()
        && item["type"].is_string()));
}

#[test]
fn given_a_stored_stream_when_replayed_then_the_draft_equals_the_one_built_from_the_original_deltas()
 {
    let original = deltas(&script());
    let reloaded = DeltaStream::from_json(&DeltaStream::new(original.clone()).to_json().unwrap())
        .unwrap()
        .into_deltas();
    let build = |steps: &[ResponseDelta]| {
        let mut draft = ResponseDraft::new(steps[0].turn(), steps[0].entry());
        for step in steps {
            draft.apply(step).unwrap();
        }
        draft
    };

    assert_eq!(build(&reloaded), build(&original));
    assert!(build(&reloaded).response().is_some());
}

#[test]
fn given_an_empty_stream_when_stored_then_it_round_trips() {
    let empty = DeltaStream::new(Vec::new());

    let reloaded = DeltaStream::from_json(&empty.to_json().unwrap()).unwrap();

    assert!(reloaded.deltas().is_empty());
}

#[test]
fn given_another_or_missing_schema_when_the_stream_is_loaded_then_schema_mismatch() {
    for schema in [
        json!("br-llm-messages-delta/2"),
        json!("br-llm-messages-draft/1"),
        json!(1),
    ] {
        let mut value = document();
        value["schema"] = schema;
        assert_eq!(load(value).code(), ErrorCode::SchemaMismatch);
    }
    let mut missing = document();
    missing.as_object_mut().unwrap().remove("schema");
    assert_eq!(load(missing).code(), ErrorCode::SchemaMismatch);
}

#[test]
fn given_a_bad_delta_inside_a_stream_when_loaded_then_the_error_names_its_index() {
    let mut value = document();
    value["deltas"][2]["type"] = json!("block_teleported");

    let error = load(value);

    assert_eq!(error.code(), ErrorCode::VariantMismatch);
    assert_eq!(error.location(), Some("deltas[2]"));
}

#[test]
fn given_a_delta_that_still_carries_a_schema_key_inside_a_stream_when_loaded_then_it_is_refused() {
    let mut value = document();
    value["deltas"][0]["schema"] = json!(DELTA_SCHEMA);

    let error = load(value);

    assert_eq!(error.code(), ErrorCode::InvalidContent);
    assert_eq!(error.location(), Some("deltas[0]"));
}

#[test]
fn given_unknown_or_missing_members_in_the_stream_document_when_loaded_then_invalid_content() {
    let mut extra = document();
    extra["cursor"] = json!(7);
    let mut missing = document();
    missing.as_object_mut().unwrap().remove("deltas");

    assert_eq!(load(extra).code(), ErrorCode::InvalidContent);
    assert_eq!(load(missing).code(), ErrorCode::InvalidContent);
}

#[test]
fn given_a_single_delta_when_serialized_then_it_has_no_schema_and_a_stream_of_one_has_it() {
    let one = delta(DeltaOp::text_appended(bid(11), "x"));

    let alone = one.to_value().unwrap().raw();
    let wrapped = DeltaStream::new(vec![one]).to_value().unwrap().raw();

    assert!(alone.get("schema").is_none());
    assert_eq!(wrapped["schema"], DELTA_SCHEMA);
    assert_eq!(wrapped["deltas"][0], alone);
}
