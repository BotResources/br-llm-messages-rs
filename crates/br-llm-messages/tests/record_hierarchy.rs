mod common;
use common::Raw as _;

use br_llm_messages::{Error, ErrorCode, RepresentationState};
use common::conversations::rich;
use common::records::{edited, rich_set, row, try_edit};
use common::turns::tid;
use common::{bid, eid};
use serde_json::{Value, json};

fn failure(change: impl FnOnce(&mut Value)) -> Error {
    edited(&rich_set(), change)
        .into_conversation(rich().id())
        .unwrap_err()
}

fn uuid(id: u64) -> br_llm_messages::Uuid {
    common::v7(id)
}

fn text(id: u64) -> String {
    uuid(id).hyphenated().to_string()
}

#[test]
fn given_a_conversation_when_decomposed_then_rows_follow_the_history_order_without_positions() {
    let set = rich_set();
    let document = set.to_value().unwrap().raw();

    for collection in ["turns", "entries", "blocks"] {
        for row in document[collection].as_array().unwrap() {
            assert!(row.get("position").is_none(), "{collection}: {row}");
        }
    }
    let entries: Vec<_> = set
        .entries()
        .iter()
        .filter(|entry| entry.turn() == tid(10))
        .map(|entry| entry.id())
        .collect();
    let original = rich();
    let expected: Vec<_> = original
        .turns()
        .iter()
        .find(|turn| turn.id() == tid(10))
        .map(|turn| match turn {
            br_llm_messages::Turn::AITurn(ai) => ai.entries().iter().map(|e| e.id()).collect(),
            _ => Vec::new(),
        })
        .unwrap();
    assert_eq!(entries, expected);
    let response_blocks: Vec<_> = set
        .blocks()
        .iter()
        .filter(|block| block.entry() == eid(12) && block.parent().is_none())
        .map(|block| block.id())
        .collect();
    let ids: Vec<_> = [13, 14, 15, 16, 17, 19].into_iter().map(bid).collect();
    assert_eq!(response_blocks, ids);
}

#[test]
fn given_a_position_key_on_a_row_when_loaded_then_it_is_refused_as_an_unknown_field() {
    for collection in ["turns", "entries", "blocks"] {
        let error = try_edit(&rich_set(), |d| {
            d[collection][0]["position"] = json!(0);
        })
        .unwrap_err();

        assert_eq!(error.code(), ErrorCode::InvalidContent, "{collection}");
    }
}

#[test]
fn given_nested_result_content_when_decomposed_then_children_are_rows_of_the_same_entry() {
    let set = rich_set();
    let children: Vec<_> = set
        .blocks()
        .iter()
        .filter(|block| block.parent() == Some(bid(22)))
        .collect();
    assert_eq!(children.len(), 3);
    assert!(children.iter().all(|block| block.entry() == eid(21)));
    let hosted_child = set.blocks().iter().find(|b| b.id() == bid(18)).unwrap();
    assert_eq!(hosted_child.parent(), Some(bid(17)));
    let record_child = set.blocks().iter().find(|b| b.id() == bid(20)).unwrap();
    assert_eq!(record_child.parent(), Some(bid(19)));
}

#[test]
fn given_container_rows_when_decomposed_then_payloads_do_not_embed_their_children() {
    let set = rich_set();
    let by_id = |n: u64| set.blocks().iter().find(|b| b.id() == bid(n)).unwrap();

    assert!(by_id(22).payload().raw().get("content").is_none());
    assert!(by_id(17).payload().raw().get("content").is_none());
    assert!(by_id(19).payload().raw().get("content").is_none());
    assert!(by_id(19).payload().raw().get("exposure").is_none());
    let serialized = serde_json::to_string(&set.to_value().unwrap().raw()).unwrap();
    assert_eq!(serialized.matches("\"found\"").count(), 1);
}

#[test]
fn given_a_parent_that_does_not_resolve_in_the_entry_when_reconstructed_then_it_is_an_invalid_record_structure()
 {
    let missing = failure(|d| row(d, "blocks", uuid(23))["parent"] = json!(text(999)));
    let other_entry = failure(|d| row(d, "blocks", uuid(23))["parent"] = json!(text(17)));

    assert_eq!(missing.code(), ErrorCode::InvalidRecordStructure);
    assert_eq!(other_entry.code(), ErrorCode::InvalidRecordStructure);
}

#[test]
fn given_a_parent_that_is_not_a_container_when_reconstructed_then_it_is_an_invalid_record_structure()
 {
    let error = failure(|d| row(d, "blocks", uuid(18))["parent"] = json!(text(14)));

    assert_eq!(error.code(), ErrorCode::InvalidRecordStructure);
}

#[test]
fn given_a_non_content_block_with_a_parent_when_loaded_then_it_is_an_invalid_record_structure() {
    let error = try_edit(&rich_set(), |d| {
        row(d, "blocks", uuid(13))["parent"] = json!(text(22));
    })
    .unwrap_err();

    assert_eq!(error.code(), ErrorCode::InvalidRecordStructure);
}

#[test]
fn given_rows_that_are_each_others_parents_when_reconstructed_then_the_cycle_is_refused() {
    let error = failure(|d| {
        row(d, "blocks", uuid(23))["parent"] = json!(text(24));
        row(d, "blocks", uuid(24))["parent"] = json!(text(23));
    });
    let own_parent = try_edit(&rich_set(), |d| {
        row(d, "blocks", uuid(23))["parent"] = json!(text(23));
    })
    .unwrap_err();

    assert_eq!(error.code(), ErrorCode::InvalidRecordStructure);
    assert_eq!(own_parent.code(), ErrorCode::InvalidRecordStructure);
}

