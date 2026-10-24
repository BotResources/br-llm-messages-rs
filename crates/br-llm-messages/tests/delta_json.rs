mod common;
use common::Raw as _;
mod stream_support;

use br_llm_messages::{
    BlockMetadataUpdate, BlockStart, DeltaOp, ErrorCode, ExecutionStatus, ExternalId, Finalization,
    InterruptionReason, ItemRef, ModelName, ReplayFormat, ResponseDelta, ResponseDeltaKind,
    ResponseDraft, ResponseEnding, ResponseMetadata, ResponseMetadataUpdate, ThinkingBlock, Usage,
};
use common::{bid, eid, signature, text_block};
use serde_json::{Value, json};
use stream_support::{citation, delta, operation, tid};

fn every_op() -> Vec<DeltaOp> {
    vec![
        DeltaOp::response_started(
            ResponseMetadata::new().with_model(ModelName::new("model-x").unwrap()),
        ),
        DeltaOp::block_started(
            bid(10),
            BlockStart::runtime_tool_call(
                Some(common::tool("search")),
                Some(ExternalId::new("toolu_1").unwrap()),
            )
            .with_arguments(common::oj(json!({"q": "provisional"})))
            .unwrap(),
        ),
        DeltaOp::text_appended(bid(11), "héllo "),
        DeltaOp::thinking_appended(bid(12), "pondering"),
        DeltaOp::json_appended(bid(10), "{\"q\":"),
        DeltaOp::replay_fragment_appended(
            bid(12),
            ReplayFormat::new("anthropic.signature").unwrap(),
            "sig-part",
        ),
        DeltaOp::replay_material_set(bid(11), signature("whole")),
        DeltaOp::block_metadata_updated(
            bid(11),
            BlockMetadataUpdate::citation_added(citation(50, 0, 3)),
        ),
        DeltaOp::block_metadata_updated(
            bid(11),
            BlockMetadataUpdate::item_ref_set(ItemRef::new("msg_1").unwrap()),
        ),
        DeltaOp::block_metadata_updated(
            bid(10),
            BlockMetadataUpdate::tool_name_set(common::tool("lookup")),
        ),
        DeltaOp::block_metadata_updated(
            bid(10),
            BlockMetadataUpdate::external_id_set(ExternalId::new("toolu_2").unwrap()),
        ),
        DeltaOp::block_metadata_updated(
            bid(13),
            BlockMetadataUpdate::operation_set(operation("code_run")),
        ),
        DeltaOp::block_metadata_updated(
            bid(13),
            BlockMetadataUpdate::execution_status_set(ExecutionStatus::Running),
        ),
        DeltaOp::block_inserted(ThinkingBlock::replay_only(bid(14), signature("s"))),
        DeltaOp::block_finalized(
            bid(11),
            Finalization::authoritative(text_block(11, "authoritative").into()),
        ),
        DeltaOp::response_metadata_updated(
            ResponseMetadataUpdate::new().with_usage(Usage::new().with_output_tokens(u64::MAX)),
        ),
        DeltaOp::response_completed(ResponseEnding::unclassified("vendor").unwrap()),
        DeltaOp::response_interrupted(InterruptionReason::failed().with_detail("busy").unwrap()),
    ]
}

fn wire(op: DeltaOp) -> Value {
    delta(op).to_value().unwrap().raw()
}

fn load(value: Value) -> br_llm_messages::Error {
    ResponseDelta::from_value(common::oj(value)).unwrap_err()
}

#[test]
fn given_every_operation_when_round_tripped_through_json_then_equal() {
    let deltas: Vec<ResponseDelta> = every_op().into_iter().map(delta).collect();

    for original in &deltas {
        let text = original.to_json().unwrap();
        assert_eq!(&ResponseDelta::from_json(&text).unwrap(), original);
    }

    let covered: std::collections::HashSet<ResponseDeltaKind> =
        deltas.iter().map(|d| d.kind()).collect();
    assert_eq!(covered.len(), ResponseDeltaKind::ALL.len());
}

#[test]
fn given_a_block_operation_when_serialized_then_one_flat_object_names_type_and_identities() {
    let value = wire(DeltaOp::text_appended(bid(11), "hi"));

    assert_eq!(
        value,
        json!({
            "type": "text_appended",
            "turn": tid(1).to_string(),
            "entry": eid(2).to_string(),
            "block": bid(11).to_string(),
            "text": "hi"
        })
    );
}

#[test]
fn given_a_response_operation_when_serialized_then_no_block_identity_is_present() {
    let value = wire(DeltaOp::response_completed(ResponseEnding::finished()));

    assert_eq!(
        value,
        json!({
            "type": "response_completed",
            "turn": tid(1).to_string(),
            "entry": eid(2).to_string(),
            "ending": {"type": "finished"}
        })
    );
}

