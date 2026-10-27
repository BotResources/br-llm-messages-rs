mod common;
mod projection_support;

use br_llm_messages::{
    BlockRecord, ContentBlockKind, EntryRecord, ErrorCode, PendingResponse, RecordSet,
    RepresentationState,
};
use common::{aid, bid, cvid, eid, oj, refused};
use projection_support::*;
use serde_json::json;

fn with_rows(set: RecordSet, entries: Vec<EntryRecord>, blocks: Vec<BlockRecord>) -> RecordSet {
    let mut all_entries = set.entries().to_vec();
    all_entries.extend(entries);
    let mut all_blocks = set.blocks().to_vec();
    all_blocks.extend(blocks);
    set.with_entries(all_entries).with_blocks(all_blocks)
}

fn typing_rows() -> RecordSet {
    rows_with(&[open_text(30, 40, 43, &["typing"])], &history(), 1)
}

fn producing_row(kind: ContentBlockKind, payload: serde_json::Value) -> BlockRecord {
    let row = BlockRecord::new(
        bid(70),
        eid(40),
        kind,
        RepresentationState::Producing,
        oj(payload),
    );
    match kind {
        ContentBlockKind::ProviderToolResult => row.with_call(bid(71)),
        _ => row,
    }
}

#[test]
fn given_a_producing_row_of_a_non_plain_kind_under_a_producing_foreign_response_when_projected_then_disallowed_kind_at_the_row()
 {
    let rows = [
        (ContentBlockKind::Thinking, json!({"text": "secret"})),
        (ContentBlockKind::RedactedThinking, json!({})),
        (
            ContentBlockKind::RuntimeToolCall,
            json!({"raw_arguments": "{}"}),
        ),
        (
            ContentBlockKind::ProviderToolCall,
            json!({"raw_arguments": "{}"}),
        ),
        (ContentBlockKind::ProviderToolResult, json!({})),
        (ContentBlockKind::ProviderToolExecutionRecord, json!({})),
    ];
    for (kind, payload) in rows {
        let base = typing_rows();
        let index = base.blocks().len();

        let error = refused(
            with_rows(base, vec![], vec![producing_row(kind, payload)])
                .into_projected_thread(cvid(900), aid(1)),
        );

        assert_eq!(error.code(), ErrorCode::DisallowedKind, "{kind:?}");
        assert_eq!(error.object_id(), Some(bid(70).as_uuid()), "{kind:?}");
        assert_eq!(
            error.location(),
            Some(format!("blocks[{index}]").as_str()),
            "{kind:?}"
        );
    }
}

fn reserved_by_skipped(skipped: BlockRecord) -> RecordSet {
    with_rows(typing_rows(), vec![], vec![skipped])
}

fn skipped_rows() -> Vec<(&'static str, BlockRecord)> {
    let blank = BlockRecord::new(
        bid(45),
        eid(40),
        ContentBlockKind::Text,
        RepresentationState::Producing,
        oj(json!({"text": "   "})),
    );
    let fragments = BlockRecord::new(
        bid(45),
        eid(40),
        ContentBlockKind::StructuredContent,
        RepresentationState::Producing,
        oj(json!({"raw_json": "{\"a\":"})),
    );
    vec![("blank text", blank), ("structured fragments", fragments)]
}

#[test]
fn given_a_skipped_root_row_when_a_pending_item_reuses_its_identity_then_reused_identity_at_the_item()
 {
    for (name, skipped) in skipped_rows() {
        let rows = reserved_by_skipped(skipped);
        let clash = producing(50, 45, 70, "later");

        let error = refused(rows.into_projected_thread_pending(
            cvid(900),
            aid(1),
            &[PendingResponse::from(&clash)],
        ));

        assert_eq!(error.code(), ErrorCode::ReusedIdentity, "{name}");
        assert_eq!(error.object_id(), Some(bid(45).as_uuid()), "{name}");
        assert_eq!(error.location(), Some("pending[0]"), "{name}");
    }
}

#[test]
fn given_a_skipped_root_row_when_an_entry_row_reuses_its_identity_then_reused_identity_at_the_entry_row()
 {
    for (name, skipped) in skipped_rows() {
        let base = reserved_by_skipped(skipped);
        let index = base.entries().len();
        let clash = started(50, 45).to_snapshot().unwrap().entry().clone();

        let error =
            refused(with_rows(base, vec![clash], vec![]).into_projected_thread(cvid(900), aid(1)));

        assert_eq!(error.code(), ErrorCode::ReusedIdentity, "{name}");
        assert_eq!(error.object_id(), Some(bid(45).as_uuid()), "{name}");
        assert_eq!(
            error.location(),
            Some(format!("entries[{index}]").as_str()),
            "{name}"
        );
    }
}

#[test]
fn given_an_entry_row_whose_turn_row_was_not_sent_when_projected_then_invalid_record_structure_at_its_turn()
 {
    let base = typing_rows();
    let index = base.entries().len();
    let orphan = started(61, 60).to_snapshot().unwrap().entry().clone();

    let error =
        refused(with_rows(base, vec![orphan], vec![]).into_projected_thread(cvid(900), aid(1)));

    assert_eq!(error.code(), ErrorCode::InvalidRecordStructure);
    assert_eq!(error.object_id(), Some(eid(60).as_uuid()));
    assert_eq!(
        error.location(),
        Some(format!("entries[{index}].turn").as_str())
    );
}

#[test]
fn given_a_block_row_whose_entry_row_was_not_sent_when_projected_then_invalid_record_structure_at_its_entry()
 {
    let base = typing_rows();
    let index = base.blocks().len();
    let orphan = open_text(30, 63, 64, &["lost"])
        .to_snapshot()
        .unwrap()
        .blocks()
        .to_vec();
    assert_eq!(orphan.len(), 1);

    let error = refused(with_rows(base, vec![], orphan).into_projected_thread(cvid(900), aid(1)));

    assert_eq!(error.code(), ErrorCode::InvalidRecordStructure);
    assert_eq!(error.object_id(), Some(bid(64).as_uuid()));
    assert_eq!(
        error.location(),
        Some(format!("blocks[{index}].entry").as_str())
    );
}
