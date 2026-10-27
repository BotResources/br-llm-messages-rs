mod common;
mod projection_support;

use br_llm_messages::{
    Citation, CitationSource, DeltaOp, ErrorCode, ModelResponse, PendingResponse, Perspective,
    ResponseDraft, ResponseEnding, ResponseMetadata, TextBlock, ThinkingBlock,
};
use common::turns::{hosted_call, hosted_result, tid};
use common::{aid, bid, cid, cvid, eid, refused, v7};
use projection_support::*;

const TURN: u64 = 30;
const CALL: u64 = 41;
const RESULT: u64 = 42;
const CHILD: u64 = 3042;
const CITATION: u64 = 47;

fn cited_block() -> TextBlock {
    TextBlock::new(bid(45), "found")
        .unwrap()
        .with_citations(vec![Citation::new(
            cid(CITATION),
            CitationSource::new("doc").unwrap(),
        )])
        .unwrap()
}

fn holding_a_child(turn: u64, entry: u64) -> ResponseDraft {
    let mut draft = started(turn, entry);
    feed(
        &mut draft,
        turn,
        entry,
        vec![
            DeltaOp::block_inserted(hosted_call(CALL)),
            DeltaOp::block_inserted(hosted_result(RESULT, CALL)),
            DeltaOp::block_inserted(cited_block()),
        ],
    );
    draft
}

fn interrupted_holding_a_child() -> ResponseDraft {
    let mut draft = holding_a_child(TURN, 40);
    feed(
        &mut draft,
        TURN,
        40,
        vec![DeltaOp::response_interrupted(
            br_llm_messages::InterruptionReason::cancelled(),
        )],
    );
    draft
}

fn full_draft(turn: u64, entry: u64) -> ResponseDraft {
    let mut draft = holding_a_child(turn, entry);
    feed(
        &mut draft,
        turn,
        entry,
        vec![
            DeltaOp::block_inserted(ThinkingBlock::new(bid(44), "plan").unwrap()),
            DeltaOp::response_completed(ResponseEnding::finished()),
        ],
    );
    draft
}

fn second_response(entry: u64, block: u64, citation: u64) -> ModelResponse {
    let cited = TextBlock::new(bid(block), "again")
        .unwrap()
        .with_citations(vec![Citation::new(
            cid(citation),
            CitationSource::new("doc").unwrap(),
        )])
        .unwrap();
    ModelResponse::new(
        eid(entry),
        vec![cited.into()],
        ResponseEnding::finished(),
        ResponseMetadata::new(),
    )
    .unwrap()
}

fn rows_with_every_interrupted_row(
    interrupted: &ResponseDraft,
    turns: &[br_llm_messages::Turn],
) -> br_llm_messages::RecordSet {
    let base = rows_with(&[], turns, 1);
    let snapshot = interrupted.to_snapshot().unwrap();
    let mut entries = base.entries().to_vec();
    entries.push(snapshot.entry().clone());
    let mut blocks = base.blocks().to_vec();
    blocks.extend(snapshot.blocks().iter().cloned());
    br_llm_messages::RecordSet::new()
        .with_turns(base.turns().to_vec())
        .with_entries(entries)
        .with_blocks(blocks)
}

#[test]
fn given_an_interrupted_draft_holding_a_child_block_when_a_retry_reuses_it_then_both_doors_refuse_the_same_way()
 {
    let turns = history();
    let interrupted = interrupted_holding_a_child();

    for reused in [CALL, RESULT, CHILD, 45, CITATION] {
        let retry = open_text(TURN, 55, reused, &["again"]);
        let items = [
            PendingResponse::from(&interrupted),
            PendingResponse::from(&retry),
        ];

        let live = Perspective::of(aid(1)).project_pending(cvid(900), &turns, &items);
        let stored = rows_with_every_interrupted_row(&interrupted, &turns)
            .into_projected_thread_pending(cvid(900), aid(1), &[PendingResponse::from(&retry)]);

        for (door, result) in [("memory", live), ("records", stored)] {
            let error = refused(result);
            assert_eq!(error.code(), ErrorCode::ReusedIdentity, "{door} {reused}");
            assert_eq!(error.object_id(), Some(v7(reused)), "{door} {reused}");
        }
    }
}

