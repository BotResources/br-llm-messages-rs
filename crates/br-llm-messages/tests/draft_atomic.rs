mod common;
mod stream_support;

use std::collections::BTreeSet;

use br_llm_messages::{
    BlockMetadataUpdate, DeltaOp, ErrorCode, Finalization, InterruptionReason, ReplayFormat,
    ResponseDelta, ResponseEnding, ResponseMetadata,
};
use common::{bid, call, envelope, png, result, signature, text_block};
use stream_support::{
    citation, complete, delta, finalize, finish, json, ok, refuse, run, start_call, start_json,
    start_provider_call, start_record, start_text, start_thinking, text, tid,
};

fn rich_draft() -> br_llm_messages::ResponseDraft {
    run(vec![
        start_text(10, 0),
        text(10, "hello"),
        DeltaOp::replay_material_set(bid(10), envelope(serde_json::json!({"enc": 1}))),
        DeltaOp::block_metadata_updated(
            bid(10),
            BlockMetadataUpdate::citation_added(citation(50, 0, 2)),
        ),
        start_thinking(11, 1),
        DeltaOp::replay_fragment_appended(
            bid(11),
            ReplayFormat::new("anthropic.signature").unwrap(),
            "sig",
        ),
        start_json(12, 2),
        json(12, "{"),
        start_call(13, 3, Some("lookup"), None),
        DeltaOp::block_inserted(4, text_block(14, "final")),
        start_text(15, 5),
        DeltaOp::block_discarded(bid(15)),
        start_record(16, 6),
        start_provider_call(17, 7),
    ])
}

fn refusals() -> Vec<(DeltaOp, ErrorCode)> {
    let other_format = ReplayFormat::new("openai.reasoning").unwrap();
    vec![
        (
            DeltaOp::response_started(ResponseMetadata::new()),
            ErrorCode::StreamPhaseError,
        ),
        (text(99, "x"), ErrorCode::StreamTargetError),
        (text(11, "x"), ErrorCode::StreamTargetError),
        (
            DeltaOp::thinking_appended(bid(10), "x"),
            ErrorCode::StreamTargetError,
        ),
        (json(10, "{}"), ErrorCode::StreamTargetError),
        (text(14, "x"), ErrorCode::StreamPhaseError),
        (finalize(14), ErrorCode::StreamPhaseError),
        (
            DeltaOp::block_discarded(bid(14)),
            ErrorCode::StreamPhaseError,
        ),
        (text(15, "x"), ErrorCode::StreamPhaseError),
        (finalize(15), ErrorCode::StreamPhaseError),
        (
            DeltaOp::replay_fragment_appended(bid(11), other_format.clone(), "x"),
            ErrorCode::StreamTargetError,
        ),
        (
            DeltaOp::replay_fragment_appended(bid(10), other_format, "x"),
            ErrorCode::StreamTargetError,
        ),
        (
            DeltaOp::replay_material_set(bid(11), signature("x")),
            ErrorCode::StreamTargetError,
        ),
        (
            DeltaOp::replay_material_set(bid(12), signature("x")),
            ErrorCode::StreamTargetError,
        ),
        (
            DeltaOp::block_metadata_updated(
                bid(10),
                BlockMetadataUpdate::citation_added(citation(50, 0, 1)),
            ),
            ErrorCode::ReusedIdentity,
        ),
        (
            DeltaOp::block_metadata_updated(
                bid(12),
                BlockMetadataUpdate::citation_added(citation(51, 0, 1)),
            ),
            ErrorCode::StreamTargetError,
        ),
        (start_text(10, 20), ErrorCode::ReusedIdentity),
        (start_text(30, 0), ErrorCode::InvalidRecordStructure),
        (start_text(30, 5), ErrorCode::InvalidRecordStructure),
        (
            DeltaOp::block_inserted(0, png(30)),
            ErrorCode::InvalidRecordStructure,
        ),
        (
            DeltaOp::block_inserted(9, png(10)),
            ErrorCode::ReusedIdentity,
        ),
        (
            DeltaOp::block_inserted(9, result(30, 13, "lookup")),
            ErrorCode::DisallowedKind,
        ),
        (
            DeltaOp::block_finalized(
                bid(13),
                Finalization::authoritative(text_block(13, "x").into()),
            ),
            ErrorCode::InconsistentFinalization,
        ),
        (
            DeltaOp::block_finalized(
                bid(13),
                Finalization::authoritative(call(31, "lookup").into()),
            ),
            ErrorCode::InconsistentFinalization,
        ),
        (
            DeltaOp::block_discarded(bid(10)),
            ErrorCode::InconsistentFinalization,
        ),
        (
            DeltaOp::block_discarded(bid(11)),
            ErrorCode::InconsistentFinalization,
        ),
        (finalize(12), ErrorCode::MalformedJson),
        (finalize(13), ErrorCode::MalformedJson),
        (finalize(16), ErrorCode::InconsistentFinalization),
        (finish(), ErrorCode::InconsistentFinalization),
    ]
}

