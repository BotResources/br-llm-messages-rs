mod common;
mod stream_support;

use br_llm_messages::{
    ContentBlock, DeltaOp, ErrorCode, ExternalId, Finalization, ProviderOperation,
    ProviderToolCall, RuntimeToolCall, StructuredBlock, TextBlock,
};
use common::{bid, cid, signature, text_block, tool};
use serde_json::json;
use stream_support::{
    citation, finalize, finish, json, ok, refuse, run, start_call, start_json, start_provider_call,
    start_text, text,
};

fn authoritative(block: impl Into<ContentBlock>) -> DeltaOp {
    let block = block.into();
    DeltaOp::block_finalized(block.id(), Finalization::authoritative(block))
}

#[test]
fn given_accumulated_content_when_finalized_then_each_kind_becomes_a_validated_block() {
    let mut draft = run(vec![
        start_text(10),
        text(10, "hello"),
        start_json(11),
        json(11, "{\"k\":"),
        json(11, "[1,2]}"),
        start_call(12, Some("lookup"), Some("toolu_1")),
        json(12, "{\"q\":\"x\"}"),
        start_provider_call(13),
        json(13, "{\"p\":1}"),
        start_provider_call(14),
    ]);

    for block in [10, 11, 12, 13, 14] {
        ok(&mut draft, finalize(block));
    }

    let view = draft.view();
    let block = |n: u64| view.slot(bid(n)).unwrap().final_block().unwrap().clone();
    assert_eq!(block(10), text_block(10, "hello").into());
    assert_eq!(
        block(11),
        StructuredBlock::new(bid(11), common::oj(json!({"k": [1, 2]})))
            .unwrap()
            .into()
    );
    assert_eq!(
        block(12),
        RuntimeToolCall::new(bid(12), tool("lookup"), common::oj(json!({"q": "x"})))
            .unwrap()
            .with_external_id(ExternalId::new("toolu_1").unwrap())
            .into()
    );
    let expected_provider = ProviderToolCall::new(bid(13))
        .with_operation(ProviderOperation::new("web_search").unwrap())
        .with_arguments(common::oj(json!({"p": 1})))
        .unwrap();
    assert_eq!(block(13), expected_provider.into());
    assert!(matches!(
        block(14),
        ContentBlock::ProviderToolCall(call) if call.arguments().is_none()
    ));
}

#[test]
fn given_citations_added_while_streaming_when_finalized_then_they_are_validated_against_the_text() {
    let mut draft = run(vec![
        start_text(10),
        text(10, "abc"),
        DeltaOp::block_metadata_updated(
            bid(10),
            br_llm_messages::BlockMetadataUpdate::citation_added(citation(50, 0, 99)),
        ),
    ]);

    let error = refuse(&mut draft, finalize(10));

    assert_eq!(error.code(), ErrorCode::InconsistentFinalization);
    assert_eq!(error.object_id(), Some(cid(50).as_uuid()));
    assert_eq!(error.location(), Some("citations[0].range"));
}

#[test]
fn given_invalid_accumulated_content_when_finalized_then_the_specific_code_and_nothing_changes() {
    let mut draft = run(vec![
        start_text(10),
        text(10, " \n "),
        start_json(11),
        json(11, "{\"a\":"),
        start_json(12),
        start_call(13, None, None),
        json(13, "{}"),
        start_call(14, Some("t"), None),
        json(14, "{\"a\":"),
    ]);

    let cases = [
        (10, ErrorCode::InconsistentFinalization),
        (11, ErrorCode::MalformedJson),
        (12, ErrorCode::InconsistentFinalization),
        (13, ErrorCode::InconsistentFinalization),
        (14, ErrorCode::MalformedJson),
    ];
    for (block, code) in cases {
        let error = refuse(&mut draft, finalize(block));
        assert_eq!(error.code(), code);
        assert_eq!(error.object_id(), Some(bid(block).as_uuid()));
    }
}

