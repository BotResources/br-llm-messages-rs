mod common;
mod projection_support;

use br_llm_messages::{
    ClosureReason, ErrorCode, ModelResponse, PendingResponse, Perspective, ProjectedThread,
    RecordSet, ResponseEnding, ResponseMetadata, TextBlock, Turn,
};
use common::turns::{close, tid};
use common::{aid, bid, cvid, eid, refused};
use projection_support::*;

fn unrecorded(entry: u64, block: u64, text: &str) -> ModelResponse {
    ModelResponse::new(
        eid(entry),
        vec![TextBlock::new(bid(block), text).unwrap().into()],
        ResponseEnding::finished(),
        ResponseMetadata::new(),
    )
    .unwrap()
}

fn closed_history() -> Vec<Turn> {
    vec![
        ai(
            10,
            2,
            vec![
                said(11, 12, "complete"),
                close(13, ClosureReason::completed()),
            ],
        ),
        waiting(30, 2),
    ]
}

fn both(
    turns: &[Turn],
    agent: u64,
    pending: &[PendingResponse<'_>],
) -> Result<ProjectedThread, br_llm_messages::Error> {
    let live = Perspective::of(aid(agent)).project_pending(cvid(900), turns, pending);
    let stored =
        rows_with(&[], turns, agent).into_projected_thread_pending(cvid(900), aid(agent), pending);
    match (&live, &stored) {
        (Ok(live), Ok(stored)) => assert_eq!(live, stored),
        (Err(live), Err(stored)) => {
            assert_eq!(live.code(), stored.code());
            assert_eq!(live.object_id(), stored.object_id());
        }
        _ => panic!("the two paths disagree: {live:?} / {stored:?}"),
    }
    live
}

#[test]
fn given_an_unrecorded_response_for_a_closed_foreign_turn_when_projected_then_it_is_an_output_at_the_end_of_the_turn()
 {
    let response = unrecorded(14, 15, "late");
    let turns = closed_history();

    let thread = both(&turns, 1, &[PendingResponse::response(tid(10), &response)]).unwrap();

    assert_eq!(kinds(&thread.turns()[0]), ["output", "output"]);
}

#[test]
fn given_a_completed_draft_for_a_closed_foreign_turn_when_projected_then_it_is_an_output() {
    let mut draft = open_text(10, 14, 15, &["late"]);
    feed(
        &mut draft,
        10,
        14,
        vec![
            br_llm_messages::DeltaOp::block_finalized(
                bid(15),
                br_llm_messages::Finalization::accumulated(),
            ),
            br_llm_messages::DeltaOp::response_completed(ResponseEnding::finished()),
        ],
    );

    let thread = both(&closed_history(), 1, &[PendingResponse::draft(&draft)]).unwrap();

    assert_eq!(kinds(&thread.turns()[0]), ["output", "output"]);
}

#[test]
fn given_several_unrecorded_items_per_turn_when_projected_then_they_follow_the_recorded_entries_in_the_order_given()
 {
    let first = unrecorded(14, 15, "one");
    let draft = producing(10, 16, 17, "two");
    let last = unrecorded(20, 21, "three");
    let pending = [
        PendingResponse::response(tid(10), &first),
        PendingResponse::draft(&draft),
        PendingResponse::response(tid(10), &last),
    ];

    let thread = both(&closed_history(), 1, &pending).unwrap();

    assert_eq!(
        kinds(&thread.turns()[0]),
        ["output", "output", "partial", "output"]
    );
    let ids: Vec<_> = match &thread.turns()[0] {
        Turn::InputTurn(input) => input.entries().iter().map(|entry| entry.id()).collect(),
        _ => panic!("input turn expected"),
    };
    assert_eq!(ids, [eid(11), eid(14), eid(16), eid(20)]);
}

#[test]
fn given_an_unrecorded_item_for_an_unknown_turn_or_an_input_turn_when_projected_then_unknown_reference_at_its_entry()
 {
    let response = unrecorded(14, 15, "late");
    let mut turns = closed_history();
    turns.push(common::turns::input_turn(60, 61).into());

    for turn in [99, 60] {
        let error = refused(both(
            &turns,
            1,
            &[PendingResponse::response(tid(turn), &response)],
        ));

        assert_eq!(error.code(), ErrorCode::UnknownReference);
        assert_eq!(error.object_id(), Some(eid(14).as_uuid()));
    }
}

#[test]
fn given_an_unrecorded_item_for_the_targets_own_turn_when_projected_then_disallowed_kind_at_its_entry()
 {
    let draft = producing(30, 32, 33, "x");

    let error = refused(both(&history(), 2, &[PendingResponse::draft(&draft)]));

    assert_eq!(error.code(), ErrorCode::DisallowedKind);
    assert_eq!(error.object_id(), Some(eid(32).as_uuid()));
}

#[test]
fn given_an_entry_identity_that_already_exists_when_an_item_is_added_then_reused_identity_at_its_entry()
 {
    let turns = closed_history();
    let dropped_thinking_only = ai(
        40,
        3,
        vec![common::turns::response(
            41,
            vec![
                br_llm_messages::ThinkingBlock::new(bid(42), "plan")
                    .unwrap()
                    .into(),
            ],
            ResponseEnding::finished(),
        )],
    );
    let mut all = turns.clone();
    all.push(dropped_thinking_only);
    for existing in [11, 30, 41, 60, 12] {
        let mut all = all.clone();
        all.push(common::turns::input_turn(50, 60).into());
        let response = unrecorded(existing, 70, "again");

        let error = refused(both(
            &all,
            1,
            &[PendingResponse::response(tid(30), &response)],
        ));

        assert_eq!(error.code(), ErrorCode::ReusedIdentity, "{existing}");
        assert_eq!(error.object_id(), Some(eid(existing).as_uuid()));
    }
}

#[test]
fn given_two_items_with_the_same_entry_identity_when_projected_then_reused_identity() {
    let first = unrecorded(14, 15, "one");
    let second = unrecorded(14, 16, "two");

    let error = refused(both(
        &closed_history(),
        1,
        &[
            PendingResponse::response(tid(10), &first),
            PendingResponse::response(tid(30), &second),
        ],
    ));

    assert_eq!(error.code(), ErrorCode::ReusedIdentity);
}

#[test]
fn given_a_draft_whose_producing_snapshot_rows_were_also_sent_when_projected_then_reused_identity()
{
    let turns = history();
    let draft = producing(30, 32, 33, "so far");
    let rows: RecordSet = rows_with(std::slice::from_ref(&draft), &turns, 1);

    let error = refused(rows.into_projected_thread_pending(
        cvid(900),
        aid(1),
        &[PendingResponse::draft(&draft)],
    ));

    assert_eq!(error.code(), ErrorCode::ReusedIdentity);
    assert_eq!(error.object_id(), Some(eid(32).as_uuid()));
}

#[test]
fn given_an_unstarted_or_interrupted_draft_when_projected_then_it_gives_nothing() {
    let unstarted = br_llm_messages::ResponseDraft::new(tid(30), eid(32));
    let interrupted = interrupted(30, 42, 43);
    let pending = [
        PendingResponse::draft(&unstarted),
        PendingResponse::draft(&interrupted),
    ];

    let thread = both(&history(), 1, &pending).unwrap();

    assert_eq!(thread.len(), 1);
}

#[test]
fn given_a_closure_entry_identity_when_an_item_is_added_on_the_history_path_then_reused_identity() {
    let response = unrecorded(13, 70, "again");

    let error = refused(Perspective::of(aid(1)).project_pending(
        cvid(900),
        &closed_history(),
        &[PendingResponse::response(tid(30), &response)],
    ));

    assert_eq!(error.code(), ErrorCode::ReusedIdentity);
    assert_eq!(error.object_id(), Some(eid(13).as_uuid()));
}

#[test]
fn given_a_block_identity_already_in_the_translated_turn_when_an_item_is_added_then_reused_identity_at_its_entry()
 {
    let response = unrecorded(14, 12, "clash");

    let error = refused(both(
        &closed_history(),
        1,
        &[PendingResponse::response(tid(10), &response)],
    ));

    assert_eq!(error.code(), ErrorCode::ReusedIdentity);
    assert_eq!(error.object_id(), Some(bid(12).as_uuid()));
    assert_eq!(error.location(), Some("turns[0].entries[1]"));
}