#[test]
fn given_any_refused_delta_when_applied_then_the_draft_equals_its_previous_clone_and_names_the_target()
 {
    let mut draft = rich_draft();
    let mut codes = BTreeSet::new();

    for (op, expected) in refusals() {
        let target = op
            .block_id()
            .map_or(draft.entry().as_uuid(), |block| block.as_uuid());

        let error = refuse(&mut draft, op);

        assert_eq!(error.code(), expected);
        assert_eq!(error.object_id(), Some(target));
        codes.insert(expected.as_str());
    }
    assert!(codes.len() >= 7);
}

#[test]
fn given_a_foreign_turn_or_entry_when_applied_then_refused_and_unchanged() {
    let mut draft = rich_draft();
    let before = draft.clone();
    let foreign = [
        ResponseDelta::new(tid(9), draft.entry(), text(10, "x")),
        ResponseDelta::new(draft.turn(), common::eid(9), text(10, "x")),
        ResponseDelta::new(tid(9), draft.entry(), finish()),
    ];

    for delta in foreign {
        let error = draft.apply(&delta).unwrap_err();
        assert_eq!(error.code(), ErrorCode::StreamTargetError);
        assert!(error.object_id().is_some());
        assert_eq!(draft, before);
    }
}

#[test]
fn given_a_completion_that_direct_construction_refuses_when_applied_then_the_draft_is_unchanged() {
    let mut draft = run(vec![start_text(10, 0), text(10, "x"), finalize(10)]);

    let error = refuse(&mut draft, complete(ResponseEnding::tool_use()));

    assert_eq!(error.code(), ErrorCode::InvalidContent);
    assert_eq!(error.object_id(), Some(draft.entry().as_uuid()));
}

#[test]
fn given_a_gap_when_completing_then_refused_unchanged_and_the_gap_can_still_be_filled() {
    let mut draft = run(vec![
        start_text(10, 0),
        text(10, "x"),
        finalize(10),
        start_text(11, 2),
    ]);
    ok(&mut draft, text(11, "z"));
    ok(&mut draft, finalize(11));

    let error = refuse(&mut draft, finish());
    assert_eq!(error.code(), ErrorCode::InvalidRecordStructure);

    ok(&mut draft, DeltaOp::block_inserted(1, text_block(12, "y")));
    ok(&mut draft, finish());
    assert_eq!(draft.response().unwrap().blocks().len(), 3);
}

#[test]
fn given_a_first_delta_that_is_not_a_start_when_beginning_then_the_error_names_the_entry() {
    let error = br_llm_messages::ResponseDraft::begin(&delta(finish())).unwrap_err();

    assert_eq!(error.code(), ErrorCode::StreamPhaseError);
    assert_eq!(error.object_id(), Some(common::eid(2).as_uuid()));
}

#[test]
fn given_a_terminal_draft_when_any_delta_arrives_then_refused_and_unchanged() {
    let mut completed = run(vec![finish()]);
    let mut interrupted = rich_draft();
    ok(
        &mut interrupted,
        DeltaOp::response_interrupted(InterruptionReason::cancelled()),
    );

    for draft in [&mut completed, &mut interrupted] {
        for op in [
            text(10, "x"),
            start_text(40, 40),
            finish(),
            DeltaOp::block_discarded(bid(15)),
        ] {
            assert_eq!(refuse(draft, op).code(), ErrorCode::StreamPhaseError);
        }
    }
}
