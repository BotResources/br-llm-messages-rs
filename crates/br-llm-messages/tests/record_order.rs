mod common;

use br_llm_messages::{ContentBlock, RecordSet, ResponseEnding, Turn, TurnEntry};
use common::records::interleaved;
use common::turns::{human, input_turn, open_turn, pushed, response};
use common::{cvid, text_block};

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
            ResponseEnding::finished(),
        )],
    );
    RecordSet::from_turn(&Turn::AITurn(turn)).unwrap()
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
fn given_block_rows_in_declaration_order_when_rebuilt_then_content_follows_the_slice() {
    let set = two_blocks();

    assert_eq!(texts(set.into_turns().unwrap()), ["first", "third"]);
}

#[test]
fn given_the_same_block_rows_in_another_slice_order_when_rebuilt_then_content_follows_the_new_order()
 {
    let set = two_blocks();
    let mut blocks = set.blocks().to_vec();
    blocks.reverse();
    let reversed = RecordSet::new()
        .with_turns(set.turns().to_vec())
        .with_entries(set.entries().to_vec())
        .with_blocks(blocks);

    assert_eq!(texts(reversed.into_turns().unwrap()), ["third", "first"]);
}

#[test]
fn given_turn_rows_in_another_slice_order_when_rebuilt_then_turns_follow_the_new_order() {
    let first = RecordSet::from_turn(&Turn::InputTurn(input_turn(1, 11))).unwrap();
    let second = RecordSet::from_turn(&Turn::InputTurn(input_turn(2, 12))).unwrap();
    let swapped = RecordSet::new()
        .with_turns(vec![second.turns()[0].clone(), first.turns()[0].clone()])
        .with_entries(vec![
            second.entries()[0].clone(),
            first.entries()[0].clone(),
        ])
        .with_blocks(vec![second.blocks()[0].clone(), first.blocks()[0].clone()]);

    let ids: Vec<_> = swapped
        .into_conversation(cvid(900))
        .unwrap()
        .turns()
        .iter()
        .map(Turn::id)
        .collect();

    assert_eq!(ids, [common::turns::tid(2), common::turns::tid(1)]);
}

#[test]
fn given_entry_rows_of_one_turn_in_another_slice_order_when_rebuilt_then_entries_follow_the_new_order()
 {
    let mut turn = open_turn(1);
    pushed(&mut turn, vec![human(10), human(11)]);
    let set = RecordSet::from_turn(&Turn::AITurn(turn)).unwrap();
    let mut entries = set.entries().to_vec();
    entries.reverse();
    let reversed = RecordSet::new()
        .with_turns(set.turns().to_vec())
        .with_entries(entries)
        .with_blocks(set.blocks().to_vec());

    let rebuilt = reversed.into_turns().unwrap();

    let Turn::AITurn(turn) = &rebuilt[0] else {
        panic!("AI turn expected");
    };
    let ids: Vec<_> = turn.entries().iter().map(TurnEntry::id).collect();
    assert_eq!(ids, [common::eid(11), common::eid(10)]);
}

#[test]
fn given_rows_of_several_turns_interleaved_in_the_slice_when_rebuilt_then_each_turn_keeps_its_own_order()
 {
    let mut first = open_turn(1);
    pushed(&mut first, vec![human(10), human(11), human(12)]);
    let mut second = open_turn(2);
    pushed(&mut second, vec![human(20), human(21)]);
    let mut set = RecordSet::from_turn(&Turn::AITurn(first.clone())).unwrap();
    let other = RecordSet::from_turn(&Turn::AITurn(second.clone())).unwrap();
    set = RecordSet::new()
        .with_turns([set.turns(), other.turns()].concat())
        .with_entries([set.entries(), other.entries()].concat())
        .with_blocks([set.blocks(), other.blocks()].concat());

    let rebuilt = interleaved(&set).into_turns().unwrap();

    assert_eq!(rebuilt, vec![Turn::AITurn(first), Turn::AITurn(second)]);
}
