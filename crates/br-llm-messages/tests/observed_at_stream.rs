mod common;
mod snapshot_support;
mod stream_support;
use common::Raw as _;

use br_llm_messages::{
    BlockStart, ContentBlock, DeltaOp, ErrorCode, Finalization, ResponseDelta, ResponseDraft,
    ResponseEnding, ResponseMetadata, ResponseMetadataUpdate, SlotLifecycle, StructuredBlock,
    Usage,
};
use common::{at, bid, oj, text_block};
use serde_json::json;
use stream_support::{delta, feed, finish, ok, refuse, text};

fn started(draft_time: u32) -> ResponseDraft {
    stream_support::started(ResponseMetadata::new().with_observed_at(at(draft_time)))
}

fn stamped_start(block: u64, start: BlockStart, time: u32) -> DeltaOp {
    DeltaOp::block_started(bid(block), start.with_observed_at(at(time)))
}

#[test]
fn given_a_response_started_with_a_time_when_completed_then_the_response_carries_it_and_equals_direct_construction()
 {
    let mut draft = started(1);
    feed(
        &mut draft,
        vec![
            stamped_start(10, BlockStart::text(), 2),
            text(10, "hello"),
            stream_support::finalize(10),
            finish(),
        ],
    );

    let response = draft.into_response().unwrap();

    let direct = br_llm_messages::ModelResponse::new(
        stream_support::delta(finish()).entry(),
        vec![text_block(10, "hello").with_observed_at(at(2)).into()],
        ResponseEnding::finished(),
        ResponseMetadata::new().with_observed_at(at(1)),
    )
    .unwrap();
    assert_eq!(response.observed_at(), Some(&at(1)));
    assert_eq!(response, direct);
}

#[test]
fn given_every_progressively_exposed_block_kind_when_started_with_a_time_then_the_finalized_block_keeps_it()
 {
    let mut draft = started(1);
    feed(
        &mut draft,
        vec![
            stamped_start(10, BlockStart::text(), 10),
            stamped_start(11, BlockStart::thinking(), 11),
            stamped_start(12, BlockStart::structured_content(), 12),
            stamped_start(
                13,
                BlockStart::runtime_tool_call(Some(common::tool("lookup")), None),
                13,
            ),
            stamped_start(14, BlockStart::provider_tool_call(None, None), 14),
            text(10, "a"),
            DeltaOp::thinking_appended(bid(11), "plan"),
            stream_support::json(12, "{\"k\":1}"),
            stream_support::json(13, "{}"),
        ],
    );
    let open: Vec<_> = draft
        .view()
        .slots()
        .iter()
        .map(|slot| (slot.lifecycle(), slot.observed_at().cloned()))
        .collect();
    assert!(
        open.iter().enumerate().all(
            |(index, (lifecycle, time))| *lifecycle == SlotLifecycle::Open
                && time.as_ref() == Some(&at(10 + index as u32))
        )
    );

    feed(
        &mut draft,
        (10..=14).map(stream_support::finalize).collect(),
    );
    let finished: Vec<_> = draft
        .view()
        .slots()
        .iter()
        .map(|slot| slot.observed_at().cloned())
        .collect();
    assert_eq!(
        finished,
        (10..=14).map(|time| Some(at(time))).collect::<Vec<_>>()
    );
    ok(&mut draft, finish());
    let response = draft.into_response().unwrap();
    let times: Vec<_> = response
        .blocks()
        .iter()
        .map(|block| block.observed_at().cloned())
        .collect();
    assert_eq!(
        times,
        (10..=14).map(|time| Some(at(time))).collect::<Vec<_>>()
    );
}

#[test]
fn given_a_block_started_without_a_time_when_finalized_then_it_has_none() {
    let mut draft = started(1);
    feed(
        &mut draft,
        vec![
            stream_support::start_text(10),
            text(10, "x"),
            stream_support::finalize(10),
        ],
    );

    assert_eq!(draft.view().slot(bid(10)).unwrap().observed_at(), None);
}

