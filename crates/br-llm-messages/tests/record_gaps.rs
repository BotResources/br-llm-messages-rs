mod common;

use br_llm_messages::{ContentBlock, ErrorCode, RecordSet, ResponseEnding, Turn, TurnEntry};
use common::records::{edited, row, try_edit};
use common::turns::{open_turn, pushed, response};
use common::{bid, refused, text_block, v7};
use serde_json::json;

fn two_blocks() -> RecordSet {
    let mut turn = open_turn(1);
    pushed(
        &mut turn,
        vec![response(
            10,
            vec![
                text_block(11, "first").into(),
                text_block(12, "third").into(),
            ],
            ResponseEnding::Finished,
        )],
    );
    RecordSet::from_turn(&Turn::AITurn(turn), 0).unwrap()
}

fn texts(turns: Vec<Turn>) -> Vec<String> {
    let Turn::AITurn(turn) = &turns[0] else {
        panic!("AI turn expected");
    };
    let TurnEntry::ModelResponse(response) = &turn.entries()[0] else {
        panic!("model response expected");
    };
    response
        .blocks()
        .iter()
        .map(|block| match block {
            ContentBlock::Text(text) => text.text().as_str().to_owned(),
            other => panic!("unexpected block {other:?}"),
        })
        .collect()
}

#[test]
fn given_a_root_block_kept_at_its_declared_position_when_rebuilt_then_order_follows_positions() {
    let set = edited(&two_blocks(), |d| {
        row(d, "blocks", v7(12))["position"] = json!(2);
    });

    let turns = set.into_turns().unwrap();

    assert_eq!(texts(turns), ["first", "third"]);
}

#[test]
fn given_root_blocks_stored_out_of_order_when_rebuilt_then_positions_decide_the_order() {
    let set = edited(&two_blocks(), |d| {
        row(d, "blocks", v7(11))["position"] = json!(5);
        row(d, "blocks", v7(12))["position"] = json!(1);
    });

    assert_eq!(texts(set.into_turns().unwrap()), ["third", "first"]);
}

#[test]
fn given_two_root_blocks_sharing_a_position_when_rebuilt_then_invalid_position() {
    let set = edited(&two_blocks(), |d| {
        row(d, "blocks", v7(12))["position"] = json!(0);
    });

    let error = refused(set.into_turns());

    assert_eq!(error.code(), ErrorCode::InvalidPosition);
}

#[test]
fn given_a_discarded_row_left_in_a_completed_bundle_when_rebuilt_then_invalid_record_names_the_row()
{
    let error = refused(
        try_edit(&two_blocks(), |d| {
            row(d, "blocks", v7(12))["state"] = json!("discarded");
        })
        .and_then(RecordSet::into_turns),
    );

    assert_eq!(error.code(), ErrorCode::InvalidRecord);
    assert_eq!(error.object_id(), Some(bid(12).as_uuid()));
}
