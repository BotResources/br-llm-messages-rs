mod common;
use common::Raw as _;
mod snapshot_support;
mod stream_support;

use br_llm_messages::{DraftSnapshot, ErrorCode, RecordSet, ResponseDraft};
use common::records::{rich_set, try_edit};
use serde_json::{Value, json};
use snapshot_support::{deltas, draft_after, script};

fn snapshot() -> DraftSnapshot {
    draft_after(&deltas(&script()), 24).to_snapshot().unwrap()
}

fn model_response_payload(document: &mut Value) -> &mut Value {
    document["entries"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|row| row["kind"] == "model_response")
        .map(|row| &mut row["payload"])
        .expect("a model response row")
}

#[test]
fn given_a_record_set_with_an_explicit_null_on_an_optional_payload_key_when_loaded_then_it_is_refused()
 {
    for key in ["model", "usage", "provider_response_id"] {
        let result = try_edit(&rich_set(), |document| {
            model_response_payload(document)[key] = Value::Null;
        });

        assert_eq!(
            result.unwrap_err().code(),
            ErrorCode::InvalidContent,
            "{key}"
        );
    }
}

#[test]
fn given_a_draft_snapshot_with_an_explicit_null_on_an_optional_payload_key_when_loaded_then_it_is_refused()
 {
    for key in ["model", "usage", "provider_response_id"] {
        let mut document = snapshot().to_value().unwrap().raw();
        document["entry"]["payload"][key] = Value::Null;

        let result = DraftSnapshot::from_value(common::oj(document));

        assert_eq!(
            result.unwrap_err().code(),
            ErrorCode::InvalidContent,
            "{key}"
        );
    }
}

#[test]
fn given_a_stored_snapshot_when_restored_and_taken_again_then_it_is_equal_and_byte_stable() {
    let stored = snapshot();

    let restored = ResponseDraft::from_snapshot(&stored).unwrap();
    let again = restored.to_snapshot().unwrap();

    assert_eq!(again, stored);
    assert_eq!(again.to_json().unwrap(), stored.to_json().unwrap());
}

#[test]
fn given_a_stored_record_set_when_reloaded_then_it_is_equal_and_byte_stable() {
    let stored = rich_set();

    let reloaded = RecordSet::from_json(&stored.to_json().unwrap()).unwrap();

    assert_eq!(reloaded, stored);
    assert_eq!(reloaded.to_json().unwrap(), stored.to_json().unwrap());
}

#[test]
fn given_a_provider_call_row_with_a_null_arguments_value_when_loaded_then_it_is_refused() {
    let result = try_edit(&rich_set(), |document| {
        let row = document["blocks"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|row| row["kind"] == "provider_tool_call")
            .expect("a provider call row");
        row["payload"]["arguments"] = json!(null);
    });

    assert_eq!(result.unwrap_err().code(), ErrorCode::InvalidContent);
}

#[test]
fn given_a_record_set_with_a_null_inside_a_nested_optional_value_when_loaded_then_it_is_refused() {
    let result = try_edit(&rich_set(), |document| {
        model_response_payload(document)["usage"]["thinking_tokens"] = Value::Null;
    });

    assert_eq!(result.unwrap_err().code(), ErrorCode::InvalidContent);
}
