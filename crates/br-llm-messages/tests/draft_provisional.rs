mod common;
mod stream_support;
use common::Raw as _;

use br_llm_messages::{
    BlockStart, ContentBlock, DeltaOp, DeltaOutcome, DraftSnapshot, ErrorCode, ExecutionStatus,
    Finalization, ProviderToolCall, ProviderToolExecutionRecord, ResponseDelta, ResponseDraft,
    ResultExposure, RuntimeToolCall,
};
use common::{bid, oj, signature, text_block, tool};
use serde_json::{Value, json};
use stream_support::{
    delta, finalize, json, ok, operation, refuse, run, start_provider_call, start_record,
    start_text,
};

fn call_with(provisional: Option<Value>) -> DeltaOp {
    let start = BlockStart::runtime_tool_call(Some(tool("lookup")), None);
    let start = match provisional {
        Some(value) => start.with_arguments(oj(value)).unwrap(),
        None => start,
    };
    DeltaOp::block_started(bid(10), start)
}

fn arguments_of(draft: &ResponseDraft) -> Value {
    match draft.view().slot(bid(10)).unwrap().final_block() {
        Some(ContentBlock::RuntimeToolCall(call)) => call.arguments().raw(),
        other => panic!("unexpected {other:?}"),
    }
}

#[test]
fn given_a_provisional_object_and_no_fragments_when_finalized_then_the_provisional_value_is_used() {
    let mut draft = run(vec![call_with(Some(json!({"q": "provisional"})))]);

    ok(&mut draft, finalize(10));

    assert_eq!(arguments_of(&draft), json!({"q": "provisional"}));
}

#[test]
fn given_a_provisional_object_and_fragments_when_finalized_then_the_fragments_win() {
    let mut draft = run(vec![
        call_with(Some(json!({"q": "provisional"}))),
        json(10, "{\"q\":\"final\"}"),
    ]);

    ok(&mut draft, finalize(10));

    assert_eq!(arguments_of(&draft), json!({"q": "final"}));
}

#[test]
fn given_whitespace_only_fragments_when_finalized_then_they_count_as_none() {
    let mut with_provisional = run(vec![
        call_with(Some(json!({"q": 1}))),
        json(10, " "),
        json(10, "\n\t"),
    ]);
    let mut without = run(vec![call_with(None), json(10, "  ")]);

    ok(&mut with_provisional, finalize(10));
    ok(&mut without, finalize(10));

    assert_eq!(arguments_of(&with_provisional), json!({"q": 1}));
    assert_eq!(arguments_of(&without), json!({}));
}

#[test]
fn given_no_fragment_and_no_provisional_when_a_runtime_call_is_finalized_then_arguments_are_empty()
{
    let mut draft = run(vec![call_with(None)]);

    ok(&mut draft, finalize(10));

    assert_eq!(arguments_of(&draft), json!({}));
}

#[test]
fn given_fragments_that_are_not_an_object_when_a_runtime_call_is_finalized_then_invalid_content() {
    let mut draft = run(vec![call_with(None), json(10, "[1]")]);

    let error = refuse(&mut draft, finalize(10));

    assert_eq!(error.code(), ErrorCode::InvalidContent);
    assert_eq!(error.object_id(), Some(bid(10).as_uuid()));
}

#[test]
fn given_a_provider_call_when_finalized_then_the_same_precedence_applies_and_absent_arguments_stay_absent()
 {
    let mut bare = run(vec![start_provider_call(10)]);
    let mut provisional = run(vec![DeltaOp::block_started(
        bid(10),
        BlockStart::provider_tool_call(Some(operation("web_search")), None)
            .with_arguments(oj(json!([1, 2])))
            .unwrap(),
    )]);

    ok(&mut bare, finalize(10));
    ok(&mut provisional, finalize(10));

    let arguments = |draft: &ResponseDraft| match draft.view().slot(bid(10)).unwrap().final_block()
    {
        Some(ContentBlock::ProviderToolCall(call)) => call.arguments().map(|a| a.raw()),
        other => panic!("unexpected {other:?}"),
    };
    assert_eq!(arguments(&bare), None);
    assert_eq!(arguments(&provisional), Some(json!([1, 2])));
}

#[test]
fn given_a_provisional_runtime_value_that_is_not_an_object_when_built_or_loaded_then_invalid_content()
 {
    let built = BlockStart::runtime_tool_call(None, None)
        .with_arguments(oj(json!([1])))
        .unwrap_err();
    let mut wire = delta(call_with(Some(json!({"q": 1}))))
        .to_value()
        .unwrap()
        .raw();
    wire["start"]["arguments"] = json!("text");
    let loaded = ResponseDelta::from_value(oj(wire)).unwrap_err();

    assert_eq!(built.code(), ErrorCode::InvalidContent);
    assert_eq!(loaded.code(), ErrorCode::InvalidContent);
    assert_eq!(
        BlockStart::text()
            .with_arguments(oj(json!({})))
            .unwrap_err()
            .code(),
        ErrorCode::DisallowedKind
    );
}

