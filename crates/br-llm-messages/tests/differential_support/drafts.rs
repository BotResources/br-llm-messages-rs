use br_llm_messages::{
    BlockId, BlockStart, Citation, CitationId, CitationSource, DeltaOp, EntryId, Finalization,
    InterruptionReason, PlainBlock, ProviderToolCall, ProviderToolResult, ResponseDelta,
    ResponseDraft, ResponseEnding, ResponseMetadata, TextBlock, TurnId,
};

#[derive(Clone, Copy)]
pub struct Extras {
    pub call: BlockId,
    pub result: BlockId,
    pub child: BlockId,
    pub cited: BlockId,
    pub citation: CitationId,
}

pub fn enriched(mut draft: ResponseDraft, extras: Extras) -> ResponseDraft {
    let child = PlainBlock::Text(TextBlock::new(extras.child, "found").unwrap());
    let cited = TextBlock::new(extras.cited, "sourced")
        .unwrap()
        .with_citations(vec![Citation::new(
            extras.citation,
            CitationSource::new("doc").unwrap(),
        )])
        .unwrap();
    feed(
        &mut draft,
        vec![
            DeltaOp::block_inserted(ProviderToolCall::new(extras.call)),
            DeltaOp::block_inserted(
                ProviderToolResult::new(extras.result, extras.call, vec![child]).unwrap(),
            ),
            DeltaOp::block_inserted(cited),
        ],
    );
    draft
}

fn feed(draft: &mut ResponseDraft, ops: Vec<DeltaOp>) {
    for op in ops {
        let delta = ResponseDelta::new(draft.turn(), draft.entry(), op);
        draft.apply(&delta).unwrap();
    }
}

pub fn draft(
    turn: TurnId,
    entry: EntryId,
    thinking: BlockId,
    text: BlockId,
    words: &str,
) -> ResponseDraft {
    let mut draft = ResponseDraft::new(turn, entry);
    feed(
        &mut draft,
        vec![
            DeltaOp::response_started(ResponseMetadata::new()),
            DeltaOp::block_started(thinking, BlockStart::thinking()),
            DeltaOp::thinking_appended(thinking, "private plan"),
            DeltaOp::block_finalized(thinking, Finalization::accumulated()),
            DeltaOp::block_started(text, BlockStart::text()),
            DeltaOp::text_appended(text, words),
        ],
    );
    draft
}

pub fn finished(mut draft: ResponseDraft, text: BlockId) -> ResponseDraft {
    feed(
        &mut draft,
        vec![
            DeltaOp::block_finalized(text, Finalization::accumulated()),
            DeltaOp::response_completed(ResponseEnding::finished()),
        ],
    );
    draft
}

pub fn stopped(mut draft: ResponseDraft) -> ResponseDraft {
    feed(
        &mut draft,
        vec![DeltaOp::response_interrupted(
            InterruptionReason::cancelled(),
        )],
    );
    draft
}
