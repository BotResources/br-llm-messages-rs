mod common;
mod projection_support;

use br_llm_messages::{
    AITurn, BlockStart, Citation, CitationSource, ClosureReason, Conversation, DeltaOp, ErrorCode,
    Finalization, ModelResponse, PendingResponse, Perspective, PlainBlock, ProviderToolResult,
    ResponseEnding, ResponseMetadata, TextBlock, Turn, TurnEntry,
};
use common::turns::{close, hosted_call, response, tid};
use common::{aid, author, bid, cid, cvid, eid, refused};
use projection_support::*;

const HIDDEN_THINKING: u64 = 20;
const HIDDEN_PROVIDER_CALL: u64 = 21;
const HIDDEN_CHILD_BLOCK: u64 = 23;
const HIDDEN_CITATION: u64 = 24;
const HIDDEN_CLOSURE: u64 = 13;
const OPEN_TURN: u64 = 10;
const OTHER_OPEN_TURN: u64 = 30;
const CLOSED_TURN: u64 = 70;

fn hidden_result() -> br_llm_messages::ContentBlock {
    let cited = TextBlock::new(bid(HIDDEN_CHILD_BLOCK), "found")
        .unwrap()
        .with_citations(vec![Citation::new(
            cid(HIDDEN_CITATION),
            CitationSource::new("doc").unwrap(),
        )])
        .unwrap();
    ProviderToolResult::new(
        bid(22),
        bid(HIDDEN_PROVIDER_CALL),
        vec![PlainBlock::try_from(cited).unwrap()],
    )
    .unwrap()
    .into()
}

fn history() -> Vec<Turn> {
    let exhausted = response(
        11,
        vec![
            br_llm_messages::ThinkingBlock::new(bid(HIDDEN_THINKING), "private plan")
                .unwrap()
                .into(),
            hosted_call(HIDDEN_PROVIDER_CALL),
            hidden_result(),
            TextBlock::new(bid(12), "visible").unwrap().into(),
        ],
        ResponseEnding::output_exhaustion(),
    );
    vec![
        AITurn::from_parts(tid(OPEN_TURN), aid(2), author("Claude"), vec![exhausted])
            .unwrap()
            .into(),
        waiting(OTHER_OPEN_TURN, 2),
        ai(
            CLOSED_TURN,
            2,
            vec![
                said(71, 72, "done"),
                close(HIDDEN_CLOSURE, ClosureReason::completed()),
            ],
        ),
    ]
}

fn unrecorded(entry: u64, block: u64) -> ModelResponse {
    ModelResponse::new(
        eid(entry),
        vec![TextBlock::new(bid(block), "late").unwrap().into()],
        ResponseEnding::finished(),
        ResponseMetadata::new(),
    )
    .unwrap()
}

