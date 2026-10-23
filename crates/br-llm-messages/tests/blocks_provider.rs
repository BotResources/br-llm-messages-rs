mod common;

use br_llm_messages::{
    ContentBlock, ErrorCode, ExecutionStatus, ExternalId, PlainBlock, ProviderOperation,
    ProviderToolCall, ProviderToolExecutionRecord, ProviderToolResult, RedactedThinkingBlock,
    ResultExposure, ResultStatus, RuntimeToolCall, RuntimeToolResult, TextBlock, ThinkingBlock,
};
use common::{bid, envelope, plain_text, png, refused, signature, text_block, tool, v7};
use serde_json::json;

fn every_non_plain_kind() -> Vec<ContentBlock> {
    vec![
        ThinkingBlock::new(bid(1), "t").into(),
        RedactedThinkingBlock::new(bid(2), signature("s")).into(),
        RuntimeToolCall::new(bid(3), tool("t"), common::oj(json!({})))
            .unwrap()
            .into(),
        RuntimeToolResult::new(bid(4), bid(3), tool("t"), ResultStatus::Error, vec![])
            .unwrap()
            .into(),
        ProviderToolCall::new(bid(5)).into(),
        ProviderToolResult::new(bid(6), bid(5), vec![plain_text(7, "r")])
            .unwrap()
            .into(),
        ProviderToolExecutionRecord::new(bid(8), ResultExposure::not_exposed())
            .unwrap()
            .into(),
    ]
}

#[test]
fn given_any_non_plain_block_when_made_plain_then_disallowed_block_kind_with_its_id() {
    for block in every_non_plain_kind() {
        let id = block.id();
        let error = refused(PlainBlock::try_from(block));

        assert_eq!(error.code(), ErrorCode::DisallowedBlockKind);
        assert_eq!(error.object_id(), Some(id.as_uuid()));
    }
}

#[test]
fn given_plain_kinds_when_made_plain_then_accepted_and_converted_back() {
    let text = ContentBlock::from(text_block(1, "x"));
    let image = ContentBlock::from(png(2));

    let plain = [
        PlainBlock::try_from(text.clone()).unwrap(),
        PlainBlock::try_from(image.clone()).unwrap(),
    ];

    assert_eq!(ContentBlock::from(plain[0].clone()), text);
    assert_eq!(ContentBlock::from(plain[1].clone()), image);
    assert_eq!(plain[0].id(), bid(1));
}

#[test]
fn given_text_carrying_replay_when_made_plain_then_invalid_content() {
    let replayed = TextBlock::new(bid(1), "x")
        .unwrap()
        .with_replay(signature("s"));

    assert_eq!(
        refused(PlainBlock::try_from(replayed.clone())).code(),
        ErrorCode::InvalidContent
    );
    assert_eq!(
        refused(PlainBlock::try_from(ContentBlock::from(replayed))).code(),
        ErrorCode::InvalidContent
    );
}

#[test]
fn given_a_plain_slice_with_a_repeated_id_when_checked_then_reused_identity() {
    let error = refused(PlainBlock::ensure_distinct_ids(&[
        plain_text(1, "a"),
        plain_text(1, "b"),
    ]));

    assert_eq!(error.code(), ErrorCode::ReusedIdentity);
    assert_eq!(error.location(), Some("[1]"));
}

#[test]
fn given_a_provider_call_with_everything_exposed_when_built_then_every_field_is_retained() {
    let call = ProviderToolCall::new(bid(1))
        .with_external_id(ExternalId::new("srvtoolu_1").unwrap())
        .with_operation(ProviderOperation::new("web_search").unwrap())
        .with_arguments(common::oj(json!(null)))
        .unwrap()
        .with_replay(signature("s"));

    assert_eq!(call.external_id().map(|e| e.as_str()), Some("srvtoolu_1"));
    assert_eq!(call.operation().map(|o| o.as_str()), Some("web_search"));
    assert_eq!(call.arguments(), Some(&common::oj(json!(null))));
    assert_eq!(call.replay(), Some(&signature("s")));
}

