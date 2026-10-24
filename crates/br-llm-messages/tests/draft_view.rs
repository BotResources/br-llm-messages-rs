mod common;
mod stream_support;

use br_llm_messages::{
    BlockMetadataUpdate, ContentBlockKind, DeltaOp, DraftPhase, ExecutionStatus, ExternalId,
    InterruptionReason, SlotLifecycle, Usage,
};
use common::{bid, eid, signature, text_block, tool};
use stream_support::{
    draft, feed, finalize, finish, json, metadata, ok, operation, run, start_call,
    start_provider_call, start_record, start_text, start_thinking, started, text, tid,
};

fn busy_draft() -> br_llm_messages::ResponseDraft {
    let mut draft = started(metadata().with_usage(Usage::new().with_input_tokens(5)));
    feed(
        &mut draft,
        vec![
            start_text(10),
            start_thinking(11),
            start_call(12, Some("lookup"), Some("toolu_1")),
            text(10, "partial te"),
            DeltaOp::thinking_appended(bid(11), "hmm"),
            json(12, "{\"q\":"),
            DeltaOp::replay_fragment_appended(
                bid(11),
                br_llm_messages::ReplayFormat::new("anthropic.signature").unwrap(),
                "sig",
            ),
            start_provider_call(13),
            start_record(14),
            DeltaOp::block_metadata_updated(
                bid(14),
                BlockMetadataUpdate::execution_status_set(ExecutionStatus::Running),
            ),
            DeltaOp::block_inserted(text_block(15, "whole")),
        ],
    );
    draft
}

#[test]
fn given_a_busy_draft_when_previewed_then_text_raw_json_order_and_lifecycle_are_exposed() {
    let draft = busy_draft();

    let view = draft.view();
    let slots = view.slots();

    let order: Vec<_> = slots.iter().map(|s| (s.block(), s.kind())).collect();
    assert_eq!(
        order,
        vec![
            (bid(10), ContentBlockKind::Text),
            (bid(11), ContentBlockKind::Thinking),
            (bid(12), ContentBlockKind::RuntimeToolCall),
            (bid(13), ContentBlockKind::ProviderToolCall),
            (bid(14), ContentBlockKind::ProviderToolExecutionRecord),
            (bid(15), ContentBlockKind::Text),
        ]
    );
    assert_eq!(slots[0].text(), Some("partial te"));
    assert_eq!(slots[1].text(), Some("hmm"));
    assert_eq!(slots[1].replay(), Some(&signature("sig")));
    assert_eq!(slots[2].raw_json(), Some("{\"q\":"));
    assert_eq!(slots[5].text(), Some("whole"));
    let lifecycles: Vec<_> = slots.iter().map(|s| s.lifecycle()).collect();
    assert_eq!(
        lifecycles,
        vec![
            SlotLifecycle::Open,
            SlotLifecycle::Open,
            SlotLifecycle::Open,
            SlotLifecycle::Open,
            SlotLifecycle::Open,
            SlotLifecycle::Final
        ]
    );
}

#[test]
fn given_unfinished_arguments_when_previewed_then_only_raw_text_exists_and_no_block() {
    let draft = busy_draft();

    let call = draft.view().slot(bid(12)).unwrap();

    assert_eq!(call.raw_json(), Some("{\"q\":"));
    assert!(call.final_block().is_none());
    assert_eq!(call.lifecycle(), SlotLifecycle::Open);
}

#[test]
fn given_provisional_fields_and_metadata_when_previewed_then_they_are_exposed() {
    let draft = busy_draft();
    let view = draft.view();

    let call = view.slot(bid(12)).unwrap();
    assert_eq!(call.name(), Some(&tool("lookup")));
    assert_eq!(
        call.external_id(),
        Some(&ExternalId::new("toolu_1").unwrap())
    );
    let provider = view.slot(bid(13)).unwrap();
    assert_eq!(provider.operation(), Some(&operation("web_search")));
    let record = view.slot(bid(14)).unwrap();
    assert_eq!(record.status(), Some(ExecutionStatus::Running));
    assert_eq!(view.metadata().model().unwrap().as_str(), "model-x");
    assert_eq!(view.metadata().usage().unwrap().input_tokens(), Some(5));
    assert!(matches!(view.phase(), DraftPhase::Producing { .. }));
    assert_eq!((view.turn(), view.entry()), (tid(1), eid(2)));
}

#[test]
fn given_a_preview_when_taken_then_the_draft_is_neither_consumed_nor_changed() {
    let mut draft = busy_draft();
    let before = draft.clone();

    let first: Vec<_> = draft
        .view()
        .slots()
        .iter()
        .map(|s| s.text().map(str::to_owned))
        .collect();
    let second: Vec<_> = draft
        .view()
        .slots()
        .iter()
        .map(|s| s.text().map(str::to_owned))
        .collect();

    assert_eq!(first, second);
    assert_eq!(draft, before);
    ok(&mut draft, text(10, "xt"));
    assert_eq!(
        draft.view().slot(bid(10)).unwrap().text(),
        Some("partial text")
    );
}

#[test]
fn given_previews_between_deltas_when_completed_then_the_response_equals_a_draft_never_previewed() {
    let script = |previewing: bool| {
        let mut draft = draft();
        let ops = vec![
            start_text(10),
            text(10, "ab"),
            text(10, "cd"),
            finalize(10),
            finish(),
        ];
        for op in ops {
            if previewing {
                let _ = draft.view().slots();
            }
            ok(&mut draft, op);
        }
        draft.into_response().unwrap()
    };

    assert_eq!(script(true), script(false));
}

#[test]
fn given_a_final_and_an_open_slot_when_previewed_then_lifecycles_and_final_block_show() {
    let draft = run(vec![
        start_text(10),
        start_text(11),
        text(10, "done"),
        finalize(10),
    ]);

    let view = draft.view();

    let done = view.slot(bid(10)).unwrap();
    assert_eq!(done.lifecycle(), SlotLifecycle::Final);
    assert_eq!(done.final_block().unwrap().id(), bid(10));
    assert_eq!(done.text(), Some("done"));
    let open = view.slot(bid(11)).unwrap();
    assert_eq!(open.lifecycle(), SlotLifecycle::Open);
    assert_eq!(open.text(), Some(""));
    assert!(open.final_block().is_none());
    assert!(view.slot(bid(99)).is_none());
}

#[test]
fn given_an_interrupted_draft_when_previewed_then_the_phase_carries_the_reason() {
    let mut draft = busy_draft();
    ok(
        &mut draft,
        DeltaOp::response_interrupted(InterruptionReason::truncated()),
    );

    let view = draft.view();

    assert!(matches!(
        view.phase(),
        DraftPhase::Interrupted { reason, .. } if *reason == InterruptionReason::truncated()
    ));
    assert_eq!(view.slot(bid(12)).unwrap().raw_json(), Some("{\"q\":"));
}
