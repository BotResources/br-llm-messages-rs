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
fn given_a_conversation_when_decomposed_then_sibling_positions_are_explicit() {
    let set = rich_set();
    let turns: Vec<u32> = set.turns().iter().map(|turn| turn.position()).collect();
    assert_eq!(turns, [0, 1, 2, 3, 4]);
    let entries: Vec<u32> = set
        .entries()
        .iter()
        .filter(|entry| entry.turn() == tid(10))
        .map(|entry| entry.position())
        .collect();
    assert_eq!(entries, [0, 1, 2, 3, 4, 5]);
    let response_blocks: Vec<_> = set
        .blocks()
        .iter()
        .filter(|block| block.entry() == eid(12) && block.parent().is_none())
        .map(|block| (block.id(), block.position()))
        .collect();
    let expected: Vec<_> = [13, 14, 15, 16, 17, 19]
        .into_iter()
        .zip(0..)
        .map(|(id, position)| (bid(id), position))
        .collect();
    assert_eq!(response_blocks, expected);
}

#[test]
fn given_a_duplicate_sibling_position_when_reconstructed_then_it_is_invalid_position() {
    let entry = failure(|d| row(d, "entries", uuid(21))["position"] = json!(0));
    let block = failure(|d| row(d, "blocks", uuid(14))["position"] = json!(0));
    let child = failure(|d| row(d, "blocks", uuid(24))["position"] = json!(0));
    let turn = failure(|d| row(d, "turns", uuid(10))["position"] = json!(0));

    for error in [entry, block, child, turn] {
        assert_eq!(error.code(), ErrorCode::InvalidPosition);
    }
}

#[test]
fn given_a_gap_in_sibling_positions_when_reconstructed_then_it_is_invalid_position() {
    let entry = failure(|d| row(d, "entries", uuid(30))["position"] = json!(9));
    let child = failure(|d| row(d, "blocks", uuid(25))["position"] = json!(7));
    let turn = failure(|d| row(d, "turns", uuid(60))["position"] = json!(8));

    for error in [entry, child, turn] {
        assert_eq!(error.code(), ErrorCode::InvalidPosition);
    }
}

#[test]
fn given_turn_rows_with_a_gap_when_rebuilt_as_turns_then_only_uniqueness_is_required() {
    let set = edited(&rich_set(), |d| {
        row(d, "turns", uuid(60))["position"] = json!(8)
    });

    assert_eq!(set.into_turns().unwrap().len(), 5);
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
    assert!(
        by_id(19).payload().raw()["exposure"]
            .get("content")
            .is_none()
    );
    assert_eq!(by_id(19).payload().raw()["exposure"]["type"], "readable");
    let serialized = serde_json::to_string(&set.to_value().unwrap().raw()).unwrap();
    assert_eq!(serialized.matches("\"found\"").count(), 1);
}

#[test]
fn given_a_parent_that_does_not_resolve_in_the_entry_when_reconstructed_then_it_is_unknown_reference()
 {
    let missing = failure(|d| row(d, "blocks", uuid(23))["parent"] = json!(text(999)));
    let other_entry = failure(|d| row(d, "blocks", uuid(23))["parent"] = json!(text(17)));

    assert_eq!(missing.code(), ErrorCode::UnknownReference);
    assert_eq!(other_entry.code(), ErrorCode::UnknownReference);
}

#[test]
fn given_a_parent_that_is_not_a_container_when_reconstructed_then_it_is_disallowed() {
    let error = failure(|d| row(d, "blocks", uuid(18))["parent"] = json!(text(14)));

    assert_eq!(error.code(), ErrorCode::DisallowedBlockKind);
}

#[test]
fn given_a_non_content_block_with_a_parent_when_loaded_then_it_is_disallowed() {
    let error = try_edit(&rich_set(), |d| {
        row(d, "blocks", uuid(13))["parent"] = json!(text(22));
    })
    .unwrap_err();

    assert_eq!(error.code(), ErrorCode::DisallowedBlockKind);
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

    assert_eq!(error.code(), ErrorCode::DisallowedBlockKind);
    assert_eq!(own_parent.code(), ErrorCode::InvalidRecord);
}

#[test]
fn given_children_embedded_in_a_parent_payload_when_loaded_then_the_duplicate_is_refused() {
    let result = try_edit(&rich_set(), |d| {
        row(d, "blocks", uuid(22))["payload"]["content"] = json!([]);
    })
    .unwrap_err();
    let record = try_edit(&rich_set(), |d| {
        row(d, "blocks", uuid(19))["payload"]["exposure"]["content"] = json!([]);
    })
    .unwrap_err();

    assert_eq!(result.code(), ErrorCode::UnsupportedField);
    assert_eq!(record.code(), ErrorCode::InvalidRecord);
}