#[test]
fn given_a_provider_call_exposing_nothing_when_built_then_absence_is_kept() {
    let call = ProviderToolCall::new(bid(1));

    assert_eq!(call.external_id(), None);
    assert_eq!(call.operation(), None);
    assert_eq!(call.arguments(), None);
    assert_eq!(call.replay(), None);
}

#[test]
fn given_a_provider_result_when_built_then_it_links_to_the_call_by_identity() {
    let result = ProviderToolResult::new(bid(2), bid(1), vec![plain_text(3, "found")])
        .unwrap()
        .with_external_id(ExternalId::new("res_1").unwrap());

    assert_eq!(result.call(), bid(1));
    assert_eq!(result.content().len(), 1);
    assert_eq!(result.external_id().map(|e| e.as_str()), Some("res_1"));
}

#[test]
fn given_no_content_and_no_replay_when_a_provider_result_is_built_then_missing_content() {
    let error = refused(ProviderToolResult::new(bid(2), bid(1), vec![]));

    assert_eq!(error.code(), ErrorCode::MissingContent);
    assert_eq!(error.object_id(), Some(v7(2)));
}

#[test]
fn given_replay_only_when_a_provider_result_is_built_then_accepted_without_invented_content() {
    let opaque = ProviderToolResult::replay_only(bid(2), bid(1), envelope(json!({}))).unwrap();

    assert!(opaque.content().is_empty());
    assert!(opaque.replay().is_some());
}

#[test]
fn given_a_provider_result_nested_id_clash_when_built_then_reused_identity() {
    let error = refused(ProviderToolResult::new(
        bid(2),
        bid(1),
        vec![plain_text(2, "x")],
    ));

    assert_eq!(error.code(), ErrorCode::ReusedIdentity);
}

#[test]
fn given_readable_or_mixed_exposure_without_content_when_built_then_missing_content() {
    assert_eq!(
        refused(ResultExposure::readable(vec![])).code(),
        ErrorCode::MissingContent
    );
    assert_eq!(
        refused(ResultExposure::mixed(vec![], signature("s"))).code(),
        ErrorCode::MissingContent
    );
}

#[test]
fn given_each_exposure_when_inspected_then_content_and_material_match_its_kind() {
    let readable = ResultExposure::readable(vec![plain_text(1, "a")]).unwrap();
    let opaque = ResultExposure::opaque(signature("s"));
    let mixed = ResultExposure::mixed(vec![plain_text(2, "b")], signature("m")).unwrap();
    let hidden = ResultExposure::not_exposed();

    assert_eq!((readable.content().len(), readable.material()), (1, None));
    assert_eq!(
        (opaque.content().len(), opaque.material()),
        (0, Some(&signature("s")))
    );
    assert_eq!(
        (mixed.content().len(), mixed.material()),
        (1, Some(&signature("m")))
    );
    assert_eq!((hidden.content().len(), hidden.material()), (0, None));
}

#[test]
fn given_a_record_without_status_when_built_then_status_stays_absent_and_never_defaults() {
    let bare = ProviderToolExecutionRecord::new(bid(1), ResultExposure::not_exposed()).unwrap();
    let running = ProviderToolExecutionRecord::new(bid(2), ResultExposure::not_exposed())
        .unwrap()
        .with_operation(ProviderOperation::new("code_execution").unwrap())
        .with_status(ExecutionStatus::Running);

    assert_eq!(bare.status(), None);
    assert_eq!(running.status(), Some(ExecutionStatus::Running));
    assert_eq!(
        running.operation().map(|o| o.as_str()),
        Some("code_execution")
    );
    assert!(matches!(bare.exposure(), ResultExposure::NotExposed { .. }));
}

#[test]
fn given_exposure_content_reusing_the_record_id_when_built_then_reused_identity() {
    let exposure = ResultExposure::readable(vec![plain_text(1, "x")]).unwrap();

    let error = refused(ProviderToolExecutionRecord::new(bid(1), exposure));

    assert_eq!(error.code(), ErrorCode::ReusedIdentity);
    assert_eq!(error.location(), Some("exposure.content[0]"));
}