#[test]
fn given_an_inserted_block_when_serialized_then_its_identity_is_a_top_level_block_key() {
    let value = wire(DeltaOp::block_inserted(text_block(21, "whole")));

    assert_eq!(value["type"], json!("block_inserted"));
    assert_eq!(value["block"], json!(bid(21).to_string()));
    assert_eq!(value["content"]["id"], json!(bid(21).to_string()));
    assert_eq!(value["content"]["type"], json!("text"));
}

#[test]
fn given_an_inserted_block_whose_declared_identity_differs_when_loaded_then_stream_target_error() {
    let mut value = wire(DeltaOp::block_inserted(text_block(21, "whole")));
    value["block"] = json!(bid(22).to_string());
    let mut missing = wire(DeltaOp::block_inserted(text_block(21, "whole")));
    missing.as_object_mut().unwrap().remove("block");

    let mismatch = load(value);

    assert_eq!(mismatch.code(), ErrorCode::StreamTargetError);
    assert_eq!(mismatch.object_id(), Some(bid(22).as_uuid()));
    assert_eq!(load(missing).code(), ErrorCode::InvalidContent);
}

#[test]
fn given_the_old_envelope_keys_when_loaded_then_they_are_refused_as_unknown_fields() {
    for key in ["schema", "target", "op"] {
        let mut value = wire(DeltaOp::text_appended(bid(11), "hi"));
        value[key] = json!("br-llm-messages-delta/1");

        assert_eq!(load(value).code(), ErrorCode::InvalidContent, "{key}");
    }
}

#[test]
fn given_an_unknown_operation_when_loaded_then_variant_mismatch_not_ignored() {
    for name in ["block_teleported", "block_discarded"] {
        let mut value = wire(DeltaOp::text_appended(bid(11), "hi"));
        value["type"] = json!(name);
        value.as_object_mut().unwrap().remove("text");

        assert_eq!(load(value).code(), ErrorCode::VariantMismatch, "{name}");
    }
}

#[test]
fn given_a_delta_without_turn_or_entry_when_loaded_then_invalid_content_names_the_key() {
    for key in ["turn", "entry"] {
        let mut value = wire(DeltaOp::text_appended(bid(11), "hi"));
        value.as_object_mut().unwrap().remove(key);

        let error = load(value);

        assert_eq!(error.code(), ErrorCode::InvalidContent);
        assert_eq!(error.location(), Some(key));
    }
}

#[test]
fn given_unknown_fields_in_nested_payloads_when_loaded_then_invalid_content() {
    let mut operation_level = wire(DeltaOp::text_appended(bid(11), "hi"));
    operation_level["extra"] = json!(1);
    let mut accumulated = wire(DeltaOp::block_finalized(
        bid(11),
        Finalization::accumulated(),
    ));
    accumulated["finalization"]["junk"] = json!(1);
    let mut start = wire(DeltaOp::block_started(bid(11), BlockStart::text()));
    start["start"]["junk"] = json!(1);
    let mut reason = wire(DeltaOp::response_interrupted(
        InterruptionReason::cancelled(),
    ));
    reason["reason"]["junk"] = json!(1);
    let mut update = wire(DeltaOp::block_metadata_updated(
        bid(11),
        BlockMetadataUpdate::item_ref_set(ItemRef::new("m").unwrap()),
    ));
    update["update"]["junk"] = json!(1);

    for value in [operation_level, accumulated, start, reason, update] {
        assert_eq!(load(value).code(), ErrorCode::InvalidContent);
    }
}

#[test]
fn given_an_identity_that_is_not_a_uuid_v7_when_loaded_then_invalid_identity() {
    let mut value = wire(DeltaOp::text_appended(bid(11), "hi"));
    value["block"] = json!("2f7d9b3e-6a9b-4c1b-8d5e-0c4f6a1b2c3d");

    assert_eq!(load(value).code(), ErrorCode::InvalidIdentity);
}

#[test]
fn given_truncated_json_or_a_non_object_when_loaded_then_distinct_codes() {
    let text = wire(DeltaOp::text_appended(bid(11), "hi")).to_string();

    assert_eq!(
        ResponseDelta::from_json(&text[..text.len() - 4])
            .unwrap_err()
            .code(),
        ErrorCode::MalformedJson
    );
    assert_eq!(load(json!([1])).code(), ErrorCode::InvalidContent);
}

#[test]
fn given_a_metadata_update_with_an_unknown_field_when_loaded_then_refused() {
    let mut value = wire(DeltaOp::response_metadata_updated(
        ResponseMetadataUpdate::new().with_model(ModelName::new("m").unwrap()),
    ));
    value["update"]["tokens"] = json!(5);

    assert_eq!(load(value).code(), ErrorCode::InvalidContent);
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
            "response_metadata_updated",
            "response_completed",
            "response_interrupted",
        ]
    );
    for kind in ResponseDeltaKind::ALL.iter().copied() {
        assert_eq!(kind.as_str().parse::<ResponseDeltaKind>().unwrap(), kind);
    }
    assert_eq!(
        "block_discarded"
            .parse::<ResponseDeltaKind>()
            .unwrap_err()
            .code(),
        ErrorCode::VariantMismatch
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
