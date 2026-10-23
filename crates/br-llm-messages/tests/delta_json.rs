mod common;
mod stream_support;

use br_llm_messages::{
    BlockMetadataUpdate, BlockStart, DeltaOp, ErrorCode, ExecutionStatus, ExternalId,
    FailureDetail, Finalization, InterruptionReason, ModelName, ResponseDelta, ResponseDeltaKind,
    ResponseDraft, ResponseEnding, ResponseMetadata, ResponseMetadataUpdate, ThinkingBlock, Usage,
};
use common::{bid, eid, signature, text_block};
use serde_json::{Value, json};
use stream_support::{citation, delta, tid};

fn every_op() -> Vec<DeltaOp> {
    vec![
        DeltaOp::response_started(
            ResponseMetadata::new().with_model(ModelName::new("model-x").unwrap()),
        ),
        DeltaOp::block_started(
            bid(10),
            0,
            BlockStart::runtime_tool_call(
                Some(common::tool("search")),
                Some(ExternalId::new("toolu_1").unwrap()),
            ),
        ),
        DeltaOp::text_appended(bid(11), "héllo "),
        DeltaOp::thinking_appended(bid(12), "pondering"),
        DeltaOp::json_appended(bid(10), "{\"q\":"),
        DeltaOp::replay_fragment_appended(
            bid(12),
            br_llm_messages::ReplayFormat::new("anthropic.signature").unwrap(),
            "sig-part",
        ),
        DeltaOp::replay_material_set(bid(11), signature("whole")),
        DeltaOp::block_metadata_updated(
            bid(11),
            BlockMetadataUpdate::citation_added(citation(50, 0, 3)),
        ),
        DeltaOp::block_metadata_updated(
            bid(13),
            BlockMetadataUpdate::execution_status_observed(ExecutionStatus::Running),
        ),
        DeltaOp::block_inserted(3, ThinkingBlock::new(bid(14), "", Some(signature("s")))),
        DeltaOp::block_finalized(
            bid(11),
            Finalization::authoritative(text_block(11, "authoritative").into()),
        ),
        DeltaOp::block_discarded(bid(15)),
        DeltaOp::response_metadata_updated(
            ResponseMetadataUpdate::new().with_usage(Usage::new().with_output_tokens(u64::MAX)),
        ),
        DeltaOp::response_completed(ResponseEnding::unclassified("vendor").unwrap()),
        DeltaOp::response_interrupted(InterruptionReason::failed(
            FailureDetail::new("overloaded", Some("busy".into())).unwrap(),
        )),
    ]
}

fn wire(op: DeltaOp) -> Value {
    delta(op).to_value().unwrap()
}

#[test]
fn given_every_operation_when_round_tripped_through_json_then_equal() {
    let deltas: Vec<ResponseDelta> = every_op().into_iter().map(delta).collect();

    for original in &deltas {
        let text = original.to_json().unwrap();
        assert_eq!(&ResponseDelta::from_json(&text).unwrap(), original);
    }

    let mut covered: Vec<ResponseDeltaKind> = deltas.iter().map(|d| d.kind()).collect();
    covered.sort();
    covered.dedup();
    assert_eq!(covered.len(), ResponseDeltaKind::ALL.len());
}

#[test]
fn given_a_block_operation_when_serialized_then_the_operation_is_nested_with_an_explicit_target() {
    let value = wire(DeltaOp::text_appended(bid(11), "hi"));

    assert_eq!(
        value,
        json!({
            "schema": "br-llm-messages-delta/1",
            "turn": tid(1).to_string(),
            "entry": eid(2).to_string(),
            "target": "block",
            "op": {"type": "text_appended", "block": bid(11).to_string(), "text": "hi"}
        })
    );
}

#[test]
fn given_a_response_operation_when_serialized_then_the_target_is_response() {
    let value = wire(DeltaOp::response_completed(ResponseEnding::Finished));

    assert_eq!(value["target"], json!("response"));
    assert_eq!(
        value["op"],
        json!({"type": "response_completed", "ending": {"type": "finished"}})
    );
}

#[test]
fn given_a_target_that_contradicts_the_operation_when_loaded_then_stream_target_with_the_block_id()
{
    let mut block_op = wire(DeltaOp::text_appended(bid(11), "hi"));
    block_op["target"] = json!("response");
    let mut response_op = wire(DeltaOp::response_completed(ResponseEnding::Finished));
    response_op["target"] = json!("block");

    let first = ResponseDelta::from_value(block_op).unwrap_err();
    let second = ResponseDelta::from_value(response_op).unwrap_err();

    assert_eq!(first.code(), ErrorCode::StreamTarget);
    assert_eq!(first.object_id(), Some(bid(11).as_uuid()));
    assert_eq!(second.code(), ErrorCode::StreamTarget);
    assert_eq!(second.object_id(), Some(eid(2).as_uuid()));
}

