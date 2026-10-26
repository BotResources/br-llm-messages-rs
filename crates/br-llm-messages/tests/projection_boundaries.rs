mod common;
mod projection_support;

use br_llm_messages::{
    Conversation, ErrorCode, HumanMessage, InputTurn, Perspective, RecordSet, Turn,
};
use common::turns::tid;
use common::{aid, bid, cvid, eid, hid, plain_text, refused};
use projection_support::*;

fn concatenated(turns: &[Turn]) -> RecordSet {
    let (mut rows, mut entries, mut blocks) = (Vec::new(), Vec::new(), Vec::new());
    for turn in turns {
        let set = RecordSet::from_turn(turn).unwrap();
        rows.extend(set.turns().iter().cloned());
        entries.extend(set.entries().iter().cloned());
        blocks.extend(set.blocks().iter().cloned());
    }
    RecordSet::new()
        .with_turns(rows)
        .with_entries(entries)
        .with_blocks(blocks)
}

#[test]
fn given_the_turns_of_a_projected_thread_when_projected_again_then_disallowed_kind_at_the_output() {
    let conversation = Conversation::from_parts(
        cvid(900),
        vec![
            waiting(5, 1),
            ai(
                10,
                2,
                vec![common::turns::human(13), said(11, 12, "foreign")],
            ),
        ],
    )
    .unwrap();
    let turns = Perspective::of(aid(1)).project(&conversation).into_turns();

    let error = refused(Perspective::of(aid(3)).project_turns(cvid(900), &turns));

    assert_eq!(error.code(), ErrorCode::DisallowedKind);
    assert_eq!(error.object_id(), Some(eid(11).as_uuid()));
    assert_eq!(error.location(), Some("turns[1].entries[1]"));
}

#[test]
fn given_a_clash_after_a_foreign_turn_that_gives_nothing_when_projected_then_located_in_the_callers_turns()
 {
    let input = HumanMessage::new(eid(31), hid(1), vec![plain_text(77, "hi")]).unwrap();
    let turns = vec![
        waiting(10, 2),
        InputTurn::new(tid(30), vec![input.into()]).unwrap().into(),
        ai(40, 3, vec![said(41, 77, "clash")]),
    ];

    let live = refused(Perspective::of(aid(1)).project_turns(cvid(900), &turns));
    let rows = concatenated(&turns);
    let index = rows
        .blocks()
        .iter()
        .rposition(|row| row.id() == bid(77))
        .unwrap();
    let stored = refused(rows.into_projected_thread(cvid(900), aid(1)));

    assert_eq!(live.code(), ErrorCode::ReusedIdentity);
    assert_eq!(live.location(), Some("turns[2].entries[0]"));
    assert_eq!(stored.code(), ErrorCode::ReusedIdentity);
    assert_eq!(stored.location(), Some(format!("blocks[{index}]").as_str()));
}

#[test]
fn given_a_kept_foreign_incoming_row_whose_blocks_are_missing_when_projected_then_located_at_its_turn_and_entry()
 {
    let turns = vec![
        input_turn_of(1, 2),
        waiting(5, 1),
        ai(10, 2, vec![common::turns::human(13), said(11, 12, "x")]),
    ];
    let rows = rows_selecting(&[], &turns, 1, &|_| true);
    let blocks = rows
        .blocks()
        .iter()
        .filter(|row| row.entry() != eid(13))
        .cloned()
        .collect();

    let error = refused(
        rows.with_blocks(blocks)
            .into_projected_thread(cvid(900), aid(1)),
    );

    assert_eq!(error.code(), ErrorCode::InvalidContent);
    assert_eq!(error.location(), Some("turns[2].entries[0].content"));
}

fn input_turn_of(turn: u64, entry: u64) -> Turn {
    common::turns::input_turn(turn, entry).into()
}

#[test]
fn given_a_block_row_whose_parent_is_unknown_when_projected_from_records_then_located_at_that_row()
{
    let turns = vec![input_turn_of(1, 2), ai(10, 2, vec![said(11, 12, "x")])];
    let rows = rows_with(&[], &turns, 1);
    let index = rows
        .blocks()
        .iter()
        .position(|row| row.entry() == eid(2))
        .unwrap();
    let mut blocks = rows.blocks().to_vec();
    blocks[index] = blocks[index].clone().with_parent(bid(999));

    let error = refused(
        rows.with_blocks(blocks)
            .into_projected_thread(cvid(900), aid(1)),
    );

    assert_eq!(error.code(), ErrorCode::InvalidRecordStructure);
    assert_eq!(
        error.location(),
        Some(format!("blocks[{index}].parent").as_str())
    );
}