#[test]
fn given_a_completed_draft_when_a_second_item_reuses_any_of_its_identities_then_the_draft_and_its_response_answer_alike()
 {
    let turns = history();
    let draft = full_draft(TURN, 60);
    let response = draft.response().unwrap().clone();
    let held = [60, 41, 42, CHILD, 44, 45, CITATION];

    for id in held {
        for (entry, block, citation) in [(id, 70, 71), (72, id, 73), (74, 75, id)] {
            let second = second_response(entry, block, citation);
            for first in [
                PendingResponse::from(&draft),
                PendingResponse::response(tid(TURN), &response),
            ] {
                let items = [first, PendingResponse::response(tid(TURN), &second)];

                let live = Perspective::of(aid(1)).project_pending(cvid(900), &turns, &items);

                match live {
                    Err(error) => {
                        assert_eq!(error.code(), ErrorCode::ReusedIdentity, "id {id}");
                        assert_eq!(error.object_id(), Some(v7(id)), "id {id}");
                    }
                    Ok(_) => panic!("id {id} as ({entry}, {block}, {citation}) was accepted"),
                }
            }
        }
    }
}

const OWN_TURN: u64 = 10;
const OWN_ENTRY: u64 = 13;
const OWN_CHILD: u64 = 3015;
const OWN_CITATION: u64 = 17;
const FOREIGN_TURN: u64 = 30;
const KEPT_ENTRY: u64 = 20;
const KEPT_BLOCK: u64 = 5020;
const INPUT_TURN: u64 = 50;

fn visible_history() -> Vec<br_llm_messages::Turn> {
    let cited = TextBlock::new(bid(16), "sourced")
        .unwrap()
        .with_citations(vec![Citation::new(
            cid(OWN_CITATION),
            CitationSource::new("doc").unwrap(),
        )])
        .unwrap();
    let own = common::turns::response(
        OWN_ENTRY,
        vec![hosted_call(14), hosted_result(15, 14), cited.into()],
        ResponseEnding::finished(),
    );
    vec![
        ai(OWN_TURN, 1, vec![own]),
        ai(
            FOREIGN_TURN,
            2,
            vec![
                said(31, 32, "first"),
                common::turns::human(KEPT_ENTRY),
                said(33, 34, "second"),
            ],
        ),
        br_llm_messages::InputTurn::new(
            tid(INPUT_TURN),
            vec![common::turns::human_message(51).into()],
        )
        .unwrap()
        .into(),
    ]
}

fn both_doors(
    turns: &[br_llm_messages::Turn],
    items: &[PendingResponse<'_>],
) -> [Result<br_llm_messages::ProjectedThread, br_llm_messages::Error>; 2] {
    [
        Perspective::of(aid(1)).project_pending(cvid(900), turns, items),
        rows_selecting(&[], turns, 1, &|_| true).into_projected_thread_pending(
            cvid(900),
            aid(1),
            items,
        ),
    ]
}

#[test]
fn given_a_pending_item_reusing_any_visible_identity_when_projected_then_both_doors_refuse_it() {
    let turns = visible_history();
    let reused = [
        OWN_TURN,
        FOREIGN_TURN,
        INPUT_TURN,
        OWN_ENTRY,
        OWN_CHILD,
        OWN_CITATION,
        KEPT_ENTRY,
        KEPT_BLOCK,
    ];

    for id in reused {
        for (entry, block, citation) in [(id, 70, 71), (72, id, 73), (74, 75, id)] {
            let response = second_response(entry, block, citation);
            let item = PendingResponse::response(tid(FOREIGN_TURN), &response);

            for (door, result) in ["memory", "records"]
                .into_iter()
                .zip(both_doors(&turns, &[item]))
            {
                let error = refused(result);
                assert_eq!(error.code(), ErrorCode::ReusedIdentity, "{door} {id}");
                assert_eq!(error.object_id(), Some(v7(id)), "{door} {id}");
                assert_eq!(error.location(), Some("pending[0]"), "{door} {id}");
            }
        }
    }
}

#[test]
fn given_a_pending_item_with_fresh_identities_when_projected_then_both_doors_accept_it() {
    let turns = visible_history();
    let response = second_response(70, 71, 72);
    let item = PendingResponse::response(tid(FOREIGN_TURN), &response);

    let [memory, records] = both_doors(&turns, &[item]);

    assert_eq!(memory.unwrap(), records.unwrap());
}

#[test]
fn given_the_same_pending_item_twice_when_projected_then_both_doors_refuse_the_second() {
    let turns = visible_history();
    let draft = full_draft(FOREIGN_TURN, 60);
    let response = second_response(70, 71, 72);

    for items in [
        [PendingResponse::from(&draft), PendingResponse::from(&draft)],
        [
            PendingResponse::response(tid(FOREIGN_TURN), &response),
            PendingResponse::response(tid(FOREIGN_TURN), &response),
        ],
    ] {
        for (door, result) in ["memory", "records"]
            .into_iter()
            .zip(both_doors(&turns, &items))
        {
            let error = refused(result);
            assert_eq!(error.code(), ErrorCode::ReusedIdentity, "{door}");
            assert_eq!(error.location(), Some("pending[1]"), "{door}");
        }
    }
}
