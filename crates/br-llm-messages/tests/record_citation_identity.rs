mod common;

use br_llm_messages::{Error, ErrorCode};
use common::conversations::rich;
use common::records::{edited, rich_set, row};
use serde_json::json;

fn uuid(id: u64) -> br_llm_messages::Uuid {
    common::v7(id)
}

fn text(id: u64) -> String {
    uuid(id).hyphenated().to_string()
}

fn refused_by_both(block: u64, citation_id: u64) -> (Error, Error) {
    let change = |document: &mut serde_json::Value| {
        row(document, "blocks", uuid(block))["payload"]["citations"][0]["id"] =
            json!(text(citation_id));
    };
    let turns = edited(&rich_set(), change).into_turns().unwrap_err();
    let conversation = edited(&rich_set(), change)
        .into_conversation(rich().id())
        .unwrap_err();
    (turns, conversation)
}

fn assert_reused(outcome: (Error, Error), clashing: u64) {
    for error in [outcome.0, outcome.1] {
        assert_eq!(error.code(), ErrorCode::ReusedIdentity);
        assert_eq!(error.object_id(), Some(uuid(clashing)));
    }
}

#[test]
fn given_two_turns_whose_text_blocks_share_a_citation_id_when_reconstructed_then_both_paths_refuse()
{
    assert_reused(refused_by_both(23, 4), 4);
}

#[test]
fn given_a_citation_id_equal_to_an_entry_id_when_reconstructed_then_both_paths_refuse() {
    assert_reused(refused_by_both(3, 12), 12);
}

#[test]
fn given_a_citation_id_equal_to_a_turn_id_when_reconstructed_then_both_paths_refuse() {
    assert_reused(refused_by_both(3, 10), 10);
}

#[test]
fn given_a_citation_id_equal_to_a_block_id_when_reconstructed_then_both_paths_refuse() {
    assert_reused(refused_by_both(3, 14), 14);
}

#[test]
fn given_distinct_citation_ids_when_reconstructed_then_both_paths_succeed() {
    let untouched = rich_set();
    let renamed = edited(&rich_set(), |document| {
        row(document, "blocks", uuid(3))["payload"]["citations"][0]["id"] = json!(text(900));
    });

    let expected = rich().turns().len();
    assert_eq!(untouched.clone().into_turns().unwrap().len(), expected);
    assert_eq!(renamed.clone().into_turns().unwrap().len(), expected);
    assert!(renamed.into_conversation(rich().id()).is_ok());
    assert!(untouched.into_conversation(rich().id()).is_ok());
}
