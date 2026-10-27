mod common;
mod projection_support;
mod recovery_support;

use br_llm_messages::{
    BlockRecord, EntryRecord, ErrorCode, PendingResponse, Perspective, RepresentationState,
    TurnEntryKind, Unfinished,
};
use common::turns::tid;
use common::{aid, cvid, eid, oj, refused};
use projection_support::*;
use recovery_support::*;
use serde_json::json;

#[test]
fn given_a_crashed_set_when_a_projected_thread_is_recovered_then_the_history_shows_the_other_agents_drafts_as_pending_and_only_the_targets_own_rows_become_snapshots()
 {
    let drafts = drafts();
    let set = crashed(&drafts);
    let others = [
        PendingResponse::from(&drafts[2]),
        PendingResponse::from(&drafts[3]),
    ];
    let expected = Perspective::of(aid(2))
        .project_pending(cvid(900), &turns(), &others)
        .unwrap();

    let recovered = set
        .recover_projected_thread(cvid(900), aid(2), Unfinished::Restore)
        .unwrap();

    assert_eq!(recovered.history(), &expected);
    assert_eq!(
        recovered.drafts(),
        [drafts[0].to_snapshot().unwrap()].as_slice()
    );
    assert_eq!(
        recovered.interrupted(),
        [drafts[1].to_snapshot().unwrap()].as_slice()
    );
}

#[test]
fn given_the_drop_option_when_a_projected_thread_is_recovered_then_no_unfinished_row_survives_for_any_agent()
 {
    let drafts = drafts();
    let set = crashed(&drafts);

    let recovered = set
        .recover_projected_thread(cvid(900), aid(2), Unfinished::Drop)
        .unwrap();

    let expected = Perspective::of(aid(2)).project(&crashed_history());
    assert_eq!(recovered.history(), &expected);
    assert!(recovered.drafts().is_empty() && recovered.interrupted().is_empty());
}

#[test]
fn given_a_recovered_snapshot_whose_identity_is_already_in_the_history_when_recovered_then_reused_identity()
 {
    let clash = producing(30, 1, 43, "typing");
    let set = crashed(&[clash]);

    let conversation = refused(set.recover_conversation(cvid(900), Unfinished::Restore));
    let thread = refused(set.recover_projected_thread(cvid(900), aid(2), Unfinished::Restore));

    assert_eq!(conversation.code(), ErrorCode::ReusedIdentity);
    assert_eq!(conversation.object_id(), Some(eid(1).as_uuid()));
    assert_eq!(thread.code(), ErrorCode::ReusedIdentity);
    assert_eq!(thread.object_id(), Some(eid(1).as_uuid()));
}

#[test]
fn given_a_recovered_snapshot_whose_entry_is_the_conversation_identity_when_recovered_then_reused_identity()
 {
    let set = crashed(&[producing(30, 900, 43, "typing")]);

    let conversation = refused(set.recover_conversation(cvid(900), Unfinished::Restore));
    let thread = refused(set.recover_projected_thread(cvid(900), aid(2), Unfinished::Restore));

    assert_eq!(conversation.code(), ErrorCode::ReusedIdentity);
    assert_eq!(thread.code(), ErrorCode::ReusedIdentity);
    assert_eq!(thread.object_id(), Some(cvid(900).as_uuid()));
}

#[test]
fn given_two_snapshots_sharing_a_block_identity_when_recovered_then_reused_identity() {
    let set = crashed(&[producing(30, 40, 43, "a"), producing(50, 60, 43, "b")]);

    let conversation = refused(set.recover_conversation(cvid(900), Unfinished::Restore));
    let thread = refused(set.recover_projected_thread(cvid(900), aid(2), Unfinished::Restore));

    assert_eq!(conversation.code(), ErrorCode::ReusedIdentity);
    assert_eq!(thread.code(), ErrorCode::ReusedIdentity);
}

#[test]
fn given_an_unfinished_row_that_is_not_a_response_when_recovered_then_it_is_still_refused() {
    let set = crashed(&drafts());
    let row = set.entries()[0].clone();
    let producing_human = EntryRecord::new(
        row.id(),
        row.turn(),
        TurnEntryKind::HumanMessage,
        RepresentationState::Producing,
        oj(json!({})),
    );
    let mut entries = set.entries().to_vec();
    entries[0] = producing_human;
    let broken = set.with_entries(entries);

    for unfinished in [Unfinished::Restore, Unfinished::Drop] {
        assert!(broken.recover_conversation(cvid(900), unfinished).is_err());
        assert!(
            broken
                .recover_projected_thread(cvid(900), aid(2), unfinished)
                .is_err()
        );
    }
}

#[test]
fn given_an_unfinished_response_row_in_an_input_turn_when_recovered_then_it_is_refused() {
    let set = crashed(&[producing(30, 40, 43, "typing")]);
    let moved: Vec<EntryRecord> = set
        .entries()
        .iter()
        .map(|row| match row.id() == eid(40) {
            true => EntryRecord::new(
                row.id(),
                tid(1),
                row.kind(),
                row.state(),
                row.payload().clone(),
            ),
            false => row.clone(),
        })
        .collect();
    let broken = set.with_entries(moved);

    for unfinished in [Unfinished::Restore, Unfinished::Drop] {
        assert!(broken.recover_conversation(cvid(900), unfinished).is_err());
    }
}

#[test]
fn given_a_corrupt_unfinished_row_when_recovered_then_it_is_refused_in_both_options() {
    let set = crashed(&[producing(30, 40, 43, "typing")]);
    let mut blocks: Vec<BlockRecord> = set.blocks().to_vec();
    let last = blocks.len() - 1;
    blocks[last] = BlockRecord::new(
        blocks[last].id(),
        blocks[last].entry(),
        blocks[last].kind(),
        blocks[last].state(),
        oj(json!({"text": 7})),
    );
    let broken = set.with_blocks(blocks);

    for unfinished in [Unfinished::Restore, Unfinished::Drop] {
        assert!(broken.recover_conversation(cvid(900), unfinished).is_err());
        assert!(
            broken
                .recover_projected_thread(cvid(900), aid(2), unfinished)
                .is_err()
        );
    }
}

#[test]
fn given_a_block_row_not_complete_under_a_complete_entry_when_recovered_then_it_is_refused() {
    let set = crashed(&[]);
    let mut blocks = set.blocks().to_vec();
    blocks[0] = BlockRecord::new(
        blocks[0].id(),
        blocks[0].entry(),
        blocks[0].kind(),
        RepresentationState::Producing,
        oj(json!({"text": "x"})),
    );

    let error = refused(
        set.with_blocks(blocks)
            .recover_conversation(cvid(900), Unfinished::Restore),
    );

    assert_eq!(error.code(), ErrorCode::DisallowedKind);
}