fn cited_unrecorded(entry: u64, block: u64, citation: u64) -> ModelResponse {
    let cited = TextBlock::new(bid(block), "late")
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

fn accepted_by_both_projections(turn: u64, pending: PendingResponse<'_>) {
    let turns = history();
    let live = Perspective::of(aid(1)).project_pending(cvid(900), &turns, &[pending]);
    let stored =
        rows_with(&[], &turns, 1).into_projected_thread_pending(cvid(900), aid(1), &[pending]);
    assert!(live.is_ok(), "turn {turn}: {live:?}");
    assert_eq!(live.unwrap(), stored.unwrap());
}

fn refused_when_recorded(turn: u64, response: ModelResponse, reused: u64) {
    let mut conversation = Conversation::from_parts(cvid(900), history()).unwrap();

    let error = refused(conversation.push_entry(&tid(turn), TurnEntry::from(response)));

    assert_eq!(error.code(), ErrorCode::ReusedIdentity);
    assert_eq!(error.object_id(), Some(common::v7(reused)));
}

fn check(turn: u64, response: ModelResponse, reused: u64) {
    accepted_by_both_projections(turn, PendingResponse::response(tid(turn), &response));
    refused_when_recorded(turn, response, reused);
}

#[test]
fn given_a_pending_block_reusing_a_hidden_foreign_block_when_recorded_then_reused_identity() {
    for turn in [OPEN_TURN, OTHER_OPEN_TURN] {
        for hidden in [
            HIDDEN_THINKING,
            HIDDEN_PROVIDER_CALL,
            HIDDEN_CHILD_BLOCK,
            22,
        ] {
            check(turn, unrecorded(40, hidden), hidden);
        }
    }
}

#[test]
fn given_a_pending_entry_reusing_a_hidden_foreign_closure_or_entry_when_recorded_then_reused_identity()
 {
    for turn in [OPEN_TURN, OTHER_OPEN_TURN] {
        check(turn, unrecorded(HIDDEN_CLOSURE, 41), HIDDEN_CLOSURE);
    }
}

#[test]
fn given_a_pending_citation_reusing_a_hidden_citation_when_recorded_then_reused_identity() {
    for turn in [OPEN_TURN, OTHER_OPEN_TURN] {
        check(
            turn,
            cited_unrecorded(40, 41, HIDDEN_CITATION),
            HIDDEN_CITATION,
        );
    }
}

#[test]
fn given_a_pending_draft_reusing_a_hidden_block_when_completed_and_recorded_then_reused_identity() {
    let mut draft = producing(OTHER_OPEN_TURN, 40, HIDDEN_THINKING, "late");
    accepted_by_both_projections(OTHER_OPEN_TURN, PendingResponse::from(&draft));
    feed(
        &mut draft,
        OTHER_OPEN_TURN,
        40,
        vec![
            DeltaOp::block_finalized(bid(HIDDEN_THINKING + 1), Finalization::accumulated()),
            DeltaOp::response_completed(ResponseEnding::finished()),
        ],
    );
    let completed = draft.response().unwrap().clone();

    refused_when_recorded(OTHER_OPEN_TURN, completed, HIDDEN_THINKING);
}

#[test]
fn given_a_pending_draft_started_over_a_hidden_text_block_id_when_recorded_then_reused_identity() {
    let mut draft = started(OTHER_OPEN_TURN, 40);
    feed(
        &mut draft,
        OTHER_OPEN_TURN,
        40,
        vec![
            DeltaOp::block_started(bid(HIDDEN_CHILD_BLOCK), BlockStart::text()),
            DeltaOp::text_appended(bid(HIDDEN_CHILD_BLOCK), "late"),
            DeltaOp::block_finalized(bid(HIDDEN_CHILD_BLOCK), Finalization::accumulated()),
            DeltaOp::response_completed(ResponseEnding::finished()),
        ],
    );
    accepted_by_both_projections(OTHER_OPEN_TURN, PendingResponse::from(&draft));

    refused_when_recorded(
        OTHER_OPEN_TURN,
        draft.response().unwrap().clone(),
        HIDDEN_CHILD_BLOCK,
    );
}

#[test]
fn given_a_pending_response_with_fresh_identities_when_recorded_then_it_is_accepted() {
    let fresh = unrecorded(40, 41);
    accepted_by_both_projections(
        OTHER_OPEN_TURN,
        PendingResponse::response(tid(OTHER_OPEN_TURN), &fresh),
    );
    let mut conversation = Conversation::from_parts(cvid(900), history()).unwrap();

    conversation
        .push_entry(&tid(OTHER_OPEN_TURN), TurnEntry::from(fresh))
        .unwrap();
}

#[test]
fn given_a_whole_turn_reusing_a_hidden_foreign_identity_when_the_conversation_is_rebuilt_then_reused_identity()
 {
    for (entry, block, reused) in [
        (40, HIDDEN_THINKING, HIDDEN_THINKING),
        (HIDDEN_CLOSURE, 41, HIDDEN_CLOSURE),
    ] {
        let mut turns = history();
        turns.push(ai(
            80,
            3,
            vec![response(
                entry,
                vec![TextBlock::new(bid(block), "late").unwrap().into()],
                ResponseEnding::finished(),
            )],
        ));

        let error = refused(Conversation::from_parts(cvid(900), turns));

        assert_eq!(error.code(), ErrorCode::ReusedIdentity);
        assert_eq!(error.object_id(), Some(common::v7(reused)));
    }
}