#[test]
fn given_a_complete_block_inserted_when_the_draft_completes_then_it_keeps_its_own_time() {
    let mut draft = started(1);
    feed(
        &mut draft,
        vec![
            DeltaOp::block_inserted(common::png(10).with_observed_at(at(20))),
            DeltaOp::block_inserted(StructuredBlock::new(bid(11), oj(json!({"a": 1}))).unwrap()),
            finish(),
        ],
    );

    let response = draft.into_response().unwrap();

    assert_eq!(response.blocks()[0].observed_at(), Some(&at(20)));
    assert_eq!(response.blocks()[1].observed_at(), None);
}

#[test]
fn given_an_authoritative_block_when_finalizing_then_its_time_replaces_the_started_one_whole() {
    let mut draft = started(1);
    feed(
        &mut draft,
        vec![
            stamped_start(10, BlockStart::text(), 5),
            stamped_start(11, BlockStart::text(), 6),
            text(10, "x"),
            text(11, "y"),
            DeltaOp::block_finalized(
                bid(10),
                Finalization::authoritative(ContentBlock::Text(text_block(10, "final"))),
            ),
            DeltaOp::block_finalized(
                bid(11),
                Finalization::authoritative(ContentBlock::Text(
                    text_block(11, "final").with_observed_at(at(30)),
                )),
            ),
        ],
    );

    assert_eq!(draft.view().slot(bid(10)).unwrap().observed_at(), None);
    assert_eq!(
        draft.view().slot(bid(11)).unwrap().observed_at(),
        Some(&at(30))
    );
}

#[test]
fn given_a_metadata_update_when_applied_then_the_response_time_stays() {
    let mut draft = started(1);

    ok(
        &mut draft,
        DeltaOp::response_metadata_updated(
            ResponseMetadataUpdate::new().with_usage(Usage::of_input_tokens(3)),
        ),
    );

    assert_eq!(draft.metadata().observed_at(), Some(&at(1)));
    assert_eq!(draft.metadata().usage().unwrap().input_tokens(), Some(3));
}

#[test]
fn given_deltas_with_times_when_serialized_then_the_time_sits_in_the_payload_and_loads_back() {
    let response = delta(DeltaOp::response_started(
        ResponseMetadata::new().with_observed_at(at(1)),
    ));
    let block = delta(stamped_start(10, BlockStart::text(), 2));

    let response_wire = response.to_value().unwrap().raw();
    let block_wire = block.to_value().unwrap().raw();

    assert_eq!(response_wire["metadata"]["observed_at"], at(1).as_str());
    assert_eq!(block_wire["start"]["observed_at"], at(2).as_str());
    assert_eq!(
        ResponseDelta::from_value(oj(response_wire)).unwrap(),
        response
    );
    assert_eq!(ResponseDelta::from_value(oj(block_wire)).unwrap(), block);
}

#[test]
fn given_a_bad_time_in_a_delta_when_loaded_then_invalid_content() {
    let response = delta(DeltaOp::response_started(ResponseMetadata::new()))
        .to_value()
        .unwrap()
        .raw();
    let block = delta(stamped_start(10, BlockStart::text(), 2))
        .to_value()
        .unwrap()
        .raw();
    let inserted = delta(DeltaOp::block_inserted(
        common::png(10).with_observed_at(at(3)),
    ))
    .to_value()
    .unwrap()
    .raw();

    for bad in [
        "2026-10-23T08:00:00+01:00",
        "2026-10-23t08:00:00Z",
        "2026-13-23T08:00:00Z",
    ] {
        let mut metadata = response.clone();
        metadata["metadata"]["observed_at"] = json!(bad);
        let mut start = block.clone();
        start["start"]["observed_at"] = json!(bad);
        let mut insert = inserted.clone();
        insert["content"]["observed_at"] = json!(bad);
        for document in [metadata, start, insert] {
            let error = ResponseDelta::from_value(oj(document)).unwrap_err();
            assert_eq!(error.code(), ErrorCode::InvalidContent, "{bad}");
        }
    }
}

#[test]
fn given_a_refused_delta_when_the_draft_has_times_then_the_draft_is_unchanged() {
    let mut draft = started(1);
    ok(&mut draft, stamped_start(10, BlockStart::text(), 2));

    let error = refuse(
        &mut draft,
        DeltaOp::response_started(ResponseMetadata::new()),
    );

    assert_eq!(error.code(), ErrorCode::StreamPhaseError);
    assert_eq!(draft.metadata().observed_at(), Some(&at(1)));
}
