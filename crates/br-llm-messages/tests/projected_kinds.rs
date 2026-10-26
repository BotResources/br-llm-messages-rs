mod common;

use br_llm_messages::{
    ContentBlock, ErrorCode, IncomingEntry, PlainBlock, ProjectedThread, ResponseDelta, TurnEntry,
    TurnEntryKind,
};
use common::turns::tid;
use common::{bid, cvid, eid, refused};
use serde_json::json;

#[test]
fn given_a_projected_kind_name_when_parsed_as_an_entry_kind_then_variant_mismatch() {
    for name in ["agent_output", "partial_agent_output"] {
        let error = refused(name.parse::<TurnEntryKind>());

        assert_eq!(error.code(), ErrorCode::VariantMismatch, "{name}");
    }
}

#[test]
fn given_a_projected_kind_name_as_a_block_or_delta_discriminant_when_decoded_then_variant_mismatch()
{
    for name in ["agent_output", "partial_agent_output"] {
        let block = json!({"type": name, "id": bid(3)}).to_string();
        let delta = json!({"turn": tid(1), "entry": eid(2), "type": name}).to_string();

        assert_eq!(
            refused(ContentBlock::from_json(&block)).code(),
            ErrorCode::VariantMismatch
        );
        assert_eq!(
            refused(PlainBlock::from_json(&block)).code(),
            ErrorCode::VariantMismatch
        );
        assert_eq!(
            refused(ResponseDelta::from_json(&delta)).code(),
            ErrorCode::VariantMismatch
        );
    }
}

#[test]
fn given_a_projected_kind_where_a_recorded_entry_is_decoded_then_disallowed_kind_with_its_identity()
{
    for name in ["agent_output", "partial_agent_output"] {
        let entry = json!({"type": name, "id": eid(2), "content": []}).to_string();

        for error in [
            refused(TurnEntry::from_json(&entry)),
            refused(IncomingEntry::from_json(&entry)),
        ] {
            assert_eq!(error.code(), ErrorCode::DisallowedKind, "{name}");
            assert_eq!(error.object_id(), Some(eid(2).as_uuid()), "{name}");
        }
    }
}

#[test]
fn given_a_deeply_nested_projected_thread_document_when_loaded_then_invalid_content_without_overflow()
 {
    let depth = 100_000;
    let text = format!("{}{}", "[".repeat(depth), "]".repeat(depth));
    let document = format!(
        r#"{{"schema":"br-llm-messages-projected-thread/1","conversation":"{}","agent":"{}","turns":{text}}}"#,
        cvid(900).as_uuid(),
        common::aid(1).as_uuid()
    );

    let error = refused(ProjectedThread::from_json(&document));

    assert_eq!(error.code(), ErrorCode::InvalidContent);
}
