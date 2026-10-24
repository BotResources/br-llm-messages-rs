mod common;
mod stream_support;

use br_llm_messages::{
    BlockMetadataUpdate, DeltaOp, ErrorCode, ExecutionStatus, ExternalId, ModelName,
    ResponseMetadataUpdate, Usage,
};
use common::{bid, tool};
use stream_support::{
    citation, draft, feed, finalize, finish, json, operation, refuse, run, start_call,
    start_provider_call, start_record, start_text, start_thinking, text,
};

fn update(update: ResponseMetadataUpdate) -> DeltaOp {
    DeltaOp::response_metadata_updated(update)
}

#[test]
fn given_two_usage_snapshots_when_applied_then_the_last_replaces_and_nothing_is_added() {
    let mut draft = draft();
    let first = Usage::new().with_input_tokens(10).with_output_tokens(4);
    let second = Usage::new().with_output_tokens(9);

    feed(
        &mut draft,
        vec![
            update(ResponseMetadataUpdate::new().with_usage(first)),
            update(ResponseMetadataUpdate::new().with_usage(second.clone())),
        ],
    );

    assert_eq!(draft.metadata().usage(), Some(&second));
    assert_eq!(draft.metadata().usage().unwrap().input_tokens(), None);
}

#[test]
fn given_model_and_provider_id_updates_when_applied_then_they_replace_and_untouched_fields_stay() {
    let mut draft = draft();
    let usage = Usage::new().with_input_tokens(1);

    feed(
        &mut draft,
        vec![
            update(
                ResponseMetadataUpdate::new()
                    .with_usage(usage.clone())
                    .with_model(ModelName::new("a").unwrap())
                    .with_provider_response_id(ExternalId::new("r1").unwrap()),
            ),
            update(ResponseMetadataUpdate::new().with_model(ModelName::new("b").unwrap())),
            update(ResponseMetadataUpdate::new()),
        ],
    );

    assert_eq!(draft.metadata().model().unwrap().as_str(), "b");
    assert_eq!(
        draft.metadata().provider_response_id().unwrap().as_str(),
        "r1"
    );
    assert_eq!(draft.metadata().usage(), Some(&usage));
}

#[test]
fn given_updated_metadata_when_completed_then_the_response_carries_it() {
    let usage = Usage::new().with_input_tokens(3).with_output_tokens(2);
    let draft = run(vec![
        update(ResponseMetadataUpdate::new().with_usage(usage.clone())),
        update(ResponseMetadataUpdate::new().with_model(ModelName::new("m").unwrap())),
        finish(),
    ]);

    let response = draft.response().unwrap();
    assert_eq!(response.usage(), Some(&usage));
    assert_eq!(response.model().unwrap().as_str(), "m");
    assert!(response.provider_response_id().is_none());
}

#[test]
fn given_a_text_block_when_citations_are_added_then_they_are_kept_in_order() {
    let mut draft = run(vec![start_text(10), text(10, "alpha beta")]);

    feed(
        &mut draft,
        vec![
            DeltaOp::block_metadata_updated(
                bid(10),
                BlockMetadataUpdate::citation_added(citation(50, 0, 5)),
            ),
            DeltaOp::block_metadata_updated(
                bid(10),
                BlockMetadataUpdate::citation_added(citation(51, 6, 10)),
            ),
        ],
    );

    let slot = draft.view().slot(bid(10)).unwrap();
    let ids: Vec<_> = slot.citations().iter().map(|c| c.id()).collect();
    assert_eq!(ids, vec![common::cid(50), common::cid(51)]);
}

#[test]
fn given_a_known_citation_id_when_added_again_then_reused_identity() {
    let mut draft = run(vec![
        start_text(10),
        start_text(11),
        DeltaOp::block_metadata_updated(
            bid(10),
            BlockMetadataUpdate::citation_added(citation(50, 0, 1)),
        ),
    ]);
    let again = BlockMetadataUpdate::citation_added(citation(50, 0, 1));

    let same_block = refuse(
        &mut draft,
        DeltaOp::block_metadata_updated(bid(10), again.clone()),
    );
    let other_block = refuse(&mut draft, DeltaOp::block_metadata_updated(bid(11), again));

    assert_eq!(same_block.code(), ErrorCode::ReusedIdentity);
    assert_eq!(other_block.code(), ErrorCode::ReusedIdentity);
    assert_eq!(other_block.object_id(), Some(bid(11).as_uuid()));
}

