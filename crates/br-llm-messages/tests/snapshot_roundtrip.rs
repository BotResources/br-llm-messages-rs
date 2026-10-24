mod common;
mod snapshot_support;
mod stream_support;

use br_llm_messages::{
    DeltaOp, DraftPhase, ErrorCode, InterruptionReason, RepresentationState, ResponseDraft,
    SlotLifecycle,
};
use common::Raw as _;
use common::bid;
use serde_json::json;
use snapshot_support::{deltas, draft_after, script};
use stream_support::delta;

const SECRET: &str = "SECRET_VISIBLE_TEXT";

fn mid_script() -> ResponseDraft {
    draft_after(&deltas(&script()), 20)
}

#[test]
fn given_a_producing_draft_when_restored_then_ids_order_fragments_and_phase_are_equal() {
    let draft = mid_script();

    let restored = ResponseDraft::from_snapshot(&draft.to_snapshot().unwrap()).unwrap();

    assert_eq!(restored, draft);
    assert!(matches!(restored.phase(), DraftPhase::Producing { .. }));
    let slots = restored.view().slots();
    let ids: Vec<_> = slots.iter().map(|slot| slot.block()).collect();
    let declared: Vec<_> = [11, 17, 12, 10, 13, 15, 16].into_iter().map(bid).collect();
    assert_eq!(ids, declared);
    assert_eq!(slots[2].raw_json(), Some("{\"query\":\"rust\"}"));
    assert_eq!(slots[3].text(), Some("Hello w\u{f6}rld"));
    assert_eq!(slots[4].raw_json(), Some("{\"a\":[1,"));
}

#[test]
fn given_finalized_and_open_blocks_when_restored_then_lifecycles_are_kept_and_identities_stay_used()
{
    let steps = deltas(&script());
    let draft = draft_after(&steps, 31);

    let mut restored = ResponseDraft::from_snapshot(&draft.to_snapshot().unwrap()).unwrap();

    assert_eq!(restored, draft);
    let lifecycles: Vec<_> = restored
        .view()
        .slots()
        .iter()
        .map(|slot| slot.lifecycle())
        .collect();
    assert_eq!(lifecycles[..6], [SlotLifecycle::Final; 6]);
    assert_eq!(lifecycles[6], SlotLifecycle::Open);
    let reuse = delta(DeltaOp::block_started(
        bid(10),
        br_llm_messages::BlockStart::text(),
    ));
    assert_eq!(
        restored.apply(&reuse).unwrap_err().code(),
        ErrorCode::ReusedIdentity
    );
}

#[test]
fn given_late_replay_and_metadata_when_restored_then_they_are_exact() {
    let steps = deltas(&script());
    let draft = draft_after(&steps, 25);

    let restored = ResponseDraft::from_snapshot(&draft.to_snapshot().unwrap()).unwrap();

    let view = restored.view();
    let text = &view.slots()[3];
    let replay = text.replay().expect("late replay is kept");
    assert_eq!(
        replay.payload(),
        &br_llm_messages::ReplayPayload::text("Ab")
    );
    assert_eq!(view.metadata().usage().unwrap().input_tokens(), Some(3));
    assert_eq!(view.metadata().model().unwrap().as_str(), "m");
}

#[test]
fn given_an_interrupted_draft_when_restored_then_phase_and_partial_content_are_equal() {
    let mut draft = mid_script();
    let reason = InterruptionReason::failed()
        .with_detail("upstream_closed")
        .unwrap();
    draft
        .apply(&delta(DeltaOp::response_interrupted(reason.clone())))
        .unwrap();

    let snapshot = draft.to_snapshot().unwrap();
    let mut restored = ResponseDraft::from_snapshot(&snapshot).unwrap();

    assert_eq!(restored, draft);
    assert!(matches!(
        restored.phase(),
        DraftPhase::Interrupted { reason: found, .. } if *found == reason
    ));
    assert!(restored.response().is_none());
    assert_eq!(snapshot.entry().state(), RepresentationState::Interrupted);
    let later = delta(DeltaOp::text_appended(bid(10), "more"));
    assert_eq!(
        restored.apply(&later).unwrap_err().code(),
        ErrorCode::StreamPhaseError
    );
}

#[test]
fn given_a_producing_snapshot_when_built_then_the_header_holds_no_block_content() {
    let mut draft = mid_script();
    draft
        .apply(&delta(DeltaOp::text_appended(bid(10), SECRET)))
        .unwrap();

    let snapshot = draft.to_snapshot().unwrap();

    let header = snapshot.entry().payload().raw();
    assert!(!header.to_string().contains(SECRET));
    let allowed = ["usage", "model", "provider_response_id", "interruption"];
    assert!(
        header
            .as_object()
            .unwrap()
            .keys()
            .all(|key| allowed.contains(&key.as_str()))
    );
    let rows: Vec<String> = snapshot
        .blocks()
        .iter()
        .map(|row| row.payload().raw().to_string())
        .collect();
    assert_eq!(rows.iter().filter(|row| row.contains(SECRET)).count(), 1);
}

#[test]
fn given_an_interrupted_snapshot_when_built_then_raw_json_and_late_replay_stay_verbatim() {
    let steps = deltas(&script());
    let mut draft = draft_after(&steps, 25);
    draft
        .apply(&delta(DeltaOp::response_interrupted(
            InterruptionReason::truncated(),
        )))
        .unwrap();

    let snapshot = draft.to_snapshot().unwrap();

    let row = |n: u64| {
        snapshot
            .blocks()
            .iter()
            .find(|row| row.id() == bid(n))
            .unwrap()
    };
    assert_eq!(row(12).state(), RepresentationState::Interrupted);
    assert_eq!(
        row(12).payload().raw()["raw_arguments"],
        "{\"query\":\"rust\"}"
    );
    assert_eq!(row(13).payload().raw()["raw_json"], "{\"a\":[1,2]}");
    assert_eq!(row(10).payload().raw()["replay"]["payload"]["value"], "Ab");
    assert_eq!(row(15).payload().raw()["raw_arguments"], "{\"q\":\"x\"}");
    assert_eq!(
        snapshot.entry().payload().raw()["interruption"],
        json!({"kind": "truncated"})
    );
}

#[test]
fn given_an_interrupted_draft_when_snapshotted_then_complete_blocks_stay_complete() {
    let steps = deltas(&script());
    let mut draft = draft_after(&steps, 30);
    draft
        .apply(&delta(DeltaOp::response_interrupted(
            InterruptionReason::cancelled(),
        )))
        .unwrap();

    let snapshot = draft.to_snapshot().unwrap();

    let state = |n: u64| {
        snapshot
            .blocks()
            .iter()
            .find(|row| row.id() == bid(n))
            .unwrap()
            .state()
    };
    assert_eq!(state(10), RepresentationState::Complete);
    assert_eq!(state(16), RepresentationState::Interrupted);
    assert_eq!(ResponseDraft::from_snapshot(&snapshot).unwrap(), draft);
}
