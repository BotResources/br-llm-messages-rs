mod common;
mod projection_support;

use br_llm_messages::{
    BlockStart, DeltaOp, ErrorCode, Finalization, IncomingEntry, Perspective, Turn,
};
use common::turns::tid;
use common::{aid, bid, cvid, refused};
use projection_support::*;

#[test]
fn given_producing_and_interrupted_rows_when_projected_from_records_then_it_equals_the_live_path() {
    let turns = history();
    let drafts = [producing(30, 32, 33, "so far"), interrupted(50, 52, 53)];
    let expected = Perspective::of(aid(1))
        .project_live(cvid(900), &turns, &drafts)
        .unwrap();

    let rows = rows_with(&drafts, &turns, 1);
    let actual = rows.into_projected_thread(cvid(900), aid(1)).unwrap();

    assert_eq!(actual, expected);
    assert_eq!(kinds(&actual.turns()[1]), ["partial"]);
}

#[test]
fn given_producing_text_that_is_still_blank_when_projected_then_it_is_skipped_on_both_paths() {
    let turns = history();
    for fragments in [vec![], vec![" "], vec!["\n", "  "]] {
        let drafts = [open_text(30, 32, 33, &fragments)];

        let live = Perspective::of(aid(1))
            .project_live(cvid(900), &turns, &drafts)
            .unwrap();
        let rows = rows_with(&drafts, &turns, 1);
        let stored = rows.into_projected_thread(cvid(900), aid(1)).unwrap();

        assert_eq!(live, stored, "{fragments:?}");
        assert_eq!(live.len(), 1, "{fragments:?}");
        assert_eq!(live.turns()[0].id(), tid(10));
    }
}

#[test]
fn given_a_blank_block_next_to_non_blank_text_when_projected_then_only_the_non_blank_text_remains()
{
    let turns = history();
    let mut draft = open_text(30, 32, 33, &["kept"]);
    feed(
        &mut draft,
        30,
        32,
        vec![
            DeltaOp::block_finalized(bid(33), Finalization::accumulated()),
            DeltaOp::block_started(bid(34), BlockStart::text()),
            DeltaOp::text_appended(bid(34), " "),
        ],
    );
    let drafts = [draft];

    let live = Perspective::of(aid(1))
        .project_live(cvid(900), &turns, &drafts)
        .unwrap();
    let stored = rows_with(&drafts, &turns, 1)
        .into_projected_thread(cvid(900), aid(1))
        .unwrap();

    assert_eq!(live, stored);
    let Turn::InputTurn(input) = &live.turns()[1] else {
        panic!("input turn expected");
    };
    let IncomingEntry::PartialAgentOutput(partial) = &input.entries()[0] else {
        panic!("partial expected");
    };
    assert_eq!(partial.content().len(), 1);
    assert_eq!(partial.content()[0].id(), bid(33));
}

#[test]
fn given_interrupted_rows_in_any_position_when_projected_from_records_then_they_are_ignored() {
    let turns = history();
    let drafts = [interrupted(30, 32, 33), producing(50, 52, 53, "live")];
    let expected = Perspective::of(aid(1))
        .project_live(cvid(900), &turns, &drafts)
        .unwrap();

    let rows = rows_with(&drafts, &turns, 1);
    let with_leftover = rows.clone();
    let actual = rows.into_projected_thread(cvid(900), aid(1)).unwrap();

    assert_eq!(actual, expected);
    assert_eq!(kinds(&actual.turns()[1]), ["partial"]);
    assert_eq!(
        with_leftover
            .into_projected_thread(cvid(900), aid(1))
            .unwrap(),
        expected
    );
}

#[test]
fn given_an_interrupted_row_under_the_target_agents_turn_when_projected_from_records_then_disallowed_kind()
 {
    let turns = history();
    let drafts = [interrupted(30, 32, 33)];

    let rows = rows_with(&drafts, &turns, 2);
    let error = refused(rows.into_projected_thread(cvid(900), aid(2)));

    assert_eq!(error.code(), ErrorCode::DisallowedKind);
}

#[test]
fn given_a_producing_row_under_the_target_agents_turn_when_projected_from_records_then_disallowed_kind()
 {
    let turns = history();
    let drafts = [producing(30, 32, 33, "so far")];

    let rows = rows_with(&drafts, &turns, 2);
    let error = refused(rows.into_projected_thread(cvid(900), aid(2)));

    assert_eq!(error.code(), ErrorCode::DisallowedKind);
}
