mod common;
mod snapshot_support;
mod stream_support;

use br_llm_messages::{DraftSnapshot, ErrorCode, ResponseDraft};
use snapshot_support::{deltas, draft_after, script};

#[test]
fn given_every_prefix_of_a_script_when_restored_and_continued_then_the_result_equals_uninterrupted()
{
    let steps = deltas(&script());
    let uninterrupted = draft_after(&steps, steps.len());
    let expected = uninterrupted.response().cloned().expect("script completes");

    for taken in 1..steps.len() {
        let draft = draft_after(&steps, taken);
        let snapshot = draft.to_snapshot().unwrap();
        let reloaded = DraftSnapshot::from_json(&snapshot.to_json().unwrap()).unwrap();
        assert_eq!(
            reloaded, snapshot,
            "prefix {taken} survives the JSON checkpoint"
        );

        let mut restored = ResponseDraft::from_snapshot(&reloaded).unwrap();
        assert_eq!(restored, draft, "prefix {taken} restores an equal draft");
        for step in &steps[taken..] {
            restored.apply(step).unwrap();
        }

        assert_eq!(restored.response(), Some(&expected), "prefix {taken}");
        assert_eq!(restored, uninterrupted, "prefix {taken}");
    }
}

#[test]
fn given_a_script_when_snapshotted_twice_then_rows_are_deterministic_and_position_ordered() {
    let steps = deltas(&script());
    let draft = draft_after(&steps, 20);

    let first = draft.to_snapshot().unwrap();
    let second = draft.to_snapshot().unwrap();

    assert_eq!(first.to_json().unwrap(), second.to_json().unwrap());
    let roots: Vec<u32> = first
        .blocks()
        .iter()
        .filter(|row| row.parent().is_none())
        .map(|row| row.position())
        .collect();
    let mut sorted = roots.clone();
    sorted.sort_unstable();
    assert_eq!(roots, sorted);
}

#[test]
fn given_a_snapshot_taken_before_any_block_when_restored_then_later_declarations_still_work() {
    let steps = deltas(&script());
    let draft = draft_after(&steps, 1);

    let mut restored = ResponseDraft::from_snapshot(&draft.to_snapshot().unwrap()).unwrap();
    for step in &steps[1..] {
        restored.apply(step).unwrap();
    }

    assert!(restored.response().is_some());
}

#[test]
fn given_a_completed_draft_when_snapshotted_then_it_is_a_stream_phase_error() {
    let steps = deltas(&script());
    let draft = draft_after(&steps, steps.len());

    let error = draft.to_snapshot().unwrap_err();

    assert_eq!(error.code(), ErrorCode::StreamPhaseError);
}
