mod common;
mod projection_support;
mod recovery_support;

use br_llm_messages::{DraftPhase, ErrorCode, RecordSet, ResponseDraft, Unfinished};
use common::refused;
use common::{aid, cvid};
use projection_support::*;
use recovery_support::*;

#[test]
fn given_a_crashed_set_when_strictly_reconstructed_then_the_unfinished_rows_are_still_refused() {
    let set = crashed(&drafts());

    let error = refused(set.to_conversation(cvid(900)));
    let projected = refused(set.to_projected_thread(cvid(900), aid(2)));

    assert_eq!(error.code(), ErrorCode::DisallowedKind);
    assert_eq!(projected.code(), ErrorCode::DisallowedKind);
    assert_eq!(set.check().unwrap_err().code(), ErrorCode::DisallowedKind);
}

#[test]
fn given_a_crashed_set_when_recovered_as_a_conversation_then_the_complete_rows_give_the_history_and_the_rest_become_snapshots()
 {
    let drafts = drafts();
    let set = crashed(&drafts);
    let untouched = set.clone();

    let recovered = set
        .recover_conversation(cvid(900), Unfinished::Restore)
        .unwrap();

    assert_eq!(set, untouched);
    assert_eq!(recovered.history(), &crashed_history());
    let producing: Vec<_> = [0, 2, 3]
        .iter()
        .map(|index| drafts[*index].to_snapshot().unwrap())
        .collect();
    assert_eq!(recovered.drafts(), producing.as_slice());
    assert_eq!(
        recovered.interrupted(),
        [drafts[1].to_snapshot().unwrap()].as_slice()
    );
}

#[test]
fn given_a_recovered_draft_snapshot_when_more_deltas_arrive_then_it_ends_as_the_uninterrupted_draft_would()
 {
    let mut uninterrupted = producing(30, 40, 43, "typing");
    let recovered = crashed(&drafts())
        .recover_conversation(cvid(900), Unfinished::Restore)
        .unwrap();
    let mut restored = ResponseDraft::from_snapshot(&recovered.drafts()[0]).unwrap();

    let delta = more_text(30, 40, 44, " on");
    uninterrupted.apply(&delta).unwrap();
    restored.apply(&delta).unwrap();

    assert_eq!(
        restored.to_snapshot().unwrap(),
        uninterrupted.to_snapshot().unwrap()
    );
    assert!(matches!(restored.phase(), DraftPhase::Producing { .. }));
}

#[test]
fn given_a_recovered_interrupted_snapshot_when_restored_then_it_stays_interrupted_and_takes_no_delta()
 {
    let reference = interrupted(50, 60, 63);
    let recovered = crashed(&drafts())
        .recover_conversation(cvid(900), Unfinished::Restore)
        .unwrap();
    let snapshot = &recovered.interrupted()[0];
    let mut restored = ResponseDraft::from_snapshot(snapshot).unwrap();

    let error = refused(restored.apply(&more_text(50, 60, 64, "late")));

    assert_eq!(snapshot, &reference.to_snapshot().unwrap());
    assert!(matches!(restored.phase(), DraftPhase::Interrupted { .. }));
    assert_eq!(error.code(), ErrorCode::StreamPhaseError);
}

#[test]
fn given_the_drop_option_when_recovering_then_the_unfinished_rows_vanish_and_the_history_is_the_same()
 {
    let set = crashed(&drafts());

    let recovered = set
        .recover_conversation(cvid(900), Unfinished::Drop)
        .unwrap();

    assert_eq!(recovered.history(), &crashed_history());
    assert!(recovered.drafts().is_empty());
    assert!(recovered.interrupted().is_empty());
}

#[test]
fn given_a_complete_set_when_recovered_then_it_equals_the_strict_reconstruction_with_no_snapshots()
{
    let set = RecordSet::from_conversation(&crashed_history()).unwrap();

    for unfinished in [Unfinished::Restore, Unfinished::Drop] {
        let recovered = set.recover_conversation(cvid(900), unfinished).unwrap();

        assert_eq!(
            recovered.history(),
            &set.to_conversation(cvid(900)).unwrap()
        );
        let (_, drafts, interrupted) = recovered.into_parts();
        assert!(drafts.is_empty() && interrupted.is_empty());
    }
}
