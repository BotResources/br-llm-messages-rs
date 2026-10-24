mod common;
mod stream_support;

use br_llm_messages::{DeltaOp, ErrorCode};
use common::{bid, signature};
use stream_support::{finalize, ok, refuse, run, start_thinking};

#[test]
fn given_a_thinking_block_with_no_text_and_no_replay_when_finalized_then_it_is_refused_and_stays_open()
 {
    let mut draft = run(vec![start_thinking(10), start_thinking(11)]);
    ok(
        &mut draft,
        DeltaOp::replay_material_set(bid(11), signature("s")),
    );

    let error = refuse(&mut draft, finalize(10));

    assert_eq!(error.code(), ErrorCode::InconsistentFinalization);
    assert!(draft.view().slot(bid(10)).unwrap().final_block().is_none());
    ok(&mut draft, finalize(11));
    ok(&mut draft, DeltaOp::thinking_appended(bid(10), "now"));
    ok(&mut draft, finalize(10));
}