#[test]
fn given_a_citation_id_equal_to_a_block_id_when_added_then_reused_identity() {
    let mut draft = run(vec![start_text(10), start_text(11)]);
    let clash = BlockMetadataUpdate::citation_added(citation(11, 0, 1));

    let error = refuse(&mut draft, DeltaOp::block_metadata_updated(bid(10), clash));

    assert_eq!(error.code(), ErrorCode::ReusedIdentity);
}

#[test]
fn given_a_citation_update_on_a_non_text_block_when_applied_then_stream_target() {
    let mut draft = run(vec![start_thinking(10), start_call(11, None, None)]);

    for block in [10, 11] {
        let error = refuse(
            &mut draft,
            DeltaOp::block_metadata_updated(
                bid(block),
                BlockMetadataUpdate::citation_added(citation(50, 0, 1)),
            ),
        );
        assert_eq!(error.code(), ErrorCode::StreamTargetError);
    }
}

#[test]
fn given_a_provisional_call_when_identified_later_then_name_and_external_id_are_replaced() {
    let mut draft = run(vec![start_call(10, None, None)]);

    feed(
        &mut draft,
        vec![
            DeltaOp::block_metadata_updated(
                bid(10),
                BlockMetadataUpdate::tool_name_set(tool("draft_name")),
            ),
            DeltaOp::block_metadata_updated(
                bid(10),
                BlockMetadataUpdate::tool_name_set(tool("real_name")),
            ),
            DeltaOp::block_metadata_updated(
                bid(10),
                BlockMetadataUpdate::external_id_set(ExternalId::new("toolu_8").unwrap()),
            ),
            DeltaOp::block_metadata_updated(
                bid(10),
                BlockMetadataUpdate::external_id_set(ExternalId::new("toolu_9").unwrap()),
            ),
        ],
    );

    let slot = draft.view().slot(bid(10)).unwrap();
    assert_eq!(slot.name().unwrap().as_str(), "real_name");
    assert_eq!(slot.external_id().unwrap().as_str(), "toolu_9");
}

#[test]
fn given_provider_activity_when_references_and_status_update_while_open_then_all_are_observed() {
    let mut draft = run(vec![start_provider_call(10), start_record(11)]);

    feed(
        &mut draft,
        vec![
            DeltaOp::block_metadata_updated(
                bid(10),
                BlockMetadataUpdate::operation_set(operation("code_run")),
            ),
            DeltaOp::block_metadata_updated(
                bid(10),
                BlockMetadataUpdate::external_id_set(ExternalId::new("srv_1").unwrap()),
            ),
            DeltaOp::block_metadata_updated(
                bid(11),
                BlockMetadataUpdate::operation_set(operation("code_run")),
            ),
            DeltaOp::block_metadata_updated(
                bid(11),
                BlockMetadataUpdate::execution_status_set(ExecutionStatus::Pending),
            ),
            DeltaOp::block_metadata_updated(
                bid(11),
                BlockMetadataUpdate::execution_status_set(ExecutionStatus::Running),
            ),
        ],
    );

    let view = draft.view();
    let call = view.slot(bid(10)).unwrap();
    let record = view.slot(bid(11)).unwrap();
    assert_eq!(call.operation().unwrap().as_str(), "code_run");
    assert_eq!(call.external_id().unwrap().as_str(), "srv_1");
    assert_eq!(record.operation().unwrap().as_str(), "code_run");
    assert_eq!(record.status(), Some(ExecutionStatus::Running));
}

#[test]
fn given_a_finalized_block_when_metadata_arrives_then_stream_phase() {
    let mut draft = run(vec![
        start_call(10, Some("t"), None),
        json(10, "{}"),
        finalize(10),
    ]);

    let error = refuse(
        &mut draft,
        DeltaOp::block_metadata_updated(bid(10), BlockMetadataUpdate::tool_name_set(tool("other"))),
    );

    assert_eq!(error.code(), ErrorCode::StreamPhaseError);
}
