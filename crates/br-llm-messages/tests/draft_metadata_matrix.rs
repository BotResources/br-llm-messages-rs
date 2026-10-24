mod common;
mod stream_support;

use br_llm_messages::{
    BlockMetadataUpdate, BlockStart, DeltaOp, ErrorCode, ExecutionStatus, ExternalId, ItemRef,
};
use common::{bid, tool};
use stream_support::{citation, ok, operation, refuse, run, start_json, start_text};

const TEXT: u64 = 10;
const THINKING: u64 = 11;
const REDACTED: u64 = 12;
const STRUCTURED: u64 = 13;
const RUNTIME_CALL: u64 = 14;
const PROVIDER_CALL: u64 = 15;
const PROVIDER_RESULT: u64 = 16;
const RECORD: u64 = 17;
const ALL: [u64; 8] = [
    TEXT,
    THINKING,
    REDACTED,
    STRUCTURED,
    RUNTIME_CALL,
    PROVIDER_CALL,
    PROVIDER_RESULT,
    RECORD,
];

fn every_kind_open() -> br_llm_messages::ResponseDraft {
    run(vec![
        start_text(TEXT),
        stream_support::start_thinking(THINKING),
        DeltaOp::block_started(bid(REDACTED), BlockStart::redacted_thinking()),
        start_json(STRUCTURED),
        stream_support::start_call(RUNTIME_CALL, None, None),
        stream_support::start_provider_call(PROVIDER_CALL),
        DeltaOp::block_started(
            bid(PROVIDER_RESULT),
            BlockStart::provider_tool_result(bid(99), None),
        ),
        stream_support::start_record(RECORD),
    ])
}

fn matrix() -> Vec<(&'static str, BlockMetadataUpdate, Vec<u64>)> {
    vec![
        (
            "tool name",
            BlockMetadataUpdate::tool_name_set(tool("t")),
            vec![RUNTIME_CALL],
        ),
        (
            "external id",
            BlockMetadataUpdate::external_id_set(ExternalId::new("e").unwrap()),
            vec![RUNTIME_CALL, PROVIDER_CALL, PROVIDER_RESULT, RECORD],
        ),
        (
            "item reference",
            BlockMetadataUpdate::item_ref_set(ItemRef::new("i").unwrap()),
            vec![TEXT, RUNTIME_CALL, PROVIDER_CALL],
        ),
        (
            "citation",
            BlockMetadataUpdate::citation_added(citation(50, 0, 1)),
            vec![TEXT],
        ),
        (
            "operation",
            BlockMetadataUpdate::operation_set(operation("op")),
            vec![PROVIDER_CALL, RECORD],
        ),
        (
            "execution status",
            BlockMetadataUpdate::execution_status_set(ExecutionStatus::Running),
            vec![RECORD],
        ),
    ]
}

#[test]
fn given_each_single_purpose_update_when_applied_to_every_open_kind_then_only_its_kinds_accept_it()
{
    for (name, update, allowed) in matrix() {
        for block in ALL {
            let mut draft = every_kind_open();
            let op = DeltaOp::block_metadata_updated(bid(block), update.clone());
            if allowed.contains(&block) {
                ok(&mut draft, op);
            } else {
                let error = refuse(&mut draft, op);
                assert_eq!(
                    error.code(),
                    ErrorCode::StreamTargetError,
                    "{name} on {block}"
                );
                assert_eq!(error.object_id(), Some(bid(block).as_uuid()));
            }
        }
    }
}

#[test]
fn given_an_item_reference_set_while_open_when_finalized_then_the_block_carries_it() {
    let mut draft = run(vec![
        start_text(TEXT),
        stream_support::text(TEXT, "x"),
        DeltaOp::block_metadata_updated(
            bid(TEXT),
            BlockMetadataUpdate::item_ref_set(ItemRef::new("msg_1").unwrap()),
        ),
    ]);
    assert_eq!(
        draft.view().slot(bid(TEXT)).unwrap().item_ref(),
        Some(&ItemRef::new("msg_1").unwrap())
    );

    ok(&mut draft, stream_support::finalize(TEXT));

    assert_eq!(
        draft.view().slot(bid(TEXT)).unwrap().item_ref(),
        Some(&ItemRef::new("msg_1").unwrap())
    );
}
