mod common;
mod projection_support;

use br_llm_messages::{
    BlockRecord, EntryRecord, ErrorCode, InputTurn, PendingResponse, Perspective, RecordSet,
    RequestAccepted, Turn,
};
use common::turns::tid;
use common::{aid, bid, cvid, eid, refused, rid};
use projection_support::*;

fn with_rows(set: RecordSet, entries: Vec<EntryRecord>, blocks: Vec<BlockRecord>) -> RecordSet {
    let mut all_entries = set.entries().to_vec();
    all_entries.extend(entries);
    let mut all_blocks = set.blocks().to_vec();
    all_blocks.extend(blocks);
    set.with_entries(all_entries).with_blocks(all_blocks)
}

fn interrupted_rows(turn: u64, entry: u64, block: u64) -> (EntryRecord, Vec<BlockRecord>) {
    let snapshot = interrupted(turn, entry, block).to_snapshot().unwrap();
    (snapshot.entry().clone(), snapshot.blocks().to_vec())
}

fn reused_entry(turns: Vec<Turn>) {
    let (entry, blocks) = interrupted_rows(20, 11, 30);
    let rows = with_rows(rows_with(&[], &turns, 1), vec![entry], blocks);
    let last = rows.entries().len() - 1;

    let error = refused(rows.into_projected_thread(cvid(900), aid(1)));

    assert_eq!(error.code(), ErrorCode::ReusedIdentity);
    assert_eq!(error.object_id(), Some(eid(11).as_uuid()));
    assert_eq!(error.location(), Some(format!("entries[{last}]").as_str()));
}

#[test]
fn given_the_targets_own_response_identity_reused_by_an_interrupted_foreign_row_when_projected_then_reused_identity()
 {
    reused_entry(vec![
        ai(10, 1, vec![said(11, 12, "own answer")]),
        waiting(20, 2),
    ]);
}

#[test]
fn given_a_foreign_output_identity_reused_by_another_agents_interrupted_row_when_projected_then_reused_identity()
 {
    reused_entry(vec![
        ai(10, 2, vec![said(11, 12, "foreign answer")]),
        waiting(20, 3),
    ]);
}

#[test]
fn given_an_input_notification_identity_reused_by_an_interrupted_foreign_row_when_projected_then_reused_identity()
 {
    let note = RequestAccepted::new(eid(11), rid(5));
    let input = InputTurn::new(tid(5), vec![note.into()]).unwrap();
    reused_entry(vec![Turn::InputTurn(input), waiting(20, 3)]);
}

#[test]
fn given_an_interrupted_rows_block_identity_reused_by_a_draft_when_projected_then_reused_identity_on_both_paths()
 {
    let turns = history();
    let stopped = interrupted(30, 32, 33);
    let retry = open_text(30, 40, 34, &["retry"]);
    let rows = rows_with(std::slice::from_ref(&stopped), &turns, 1);

    let stored = refused(rows.into_projected_thread_pending(
        cvid(900),
        aid(1),
        &[PendingResponse::from(&retry)],
    ));
    let live = refused(Perspective::of(aid(1)).project_pending(
        cvid(900),
        &turns,
        &[
            PendingResponse::from(&stopped),
            PendingResponse::from(&retry),
        ],
    ));

    for error in [&stored, &live] {
        assert_eq!(error.code(), ErrorCode::ReusedIdentity);
        assert_eq!(error.object_id(), Some(bid(34).as_uuid()));
    }
    assert_eq!(stored.location(), Some("pending[0]"));
    assert_eq!(live.location(), Some("pending[1]"));
}

#[test]
fn given_an_interrupted_rows_entry_identity_reused_by_a_draft_when_projected_then_reused_identity()
{
    let turns = history();
    let rows = rows_with(&[interrupted(30, 32, 33)], &turns, 1);
    let retry = producing(30, 32, 40, "retry");

    let error = refused(rows.into_projected_thread_pending(
        cvid(900),
        aid(1),
        &[PendingResponse::from(&retry)],
    ));

    assert_eq!(error.code(), ErrorCode::ReusedIdentity);
    assert_eq!(error.object_id(), Some(eid(32).as_uuid()));
    assert_eq!(error.location(), Some("pending[0]"));
}

#[test]
fn given_a_pending_identity_equal_to_a_hidden_foreign_entry_when_projected_then_both_paths_accept_it()
 {
    let turns = vec![
        ai(
            10,
            2,
            vec![
                said(11, 12, "complete"),
                common::turns::close(13, br_llm_messages::ClosureReason::completed()),
            ],
        ),
        waiting(30, 2),
    ];
    let late = producing(30, 13, 40, "late");
    let pending = [PendingResponse::from(&late)];

    let live = Perspective::of(aid(1))
        .project_pending(cvid(900), &turns, &pending)
        .unwrap();
    let stored = rows_with(&[], &turns, 1)
        .into_projected_thread_pending(cvid(900), aid(1), &pending)
        .unwrap();

    assert_eq!(live, stored);
}