#[test]
fn given_an_authoritative_value_when_finalized_then_provisional_fields_are_replaced_and_the_id_is_kept()
 {
    let mut draft = run(vec![
        start_call(10, Some("draft_name"), Some("provisional")),
        json(10, "{\"half\":"),
    ]);
    let value = RuntimeToolCall::new(
        bid(10),
        tool("real_name"),
        common::oj(json!({"full": true})),
    )
    .unwrap()
    .with_external_id(ExternalId::new("toolu_final").unwrap());

    ok(&mut draft, authoritative(value.clone()));

    let slot = draft.view().slot(bid(10)).unwrap();
    assert_eq!(slot.final_block(), Some(&ContentBlock::from(value)));
}

#[test]
fn given_an_authoritative_text_when_finalized_then_it_replaces_the_accumulated_text_never_appended()
{
    let mut draft = run(vec![start_text(10), text(10, "Hel")]);

    ok(
        &mut draft,
        authoritative(TextBlock::new(bid(10), "Hello").unwrap()),
    );
    ok(&mut draft, finish());

    assert_eq!(
        draft.response().unwrap().blocks(),
        &[text_block(10, "Hello").into()]
    );
}

#[test]
fn given_an_authoritative_value_when_it_replaces_replay_then_the_replacement_is_exact() {
    let mut draft = run(vec![
        start_text(10),
        text(10, "x"),
        DeltaOp::replay_material_set(bid(10), signature("old")),
    ]);
    let value = TextBlock::new(bid(10), "x")
        .unwrap()
        .with_replay(signature("new"));

    ok(&mut draft, authoritative(value));

    let slot = draft.view().slot(bid(10)).unwrap();
    assert_eq!(slot.replay(), Some(&signature("new")));
}

#[test]
fn given_an_authoritative_value_with_another_id_or_kind_when_finalized_then_inconsistent_finalization()
 {
    let mut draft = run(vec![start_text(10), text(10, "x")]);
    let other_id = DeltaOp::block_finalized(
        bid(10),
        Finalization::authoritative(text_block(11, "x").into()),
    );
    let other_kind = DeltaOp::block_finalized(
        bid(10),
        Finalization::authoritative(common::call(10, "t").into()),
    );

    for op in [other_id, other_kind] {
        let error = refuse(&mut draft, op);
        assert_eq!(error.code(), ErrorCode::InconsistentFinalization);
        assert_eq!(error.object_id(), Some(bid(10).as_uuid()));
    }
}

#[test]
fn given_an_authoritative_value_with_a_taken_nested_identity_when_finalized_then_reused_identity() {
    let mut draft = run(vec![start_text(10), start_text(11)]);
    let value = TextBlock::new(bid(10), "x")
        .unwrap()
        .with_citations(vec![citation(11, 0, 1)])
        .unwrap();

    let error = refuse(&mut draft, authoritative(value));

    assert_eq!(error.code(), ErrorCode::ReusedIdentity);
}

#[test]
fn given_a_finalized_block_when_finalized_again_or_mutated_then_stream_phase() {
    let mut draft = run(vec![start_text(10), text(10, "x"), finalize(10)]);

    let ops = [
        finalize(10),
        authoritative(text_block(10, "y")),
        text(10, "more"),
    ];
    for op in ops {
        let error = refuse(&mut draft, op);
        assert_eq!(error.code(), ErrorCode::StreamPhaseError);
    }
}

#[test]
fn given_blank_accumulated_text_when_finalized_then_inconsistent_finalization_and_the_block_stays_open()
 {
    let mut draft = run(vec![start_text(10), text(10, "\n\n")]);

    let error = refuse(&mut draft, finalize(10));

    assert_eq!(error.code(), ErrorCode::InconsistentFinalization);
    assert_eq!(error.object_id(), Some(bid(10).as_uuid()));
    ok(&mut draft, text(10, "late"));
    ok(&mut draft, finalize(10));
}
