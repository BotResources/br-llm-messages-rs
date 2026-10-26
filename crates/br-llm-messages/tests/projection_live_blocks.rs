mod common;
mod projection_support;

use br_llm_messages::{
    BlockStart, DeltaOp, Finalization, IncomingEntry, Perspective, PlainBlock, ResponseDraft,
    ResponseEnding, Turn,
};
use common::{aid, at, bid, cvid, png};
use projection_support::*;

fn structured_final(draft: &mut ResponseDraft, turn: u64, entry: u64, block: u64) {
    feed(
        draft,
        turn,
        entry,
        vec![
            DeltaOp::block_started(bid(block), BlockStart::structured_content()),
            DeltaOp::json_appended(bid(block), "{\"a\":1}"),
            DeltaOp::block_finalized(bid(block), Finalization::accumulated()),
        ],
    );
}

fn content_of(turn: &Turn, entry: usize) -> Vec<PlainBlock> {
    let Turn::InputTurn(input) = turn else {
        panic!("input turn expected");
    };
    match &input.entries()[entry] {
        IncomingEntry::AgentOutput(output) => output.content().to_vec(),
        IncomingEntry::PartialAgentOutput(partial) => partial.content().to_vec(),
        other => panic!("projected entry expected, got {other:?}"),
    }
}

fn both_paths(drafts: &[ResponseDraft]) -> br_llm_messages::ProjectedThread {
    let turns = history();
    let live = Perspective::of(aid(1))
        .project_live(cvid(900), &turns, drafts)
        .unwrap();
    let stored = rows_with(drafts, &turns, 1)
        .into_projected_thread(cvid(900), aid(1))
        .unwrap();
    assert_eq!(live, stored);
    live
}

#[test]
fn given_a_producing_draft_with_text_image_and_structured_blocks_when_projected_then_final_blocks_are_kept_bare()
 {
    let mut draft = open_text(30, 32, 33, &["so far"]);
    feed(
        &mut draft,
        30,
        32,
        vec![DeltaOp::block_inserted(png(34).with_observed_at(at(4)))],
    );
    structured_final(&mut draft, 30, 32, 35);

    let thread = both_paths(&[draft]);

    let content = content_of(&thread.turns()[1], 0);
    let ids: Vec<_> = content.iter().map(PlainBlock::id).collect();
    assert_eq!(ids, [bid(33), bid(34), bid(35)]);
    assert!(content.iter().all(|block| block.observed_at().is_none()));
}

#[test]
fn given_a_producing_structured_block_still_open_when_projected_then_it_is_skipped_on_both_paths() {
    let mut draft = open_text(30, 32, 33, &["so far"]);
    feed(
        &mut draft,
        30,
        32,
        vec![
            DeltaOp::block_started(bid(34), BlockStart::structured_content()),
            DeltaOp::json_appended(bid(34), "{\"a\":"),
        ],
    );

    let thread = both_paths(&[draft]);

    assert_eq!(content_of(&thread.turns()[1], 0).len(), 1);
}

#[test]
fn given_a_draft_with_only_an_open_structured_block_when_projected_then_no_entry_is_made() {
    let mut draft = started(30, 32);
    feed(
        &mut draft,
        30,
        32,
        vec![
            DeltaOp::block_started(bid(34), BlockStart::structured_content()),
            DeltaOp::json_appended(bid(34), "{\"a\":"),
        ],
    );

    let thread = both_paths(&[draft]);

    assert_eq!(thread.len(), 1);
}

#[test]
fn given_a_response_with_only_an_image_when_projected_then_an_entry_is_made_on_both_paths() {
    let mut draft = started(30, 32);
    feed(&mut draft, 30, 32, vec![DeltaOp::block_inserted(png(34))]);

    let thread = both_paths(&[draft]);

    assert_eq!(kinds(&thread.turns()[1]), ["partial"]);
}

#[test]
fn given_a_completed_draft_not_yet_recorded_when_projected_then_image_and_structured_blocks_are_in_the_output()
 {
    let mut draft = open_text(30, 32, 33, &["done"]);
    feed(
        &mut draft,
        30,
        32,
        vec![
            DeltaOp::block_finalized(bid(33), Finalization::accumulated()),
            DeltaOp::block_inserted(png(34).with_observed_at(at(4))),
        ],
    );
    structured_final(&mut draft, 30, 32, 35);
    feed(
        &mut draft,
        30,
        32,
        vec![DeltaOp::response_completed(ResponseEnding::finished())],
    );
    let turns = history();

    let live = Perspective::of(aid(1))
        .project_live(cvid(900), &turns, std::slice::from_ref(&draft))
        .unwrap();

    assert_eq!(kinds(&live.turns()[1]), ["output"]);
    assert_eq!(content_of(&live.turns()[1], 0).len(), 3);
}
