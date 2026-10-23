mod common;
mod stream_support;

use br_llm_messages::{
    DeltaOp, ErrorCode, ResponseDelta, ResponseDraft, ResponseMetadata, ResponseMetadataUpdate,
    Usage,
};
use common::{eid, refused};
use stream_support::{delta, draft, ok, refuse};

#[test]
fn given_an_empty_usage_in_the_first_delta_when_the_draft_begins_then_invalid_content() {
    let first = delta(DeltaOp::response_started(
        ResponseMetadata::new().with_usage(Usage::new()),
    ));

    let error = refused(ResponseDraft::begin(&first));

    assert_eq!(error.code(), ErrorCode::InvalidContent);
    assert_eq!(error.object_id(), Some(eid(2).as_uuid()));
}

#[test]
fn given_an_empty_usage_snapshot_when_applied_then_it_is_refused_and_the_old_usage_stays() {
    let mut draft = draft();
    let kept = Usage::new().with_input_tokens(3);
    ok(
        &mut draft,
        DeltaOp::response_metadata_updated(ResponseMetadataUpdate::new().with_usage(kept.clone())),
    );

    let error = refuse(
        &mut draft,
        DeltaOp::response_metadata_updated(ResponseMetadataUpdate::new().with_usage(Usage::new())),
    );

    assert_eq!(error.code(), ErrorCode::InvalidContent);
    assert_eq!(draft.metadata().usage(), Some(&kept));
}

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
        ResponseMetadataUpdate::new().with_usage(Usage::new().with_input_tokens(3)),
    ));
    let mut wire: serde_json::Value = serde_json::from_str(&valid.to_json().unwrap()).unwrap();
    assert!(ResponseDelta::from_json(&wire.to_string()).is_ok());

    wire["op"]["update"]["usage"] = serde_json::json!({});
    let error = refused(ResponseDelta::from_json(&wire.to_string()));

    assert_eq!(error.code(), ErrorCode::InvalidContent);
}