#[test]
fn given_an_unknown_operation_when_loaded_then_unsupported_variant_not_ignored() {
    let mut value = wire(DeltaOp::block_discarded(bid(11)));
    value["op"] = json!({"type": "block_teleported", "block": bid(11).to_string()});

    let error = ResponseDelta::from_value(value).unwrap_err();

    assert_eq!(error.code(), ErrorCode::UnsupportedVariant);
}

#[test]
fn given_another_or_missing_schema_when_loaded_then_schema_mismatch_before_shape_errors() {
    let mut other = wire(DeltaOp::block_discarded(bid(11)));
    other["schema"] = json!("br-llm-messages-delta/2");
    let mut missing = wire(DeltaOp::block_discarded(bid(11)));
    missing.as_object_mut().unwrap().remove("schema");
    let mut broken_shape = json!({"schema": "br-llm-messages/4", "op": 3});

    for value in [other, missing, broken_shape.take()] {
        assert_eq!(
            ResponseDelta::from_value(value).unwrap_err().code(),
            ErrorCode::SchemaMismatch
        );
    }
}

#[test]
fn given_unknown_fields_anywhere_when_loaded_then_unsupported_field() {
    let mut envelope = wire(DeltaOp::block_discarded(bid(11)));
    envelope["extra"] = json!(1);
    let mut operation_level = wire(DeltaOp::block_discarded(bid(11)));
    operation_level["op"]["extra"] = json!(1);
    let mut accumulated = wire(DeltaOp::block_finalized(
        bid(11),
        Finalization::accumulated(),
    ));
    accumulated["op"]["finalization"]["junk"] = json!(1);
    let mut start = wire(DeltaOp::block_started(bid(11), 0, BlockStart::text()));
    start["op"]["start"]["junk"] = json!(1);
    let mut reason = wire(DeltaOp::response_interrupted(InterruptionReason::Cancelled));
    reason["op"]["reason"]["junk"] = json!(1);

    for value in [envelope, operation_level, accumulated, start, reason] {
        assert_eq!(
            ResponseDelta::from_value(value).unwrap_err().code(),
            ErrorCode::UnsupportedField
        );
    }
}

#[test]
fn given_an_identity_that_is_not_a_uuid_v7_when_loaded_then_invalid_identity() {
    let mut value = wire(DeltaOp::block_discarded(bid(11)));
    value["op"]["block"] = json!("2f7d9b3e-6a9b-4c1b-8d5e-0c4f6a1b2c3d");

    assert_eq!(
        ResponseDelta::from_value(value).unwrap_err().code(),
        ErrorCode::InvalidIdentity
    );
}

#[test]
fn given_a_missing_operation_or_truncated_json_when_loaded_then_distinct_codes() {
    let mut value = wire(DeltaOp::block_discarded(bid(11)));
    value.as_object_mut().unwrap().remove("op");
    let text = wire(DeltaOp::block_discarded(bid(11))).to_string();

    assert_eq!(
        ResponseDelta::from_value(value).unwrap_err().code(),
        ErrorCode::MissingContent
    );
    assert_eq!(
        ResponseDelta::from_json(&text[..text.len() - 4])
            .unwrap_err()
            .code(),
        ErrorCode::MalformedJson
    );
}

#[test]
fn given_a_metadata_update_with_an_unknown_field_when_loaded_then_refused() {
    let mut value = wire(DeltaOp::response_metadata_updated(
        ResponseMetadataUpdate::new().with_model(ModelName::new("m").unwrap()),
    ));
    value["op"]["update"]["tokens"] = json!(5);

    assert_eq!(
        ResponseDelta::from_value(value).unwrap_err().code(),
        ErrorCode::UnsupportedField
    );
}

#[test]
fn given_the_delta_kinds_when_listed_then_names_are_stable_and_parse_back() {
    let names: Vec<&str> = ResponseDeltaKind::ALL.iter().map(|k| k.as_str()).collect();

    assert_eq!(
        names,
        vec![
            "response_started",
            "block_started",
            "text_appended",
            "thinking_appended",
            "json_appended",
            "replay_fragment_appended",
            "replay_material_set",
            "block_metadata_updated",
            "block_inserted",
            "block_finalized",
            "block_discarded",
            "response_metadata_updated",
            "response_completed",
            "response_interrupted",
        ]
    );
    for kind in ResponseDeltaKind::ALL {
        assert_eq!(kind.as_str().parse::<ResponseDeltaKind>().unwrap(), kind);
    }
    assert_eq!(
        "block_teleported"
            .parse::<ResponseDeltaKind>()
            .unwrap_err()
            .code(),
        ErrorCode::UnsupportedVariant
    );
}

#[test]
fn given_deltas_and_drafts_when_inspected_then_clone_eq_debug_send_sync() {
    fn assert_traits<T: Clone + PartialEq + std::fmt::Debug + Send + Sync>() {}

    assert_traits::<ResponseDelta>();
    assert_traits::<DeltaOp>();
    assert_traits::<ResponseDraft>();
    assert_traits::<BlockMetadataUpdate>();
}