#[test]
fn given_a_provisional_value_nested_beyond_the_depth_limit_when_built_or_loaded_then_it_is_refused()
{
    let deep = |levels: usize| (0..levels).fold(json!(1), |inner, _| json!({ "k": inner }));
    assert!(
        BlockStart::runtime_tool_call(None, None)
            .with_arguments(oj(deep(64)))
            .is_ok()
    );
    let built = BlockStart::runtime_tool_call(None, None)
        .with_arguments(oj(deep(65)))
        .unwrap_err();
    let mut wire = delta(call_with(Some(json!({"q": 1}))))
        .to_value()
        .unwrap()
        .raw();
    wire["start"]["arguments"] = deep(65);
    let loaded = ResponseDelta::from_value(oj(wire)).unwrap_err();

    assert_eq!(built.code(), ErrorCode::InvalidContent);
    assert_eq!(loaded.code(), ErrorCode::InvalidContent);
}

#[test]
fn given_blank_accumulated_structured_content_when_finalized_then_the_same_code_as_blank_text() {
    let mut structured = run(vec![stream_support::start_json(11), json(11, " \n")]);
    let mut blank_text = run(vec![start_text(12), stream_support::text(12, " \n")]);

    let first = refuse(&mut structured, finalize(11));
    let second = refuse(&mut blank_text, finalize(12));

    assert_eq!(first.code(), ErrorCode::InconsistentFinalization);
    assert_eq!(first.code(), second.code());
}

#[test]
fn given_provisional_arguments_when_snapshotted_and_restored_then_they_are_kept_and_still_apply() {
    let draft = run(vec![call_with(Some(json!({"q": "provisional"})))]);
    let snapshot = draft.to_snapshot().unwrap();
    let reloaded = DraftSnapshot::from_json(&snapshot.to_json().unwrap()).unwrap();
    let mut restored = ResponseDraft::from_snapshot(&reloaded).unwrap();

    assert_eq!(restored, draft);
    assert_eq!(
        restored
            .view()
            .slot(bid(10))
            .unwrap()
            .provisional_arguments(),
        Some(&oj(json!({"q": "provisional"})))
    );
    ok(&mut restored, finalize(10));
    assert_eq!(arguments_of(&restored), json!({"q": "provisional"}));
}

#[test]
fn given_a_snapshot_row_whose_provisional_runtime_arguments_are_not_an_object_when_loaded_then_refused()
 {
    let draft = run(vec![call_with(Some(json!({"q": 1})))]);
    let mut document = draft.to_snapshot().unwrap().to_value().unwrap().raw();
    document["blocks"][0]["payload"]["provisional_arguments"] = json!([1]);

    let error = DraftSnapshot::from_value(oj(document)).unwrap_err();

    assert_eq!(error.code(), ErrorCode::InvalidContent);
}

#[test]
fn given_accumulated_and_authoritative_finalization_when_applied_then_the_outcome_reports_replacement()
 {
    let mut draft = run(vec![start_text(10), start_text(11)]);
    ok(&mut draft, stream_support::text(10, "a"));
    ok(&mut draft, stream_support::text(11, "b"));

    let accumulated = ok(&mut draft, finalize(10));
    let authoritative = ok(
        &mut draft,
        DeltaOp::block_finalized(
            bid(11),
            Finalization::authoritative(text_block(11, "B").into()),
        ),
    );

    assert!(matches!(
        accumulated,
        DeltaOutcome::Finalized {
            replaced: false,
            ..
        }
    ));
    assert!(matches!(
        authoritative,
        DeltaOutcome::Finalized { replaced: true, .. }
    ));
    assert_eq!(
        draft.view().slot(bid(11)).unwrap().final_block(),
        Some(&text_block(11, "B").into())
    );
}

#[test]
fn given_a_provider_execution_record_when_finalized_from_accumulated_state_then_it_equals_direct_construction()
 {
    let mut draft = run(vec![
        start_record(10),
        DeltaOp::block_metadata_updated(
            bid(10),
            br_llm_messages::BlockMetadataUpdate::operation_set(operation("code_run")),
        ),
        DeltaOp::block_metadata_updated(
            bid(10),
            br_llm_messages::BlockMetadataUpdate::execution_status_set(ExecutionStatus::Completed),
        ),
        DeltaOp::replay_material_set(bid(10), signature("opaque-log")),
    ]);

    let outcome = ok(&mut draft, finalize(10));

    let direct = ProviderToolExecutionRecord::new(bid(10))
        .with_operation(operation("code_run"))
        .with_status(ExecutionStatus::Completed)
        .with_replay(signature("opaque-log"));
    assert!(matches!(
        outcome,
        DeltaOutcome::Finalized {
            replaced: false,
            ..
        }
    ));
    let built = draft.view().slot(bid(10)).unwrap().final_block().cloned();
    assert_eq!(built, Some(direct.clone().into()));
    assert_eq!(direct.exposure(), ResultExposure::Opaque);
}

#[test]
fn given_a_runtime_call_and_a_provider_call_built_from_streams_when_compared_then_they_equal_direct_construction()
 {
    let mut draft = run(vec![
        call_with(None),
        json(10, "{\"q\":\"x\"}"),
        start_provider_call(11),
    ]);
    ok(&mut draft, finalize(10));
    ok(&mut draft, finalize(11));

    let runtime = RuntimeToolCall::new(bid(10), tool("lookup"), oj(json!({"q": "x"}))).unwrap();
    let provider = ProviderToolCall::new(bid(11)).with_operation(operation("web_search"));
    let view = draft.view();
    assert_eq!(
        view.slot(bid(10)).unwrap().final_block(),
        Some(&ContentBlock::from(runtime))
    );
    assert_eq!(
        view.slot(bid(11)).unwrap().final_block(),
        Some(&ContentBlock::from(provider))
    );
}
