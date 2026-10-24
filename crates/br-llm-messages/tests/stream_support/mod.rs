#![allow(
    dead_code,
    reason = "each integration test crate uses a different subset of these helpers"
)]

use br_llm_messages::{
    BlockStart, Citation, CitationSource, DeltaOp, DeltaOutcome, Error, ExternalId, Finalization,
    ModelName, ProviderOperation, ResponseDelta, ResponseDraft, ResponseEnding, ResponseMetadata,
    TextRange, TurnId, Uuid,
};

use crate::common::{bid, cid, eid, tool, v7};

pub fn tid(n: u64) -> TurnId {
    TurnId::from_uuid(v7(n)).unwrap()
}

pub fn delta(op: DeltaOp) -> ResponseDelta {
    ResponseDelta::new(tid(1), eid(2), op)
}

pub fn fresh() -> ResponseDraft {
    ResponseDraft::new(tid(1), eid(2))
}

pub fn draft() -> ResponseDraft {
    started(ResponseMetadata::new())
}

pub fn started(metadata: ResponseMetadata) -> ResponseDraft {
    let mut draft = fresh();
    draft
        .apply(&delta(DeltaOp::response_started(metadata)))
        .unwrap();
    draft
}

pub fn feed(draft: &mut ResponseDraft, ops: Vec<DeltaOp>) {
    for op in ops {
        draft.apply(&delta(op)).unwrap();
    }
}

pub fn run(ops: Vec<DeltaOp>) -> ResponseDraft {
    let mut draft = draft();
    feed(&mut draft, ops);
    draft
}

pub fn ok(draft: &mut ResponseDraft, op: DeltaOp) -> DeltaOutcome {
    draft.apply(&delta(op)).unwrap()
}

pub fn refuse(draft: &mut ResponseDraft, op: DeltaOp) -> Error {
    let before = draft.clone();
    let error = draft.apply(&delta(op)).unwrap_err();
    assert_eq!(*draft, before, "a refused delta must leave the draft equal");
    error
}

pub fn id_of(error: &Error) -> Option<Uuid> {
    error.object_id()
}

pub fn start_text(block: u64) -> DeltaOp {
    DeltaOp::block_started(bid(block), BlockStart::text())
}

pub fn start_thinking(block: u64) -> DeltaOp {
    DeltaOp::block_started(bid(block), BlockStart::thinking())
}

pub fn start_json(block: u64) -> DeltaOp {
    DeltaOp::block_started(bid(block), BlockStart::structured_content())
}

pub fn start_call(block: u64, name: Option<&str>, external: Option<&str>) -> DeltaOp {
    DeltaOp::block_started(
        bid(block),
        BlockStart::runtime_tool_call(
            name.map(tool),
            external.map(|value| ExternalId::new(value).unwrap()),
        ),
    )
}

pub fn start_provider_call(block: u64) -> DeltaOp {
    DeltaOp::block_started(
        bid(block),
        BlockStart::provider_tool_call(Some(operation("web_search")), None),
    )
}

pub fn start_record(block: u64) -> DeltaOp {
    DeltaOp::block_started(
        bid(block),
        BlockStart::provider_tool_execution_record(None, None, None),
    )
}

pub fn operation(value: &str) -> ProviderOperation {
    ProviderOperation::new(value).unwrap()
}

pub fn text(block: u64, fragment: &str) -> DeltaOp {
    DeltaOp::text_appended(bid(block), fragment)
}

pub fn json(block: u64, fragment: &str) -> DeltaOp {
    DeltaOp::json_appended(bid(block), fragment)
}

pub fn finalize(block: u64) -> DeltaOp {
    DeltaOp::block_finalized(bid(block), Finalization::accumulated())
}

pub fn complete(ending: ResponseEnding) -> DeltaOp {
    DeltaOp::response_completed(ending)
}

pub fn finish() -> DeltaOp {
    complete(ResponseEnding::finished())
}

pub fn citation(n: u64, start: u64, end: u64) -> Citation {
    Citation::new(cid(n), CitationSource::new("doc").unwrap())
        .with_range(TextRange::new(start, end).unwrap())
}

pub fn metadata() -> ResponseMetadata {
    ResponseMetadata::new()
        .with_model(ModelName::new("model-x").unwrap())
        .with_provider_response_id(ExternalId::new("resp_1").unwrap())
}
