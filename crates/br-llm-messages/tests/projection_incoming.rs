mod common;
use common::pending_of;
mod projection_support;

use br_llm_messages::{
    Conversation, ErrorCode, IncomingEntry, Perspective, Turn, TurnEntry, TurnEntryKind,
};
use common::turns::{human, human_message, incoming_batch, tid};
use common::{aid, cvid, eid, refused};
use projection_support::*;

fn entries_of(turn: &Turn) -> &[IncomingEntry] {
    let Turn::InputTurn(input) = turn else {
        panic!("input turn expected");
    };
    input.entries()
}

fn recorded_kinds(turn: &Turn) -> Vec<Option<TurnEntryKind>> {
    entries_of(turn).iter().map(IncomingEntry::kind).collect()
}

#[test]
fn given_a_human_message_between_two_responses_of_another_agent_when_projected_then_order_is_kept()
{
    let turns = vec![ai(
        1,
        2,
        vec![said(11, 12, "first"), human(20), said(13, 14, "second")],
    )];

    let thread = Perspective::of(aid(1)).project(&turns_conversation(&turns));

    assert_eq!(kinds(&thread.turns()[0]), ["output", "recorded", "output"]);
    assert_eq!(
        entries_of(&thread.turns()[0])[1],
        IncomingEntry::from(human_message(20))
    );
    assert_eq!(thread.turns()[0].id(), tid(1));
}

fn turns_conversation(turns: &[Turn]) -> Conversation {
    Conversation::from_parts(cvid(900), turns.to_vec()).unwrap()
}

#[test]
fn given_incoming_entries_of_every_kind_in_another_agents_turn_when_projected_by_default_then_all_are_kept()
 {
    let mut entries = incoming_batch(100);
    entries.push(said(200, 201, "answer"));
    let turns = vec![ai(1, 2, entries)];

    let thread = Perspective::of(aid(1)).project(&turns_conversation(&turns));

    let found = recorded_kinds(&thread.turns()[0]);
    assert_eq!(found.len(), 9);
    assert_eq!(found[8], None);
    assert_eq!(
        &found[..8],
        TurnEntryKind::ALL[..8]
            .iter()
            .map(|k| Some(*k))
            .collect::<Vec<_>>()
    );
}

#[test]
fn given_a_selection_when_projected_then_rejected_incoming_entries_are_dropped_and_order_is_kept() {
    let mut entries = incoming_batch(100);
    entries.push(said(200, 201, "answer"));
    let turns = vec![ai(1, 2, entries)];
    let perspective = Perspective::of(aid(1)).keeping(|entry| {
        matches!(
            entry.kind(),
            Some(TurnEntryKind::HumanMessage | TurnEntryKind::AgentMail)
        )
    });

    let thread = perspective.project(&turns_conversation(&turns));

    assert_eq!(
        recorded_kinds(&thread.turns()[0]),
        [
            Some(TurnEntryKind::HumanMessage),
            Some(TurnEntryKind::AgentMail),
            None
        ]
    );
}

#[test]
fn given_a_selection_when_projected_then_it_never_applies_to_the_target_agents_own_turns() {
    let entries = vec![human(20), said(11, 12, "mine")];
    let turns = vec![ai(1, 1, entries.clone())];
    let perspective = Perspective::of(aid(1)).keeping(|_| false);

    let thread = perspective.project(&turns_conversation(&turns));

    assert_eq!(
        thread.turns()[0].as_ai().unwrap().entries(),
        entries.as_slice()
    );
}

#[test]
fn given_no_output_and_no_kept_incoming_entry_when_projected_then_there_is_no_input_turn() {
    let turns = vec![
        ai(1, 2, vec![human(20)]),
        ai(2, 2, vec![human(30)]),
        waiting(3, 2),
    ];
    let perspective = Perspective::of(aid(1)).keeping(|entry| entry.id() == eid(30));

    let thread = perspective.project(&turns_conversation(&turns));

    assert_eq!(thread.len(), 1);
    assert_eq!(thread.turns()[0].id(), tid(2));
    assert_eq!(kinds(&thread.turns()[0]), ["recorded"]);
}