#[test]
fn given_children_under_an_exposure_without_content_when_reconstructed_then_it_is_refused() {
    let error = failure(|d| {
        row(d, "blocks", uuid(19))["payload"]["exposure"] = json!({"type": "not_exposed"});
    });

    assert_eq!(error.code(), ErrorCode::InvalidRecord);
}

#[test]
fn given_rows_that_are_not_complete_when_reconstructed_then_they_are_invalid_record() {
    for state in ["producing", "interrupted"] {
        let error = failure(|d| row(d, "blocks", uuid(14))["state"] = json!(state));
        assert_eq!(error.code(), ErrorCode::InvalidRecord);
    }
    let discarded = failure(|d| {
        let block = row(d, "blocks", uuid(14));
        block["state"] = json!("discarded");
        block["payload"] = json!({});
    });
    assert_eq!(discarded.code(), ErrorCode::InvalidRecord);
    assert_eq!(RepresentationState::Complete.as_str(), "complete");
}

#[test]
fn given_a_reused_row_identity_when_reconstructed_then_it_is_reused_identity() {
    let block_as_entry = failure(|d| row(d, "blocks", uuid(14))["id"] = json!(text(11)));
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
fn given_rows_pointing_at_nothing_when_reconstructed_then_they_are_unknown_reference() {
    let block = failure(|d| row(d, "blocks", uuid(29))["entry"] = json!(text(999)));
    let entry = failure(|d| row(d, "entries", uuid(30))["turn"] = json!(text(999)));

    assert_eq!(block.code(), ErrorCode::UnknownReference);
    assert_eq!(entry.code(), ErrorCode::UnknownReference);
}

#[test]
fn given_a_broken_tool_result_link_when_reconstructed_then_the_build_rule_code_is_reported() {
    let unknown_call = failure(|d| row(d, "blocks", uuid(22))["call"] = json!(text(999)));
    let wrong_kind = failure(|d| row(d, "blocks", uuid(22))["call"] = json!(text(16)));
    let mismatched = failure(|d| {
        row(d, "blocks", uuid(22))["payload"]["tool_name"] = json!("other_tool");
    });

    assert_eq!(unknown_call.code(), ErrorCode::UnknownReference);
    assert_eq!(wrong_kind.code(), ErrorCode::WrongKindReference);
    assert_eq!(mismatched.code(), ErrorCode::ToolResultMismatch);
}

#[test]
fn given_a_response_that_breaks_a_completed_invariant_when_reconstructed_then_it_is_refused() {
    let ending = failure(|d| {
        row(d, "entries", uuid(28))["payload"]["ending"] =
            json!({"type": "tool_results_requested"});
    });
    let blank = try_edit(&rich_set(), |d| {
        row(d, "blocks", uuid(29))["payload"]["text"] = json!("  ");
    })
    .unwrap_err();

    assert_eq!(ending.code(), ErrorCode::MissingContent);
    assert_eq!(blank.code(), ErrorCode::InvalidContent);
}

#[test]
fn given_input_turn_rows_that_break_its_shape_when_reconstructed_then_they_are_refused() {
    let two_entries = failure(|d| {
        let entry = row(d, "entries", uuid(26));
        entry["turn"] = json!(text(1));
        entry["position"] = json!(1);
    });
    let not_incoming = failure(|d| {
        let entry = row(d, "entries", uuid(61));
        entry["kind"] = json!("turn_closure");
        entry["payload"] = json!({"reason": {"type": "completed"}});
        entry.as_object_mut().unwrap().remove("sender");
    });

    assert_eq!(two_entries.code(), ErrorCode::InvalidRecord);
    assert_eq!(not_incoming.code(), ErrorCode::DisallowedEntryKind);
}

#[test]
fn given_a_block_row_under_a_content_less_entry_when_reconstructed_then_it_is_disallowed() {
    let error = failure(|d| row(d, "blocks", uuid(29))["entry"] = json!(text(11)));

    assert_eq!(error.code(), ErrorCode::DisallowedBlockKind);
}

#[test]
fn given_a_location_when_a_build_rule_fails_then_the_error_names_the_turn_and_entry() {
    let error = failure(|d| {
        row(d, "entries", uuid(28))["payload"]["ending"] =
            json!({"type": "tool_results_requested"});
    });

    assert_eq!(error.location(), Some("turns[1].entries[4].content"));
}