#[test]
fn given_children_embedded_in_a_parent_payload_when_loaded_then_the_duplicate_is_refused() {
    let result = try_edit(&rich_set(), |d| {
        row(d, "blocks", uuid(22))["payload"]["content"] = json!([]);
    })
    .unwrap_err();
    let record = try_edit(&rich_set(), |d| {
        row(d, "blocks", uuid(19))["payload"]["content"] = json!([]);
    })
    .unwrap_err();

    assert_eq!(result.code(), ErrorCode::InvalidContent);
    assert_eq!(record.code(), ErrorCode::InvalidContent);
}

#[test]
fn given_rows_that_are_not_complete_when_reconstructed_then_they_are_disallowed() {
    for state in ["producing", "interrupted"] {
        let error = failure(|d| row(d, "blocks", uuid(29))["state"] = json!(state));
        assert_eq!(error.code(), ErrorCode::DisallowedKind);
    }
    let discarded = try_edit(&rich_set(), |d| {
        row(d, "blocks", uuid(29))["state"] = json!("discarded");
    })
    .unwrap_err();
    assert_eq!(discarded.code(), ErrorCode::VariantMismatch);
    assert_eq!(RepresentationState::Complete.as_str(), "complete");
}

#[test]
fn given_a_reused_row_identity_when_reconstructed_then_it_is_reused_identity() {
    let block_as_entry = failure(|d| row(d, "blocks", uuid(14))["id"] = json!(text(12)));
    let across_turns = failure(|d| row(d, "blocks", uuid(42))["id"] = json!(text(14)));

    assert_eq!(block_as_entry.code(), ErrorCode::ReusedIdentity);
    assert_eq!(across_turns.code(), ErrorCode::ReusedIdentity);
}

#[test]
fn given_a_reused_citation_identity_across_turns_when_reconstructed_then_it_is_refused() {
    let error = failure(|d| {
        row(d, "blocks", uuid(23))["payload"]["citations"][0]["id"] = json!(text(4));
    });

    assert_eq!(error.code(), ErrorCode::ReusedIdentity);
}

#[test]
fn given_rows_pointing_at_nothing_when_reconstructed_then_they_are_an_invalid_record_structure() {
    let block = failure(|d| row(d, "blocks", uuid(29))["entry"] = json!(text(999)));
    let entry = failure(|d| row(d, "entries", uuid(30))["turn"] = json!(text(999)));

    assert_eq!(block.code(), ErrorCode::InvalidRecordStructure);
    assert_eq!(entry.code(), ErrorCode::InvalidRecordStructure);
}

#[test]
fn given_a_broken_tool_result_link_when_reconstructed_then_the_build_rule_code_is_reported() {
    let unknown_call = failure(|d| row(d, "blocks", uuid(22))["call"] = json!(text(999)));
    let wrong_kind = failure(|d| row(d, "blocks", uuid(22))["call"] = json!(text(16)));
    let mismatched = failure(|d| {
        row(d, "blocks", uuid(22))["payload"]["tool_name"] = json!("other_tool");
    });

    assert_eq!(unknown_call.code(), ErrorCode::UnknownReference);
    assert_eq!(wrong_kind.code(), ErrorCode::UnknownReference);
    assert_eq!(mismatched.code(), ErrorCode::MismatchedResult);
}

#[test]
fn given_a_response_that_breaks_a_completed_invariant_when_reconstructed_then_it_is_refused() {
    let ending = failure(|d| {
        row(d, "entries", uuid(28))["payload"]["ending"] = json!({"type": "tool_use"});
    });
    let blank = try_edit(&rich_set(), |d| {
        row(d, "blocks", uuid(29))["payload"]["text"] = json!("  ");
    })
    .unwrap_err();

    assert_eq!(ending.code(), ErrorCode::InvalidContent);
    assert_eq!(blank.code(), ErrorCode::InvalidContent);
}

#[test]
fn given_input_turn_rows_that_break_its_shape_when_reconstructed_then_they_are_refused() {
    let two_entries = failure(|d| {
        let entry = row(d, "entries", uuid(26));
        entry["turn"] = json!(text(1));
    });
    let not_incoming = failure(|d| {
        let entry = row(d, "entries", uuid(61));
        entry["kind"] = json!("turn_closure");
        entry["payload"] = json!({"reason": {"type": "completed"}});
        entry.as_object_mut().unwrap().remove("sender");
    });

    assert_eq!(two_entries.code(), ErrorCode::InvalidRecordStructure);
    assert_eq!(not_incoming.code(), ErrorCode::DisallowedKind);
}

#[test]
fn given_a_block_row_under_a_content_less_entry_when_reconstructed_then_it_is_disallowed() {
    let error = failure(|d| row(d, "blocks", uuid(29))["entry"] = json!(text(30)));

    assert_eq!(error.code(), ErrorCode::DisallowedKind);
}

#[test]
fn given_a_location_when_a_build_rule_fails_then_the_error_names_the_turn_and_entry() {
    let error = failure(|d| {
        row(d, "entries", uuid(28))["payload"]["ending"] = json!({"type": "tool_use"});
    });

    assert_eq!(error.location(), Some("turns[1].entries[3].content"));
}
