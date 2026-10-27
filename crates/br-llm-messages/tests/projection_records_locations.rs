mod common;
mod projection_support;

use br_llm_messages::{
    BlockRecord, EntryRecord, ErrorCode, RecordSet, RepresentationState, TurnEntryKind,
};
use common::{aid, bid, cvid, refused};
use projection_support::*;

fn with_dropped_rows_first(rows: &RecordSet) -> RecordSet {
    let snapshot = interrupted(30, 32, 33).to_snapshot().unwrap();
    let mut entries = vec![snapshot.entry().clone()];
    entries.extend(rows.entries().iter().cloned());
    let mut blocks = snapshot.blocks().to_vec();
    blocks.extend(rows.blocks().iter().cloned());
    assert!(
        blocks.len() > 1,
        "the dropped response must own several blocks"
    );
    rows.clone().with_entries(entries).with_blocks(blocks)
}

#[test]
fn given_an_interrupted_foreign_response_before_an_invalid_entry_row_when_projected_then_the_error_carries_the_original_index()
 {
    let turns = vec![
        ai(10, 2, vec![said(11, 12, "x"), common::turns::human(13)]),
        waiting(30, 2),
    ];
    let rows = with_dropped_rows_first(&rows_selecting(&[], &turns, 1, &|_| true));
    let broken: Vec<EntryRecord> = rows
        .entries()
        .iter()
        .map(|row| {
            if row.kind() != TurnEntryKind::HumanMessage {
                return row.clone();
            }
            let copy = EntryRecord::new(
                row.id(),
                row.turn(),
                row.kind(),
                RepresentationState::Interrupted,
                row.payload().clone(),
            );
            match row.sender() {
                Some(sender) => copy.with_sender(sender.clone()),
                None => copy,
            }
        })
        .collect();
    let index = broken
        .iter()
        .position(|row| {
            row.state() == RepresentationState::Interrupted
                && row.kind() == TurnEntryKind::HumanMessage
        })
        .expect("the broken row exists");
    assert!(index > 1, "the dropped row must precede the broken one");

    let error = refused(
        rows.with_entries(broken)
            .into_projected_thread(cvid(900), aid(1)),
    );

    assert_eq!(error.code(), ErrorCode::DisallowedKind);
    assert_eq!(
        error.location(),
        Some(format!("entries[{index}].state").as_str())
    );
}

#[test]
fn given_an_interrupted_foreign_response_before_an_invalid_block_row_when_projected_then_the_error_carries_the_original_index()
 {
    let rows = with_dropped_rows_first(&rows_with(&[], &history(), 1));
    let index = rows
        .blocks()
        .iter()
        .position(|row| row.id() == bid(12))
        .expect("the block row exists");
    assert!(index > 1, "the dropped rows must precede the broken one");
    let blocks = rows
        .blocks()
        .iter()
        .map(|row| {
            if row.id() == bid(12) {
                BlockRecord::new(
                    row.id(),
                    row.entry(),
                    row.kind(),
                    RepresentationState::Interrupted,
                    row.payload().clone(),
                )
            } else {
                row.clone()
            }
        })
        .collect();

    let error = refused(
        rows.with_blocks(blocks)
            .into_projected_thread(cvid(900), aid(1)),
    );

    assert_eq!(error.code(), ErrorCode::DisallowedKind);
    assert_eq!(
        error.location(),
        Some(format!("blocks[{index}].state").as_str())
    );
}
