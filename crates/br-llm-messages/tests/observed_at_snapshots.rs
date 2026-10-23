mod common;
mod stream_support;
use common::Raw as _;

use br_llm_messages::{
    BlockMetadataUpdate, BlockStart, DeltaOp, DraftSnapshot, ErrorCode, InterruptionReason,
    ObservedAt, ResponseDraft, ResponseEnding, ResponseMetadata,
};
use common::{at, bid, oj};
use serde_json::json;
use stream_support::{delta, feed, ok, text};

fn stamped_start(block: u64, position: u32, start: BlockStart, time: u32) -> DeltaOp {
    DeltaOp::block_started(bid(block), position, start.with_observed_at(at(time)))
}

fn mid_stream(time: Option<ObservedAt>) -> (ResponseDraft, Vec<DeltaOp>) {
    let metadata = match time {
        Some(time) => ResponseMetadata::new().with_observed_at(time),
        None => ResponseMetadata::new(),
    };
    let mut draft = ResponseDraft::begin(&delta(DeltaOp::response_started(metadata))).unwrap();
    feed(
        &mut draft,
        vec![
            stamped_start(10, 0, BlockStart::text(), 2),
            stamped_start(11, 1, BlockStart::runtime_tool_call(None, None), 3),
            DeltaOp::block_inserted(2, common::png(12).with_observed_at(at(4))),
            text(10, "he"),
            stream_support::json(11, "{\"a\":"),
            DeltaOp::block_metadata_updated(
                bid(11),
                BlockMetadataUpdate::runtime_tool_call_identified(
                    Some(common::tool("lookup")),
                    None,
                ),
            ),
        ],
    );
    let rest = vec![
        text(10, "llo"),
        stream_support::json(11, "1}"),
        stream_support::finalize(10),
        stream_support::finalize(11),
        complete_tool_use(),
    ];
    (draft, rest)
}

fn complete_tool_use() -> DeltaOp {
    DeltaOp::response_completed(ResponseEnding::tool_use())
}

#[test]
fn given_a_producing_draft_with_times_when_snapshotted_and_restored_then_everything_is_equal_and_the_rest_of_the_stream_completes_the_same()
 {
    let (draft, rest) = mid_stream(Some(at(1)));

    let snapshot = draft.to_snapshot().unwrap();
    let reloaded = DraftSnapshot::from_json(&snapshot.to_json().unwrap()).unwrap();
    let mut restored = ResponseDraft::from_snapshot(&reloaded).unwrap();
    let mut uninterrupted = draft.clone();

    assert_eq!(reloaded, snapshot);
    assert_eq!(restored, draft);
    assert_eq!(snapshot.entry().observed_at(), Some(&at(1)));
    let rows: Vec<_> = snapshot
        .blocks()
        .iter()
        .map(|row| (row.id(), row.observed_at().cloned()))
        .collect();
    assert_eq!(
        rows,
        [
            (bid(10), Some(at(2))),
            (bid(11), Some(at(3))),
            (bid(12), Some(at(4)))
        ]
    );
    let wire = snapshot.to_value().unwrap().raw();
    assert!(wire["entry"]["payload"].get("observed_at").is_none());
    feed(&mut restored, rest.clone());
    feed(&mut uninterrupted, rest);
    assert_eq!(restored, uninterrupted);
    let response = restored.into_response().unwrap();
    assert_eq!(response.observed_at(), Some(&at(1)));
    assert_eq!(response.blocks()[0].observed_at(), Some(&at(2)));
    assert_eq!(response.blocks()[1].observed_at(), Some(&at(3)));
    assert_eq!(response.blocks()[2].observed_at(), Some(&at(4)));
}

#[test]
fn given_an_interrupted_draft_with_times_when_snapshotted_and_restored_then_the_times_are_retained()
{
    let (mut draft, _) = mid_stream(Some(at(1)));
    ok(
        &mut draft,
        DeltaOp::response_interrupted(InterruptionReason::cancelled()),
    );

    let snapshot = draft.to_snapshot().unwrap();
    let restored = ResponseDraft::from_snapshot(
        &DraftSnapshot::from_json(&snapshot.to_json().unwrap()).unwrap(),
    )
    .unwrap();

    assert_eq!(restored, draft);
    assert_eq!(snapshot.entry().observed_at(), Some(&at(1)));
    assert_eq!(snapshot.blocks()[0].observed_at(), Some(&at(2)));
    assert_eq!(restored.metadata().observed_at(), Some(&at(1)));
    assert_eq!(
        restored.view().slot(bid(11)).unwrap().observed_at(),
        Some(&at(3))
    );
}

#[test]
fn given_a_draft_without_times_when_snapshotted_then_no_time_appears_anywhere() {
    let mut draft =
        ResponseDraft::begin(&delta(DeltaOp::response_started(ResponseMetadata::new()))).unwrap();
    feed(
        &mut draft,
        vec![stream_support::start_text(10, 0), text(10, "x")],
    );

    let wire = draft.to_snapshot().unwrap().to_json().unwrap();

    assert!(!wire.contains("observed_at"));
}

#[test]
fn given_a_snapshot_with_a_bad_time_when_loaded_then_invalid_content_for_the_entry_and_every_block()
{
    let (draft, _) = mid_stream(Some(at(1)));
    let wire = draft.to_snapshot().unwrap().to_value().unwrap().raw();
    let bad = json!("2026-10-23T08:00:00+01:00");
    let mut broken = vec![{
        let mut entry = wire.clone();
        entry["entry"]["observed_at"] = bad.clone();
        entry
    }];
    for index in 0..3 {
        let mut block = wire.clone();
        block["blocks"][index]["observed_at"] = bad.clone();
        broken.push(block);
    }

    for document in broken {
        let error = DraftSnapshot::from_value(oj(document)).unwrap_err();
        assert_eq!(error.code(), ErrorCode::InvalidContent);
    }
}

#[test]
fn given_a_time_inside_a_partial_payload_when_loaded_then_it_is_refused() {
    let (draft, _) = mid_stream(None);
    let mut wire = draft.to_snapshot().unwrap().to_value().unwrap().raw();
    wire["blocks"][0]["payload"]["observed_at"] = json!(at(1).as_str());

    let error = DraftSnapshot::from_value(oj(wire)).unwrap_err();

    assert_eq!(error.code(), ErrorCode::InvalidContent);
}