#[test]
fn given_a_mixed_translated_turn_when_pushed_in_a_conversation_then_disallowed_kind_at_the_output()
{
    let turns = vec![ai(1, 2, vec![human(20), said(11, 12, "x")])];
    let thread = Perspective::of(aid(1)).project(&turns_conversation(&turns));
    let mut target = Conversation::new(cvid(901));

    let error = refused(target.push_turn(thread.turns()[0].clone()));

    assert_eq!(error.code(), ErrorCode::DisallowedKind);
    assert_eq!(error.location(), Some("entries[1]"));
}

#[test]
fn given_a_producing_response_after_a_kept_incoming_entry_when_projected_live_then_it_follows_it() {
    let turns = vec![ai(1, 2, vec![human(20), said(11, 12, "done"), human(21)])];
    let drafts = [producing(1, 40, 41, "so far")];

    let live = Perspective::of(aid(1))
        .project_pending(cvid(900), &turns, &pending_of(&drafts))
        .unwrap();
    let stored = rows_selecting(&drafts, &turns, 1, &|_| true)
        .into_projected_thread(cvid(900), aid(1))
        .unwrap();

    assert_eq!(
        kinds(&live.turns()[0]),
        ["recorded", "output", "recorded", "partial"]
    );
    assert_eq!(live, stored);
}

#[test]
fn given_chosen_incoming_rows_under_another_agents_turn_when_door_used_then_it_equals_the_memory_path()
 {
    let mut entries = incoming_batch(100);
    entries.push(said(200, 201, "answer"));
    let turns = vec![ai(1, 2, entries)];
    let chosen = |id: br_llm_messages::EntryId| id == eid(100) || id == eid(102);
    let expected = Perspective::of(aid(1))
        .keeping(move |entry| chosen(entry.id()))
        .project(&turns_conversation(&turns));

    let actual = rows_selecting(&[], &turns, 1, &chosen)
        .into_projected_thread(cvid(900), aid(1))
        .unwrap();

    assert_eq!(actual, expected);
    assert_eq!(
        kinds(&actual.turns()[0]),
        ["recorded", "recorded", "output"]
    );
}

#[test]
fn given_a_turn_only_with_chosen_incoming_rows_when_door_used_then_an_input_turn_is_made() {
    let turns = vec![ai(1, 2, vec![human(20)]), waiting(2, 2)];

    let actual = rows_selecting(&[], &turns, 1, &|_| true)
        .into_projected_thread(cvid(900), aid(1))
        .unwrap();

    assert_eq!(actual.len(), 1);
    assert_eq!(kinds(&actual.turns()[0]), ["recorded"]);
}

#[test]
fn given_runtime_rows_under_another_agents_turn_when_door_used_then_disallowed_kind() {
    use common::turns::{calling, delivery};
    let turns = vec![ai(
        1,
        2,
        vec![
            calling(11, &[(12, "lookup")]),
            delivery(13, &[(14, 12, "lookup")]),
        ],
    )];
    let rows = rows_selecting(&[], &turns, 1, &|_| false);
    let results = br_llm_messages::EntryRecord::new(
        eid(13),
        tid(1),
        TurnEntryKind::RuntimeToolResults,
        br_llm_messages::RepresentationState::Complete,
        common::oj(serde_json::json!({})),
    );
    let mut entries = rows.entries().to_vec();
    entries.push(results);

    let error = refused(
        rows.with_entries(entries)
            .into_projected_thread(cvid(900), aid(1)),
    );

    assert_eq!(error.code(), ErrorCode::DisallowedKind);
}

#[test]
fn given_a_stored_history_when_it_holds_a_translated_entry_then_it_is_still_refused() {
    let turns = vec![ai(1, 2, vec![human(20), said(11, 12, "x")])];
    let thread = Perspective::of(aid(1)).project(&turns_conversation(&turns));
    let IncomingEntry::AgentOutput(output) = entries_of(&thread.turns()[0])[1].clone() else {
        panic!("output expected");
    };

    let error = refused(TurnEntry::try_from(IncomingEntry::from(output)));

    assert_eq!(error.code(), ErrorCode::DisallowedKind);
}
