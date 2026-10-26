mod common;
mod stream_support;

use br_llm_messages::{DeltaOp, ErrorCode, ResponseDelta, ResponseMetadataUpdate, Usage};
use common::refused;
use stream_support::{delta, draft, ok};

#[test]
fn given_a_metadata_update_without_usage_when_applied_then_it_is_not_a_usage_refusal() {
    let mut draft = draft();

    ok(
        &mut draft,
        DeltaOp::response_metadata_updated(ResponseMetadataUpdate::new()),
    );

    assert_eq!(draft.metadata().usage(), None);
}

#[test]
fn given_an_empty_usage_in_delta_json_when_loaded_then_invalid_content() {
    let valid = delta(DeltaOp::response_metadata_updated(
        ResponseMetadataUpdate::new().with_usage(Usage::of_input_tokens(3)),
    ));
    let mut wire: serde_json::Value = serde_json::from_str(&valid.to_json().unwrap()).unwrap();
    assert!(ResponseDelta::from_json(&wire.to_string()).is_ok());

    wire["update"]["usage"] = serde_json::json!({});
    let error = refused(ResponseDelta::from_json(&wire.to_string()));

    assert_eq!(error.code(), ErrorCode::InvalidContent);
}

#[test]
fn given_an_empty_usage_in_a_started_delta_json_when_loaded_then_invalid_content() {
    let valid = delta(DeltaOp::response_started(
        br_llm_messages::ResponseMetadata::new().with_usage(Usage::of_output_tokens(3)),
    ));
    let mut wire: serde_json::Value = serde_json::from_str(&valid.to_json().unwrap()).unwrap();
    assert!(ResponseDelta::from_json(&wire.to_string()).is_ok());

    wire["metadata"]["usage"] = serde_json::json!({});
    let error = refused(ResponseDelta::from_json(&wire.to_string()));

    assert_eq!(error.code(), ErrorCode::InvalidContent);
}
