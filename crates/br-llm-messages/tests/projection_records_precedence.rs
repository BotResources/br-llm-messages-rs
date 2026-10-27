mod common;
mod projection_support;

use br_llm_messages::{
    EntryRecord, ErrorCode, PendingResponse, Perspective, RepresentationState, TurnEntryKind,
};
use common::turns::human;
use common::{aid, cvid, eid, oj, refused};
use projection_support::*;
use serde_json::json;

#[test]
fn given_a_pending_item_with_an_unknown_turn_and_a_repeated_identity_when_projected_then_unknown_reference_on_both_paths()
 {
    let turns = history();
    let stray = producing(99, 11, 70, "x");
    let pending = [PendingResponse::from(&stray)];

    let live = refused(Perspective::of(aid(1)).project_pending(cvid(900), &turns, &pending));
    let stored = refused(rows_with(&[], &turns, 1).into_projected_thread_pending(
        cvid(900),
        aid(1),
        &pending,
    ));

    for error in [&live, &stored] {
        assert_eq!(error.code(), ErrorCode::UnknownReference);
        assert_eq!(error.object_id(), Some(eid(11).as_uuid()));
        assert_eq!(error.location(), Some("pending[0]"));
    }
}

#[test]
fn given_a_producing_incoming_row_with_no_sender_when_projected_then_the_row_fault_wins_over_the_state()
 {
    let turns = vec![
        ai(10, 2, vec![said(11, 12, "x"), human(13)]),
        waiting(30, 2),
    ];
    let base = rows_selecting(&[], &turns, 1, &|_| true);
    let index = base.entries().len();
    let broken = EntryRecord::new(
        eid(70),
        base.turns()[0].id(),
        TurnEntryKind::HumanMessage,
        RepresentationState::Producing,
        oj(json!({})),
    );
    let mut entries = base.entries().to_vec();
    entries.push(broken);

    let error = refused(
        base.with_entries(entries)
            .into_projected_thread(cvid(900), aid(1)),
    );

    assert_eq!(error.code(), ErrorCode::InvalidContent);
    assert_eq!(
        error.location(),
        Some(format!("entries[{index}].sender").as_str())
    );
}

#[test]
fn given_a_kept_foreign_incoming_row_with_a_fault_when_projected_then_the_error_is_located_at_that_row()
 {
    let turns = vec![
        ai(10, 2, vec![said(11, 12, "x"), human(13)]),
        waiting(30, 2),
    ];
    let base = rows_selecting(&[], &turns, 1, &|_| true);
    let index = base
        .entries()
        .iter()
        .position(|row| row.kind() == TurnEntryKind::HumanMessage)
        .unwrap();
    let kept = &base.entries()[index];
    assert!(kept.sender().is_some());
    let stripped = EntryRecord::new(
        kept.id(),
        kept.turn(),
        kept.kind(),
        kept.state(),
        kept.payload().clone(),
    );
    let entries = base
        .entries()
        .iter()
        .enumerate()
        .map(|(position, row)| {
            if position == index {
                stripped.clone()
            } else {
                row.clone()
            }
        })
        .collect();
    let broken = base.with_entries(entries);
    let late = producing(30, 60, 61, "late");
    let pending = [PendingResponse::from(&late)];

    let plain = refused(broken.clone().into_projected_thread(cvid(900), aid(1)));
    let with_pending = refused(broken.into_projected_thread_pending(cvid(900), aid(1), &pending));

    for error in [&plain, &with_pending] {
        assert_eq!(error.code(), ErrorCode::InvalidContent);
        assert_eq!(error.object_id(), Some(eid(13).as_uuid()));
        assert_eq!(
            error.location(),
            Some(format!("entries[{index}].sender").as_str())
        );
    }
}
