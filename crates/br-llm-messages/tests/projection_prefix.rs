mod common;
mod projection_support;

use br_llm_messages::{Conversation, IncomingEntry, Perspective, Turn};
use common::histories::history;
use common::turns::{human, tid};
use common::{aid, cvid};
use projection_support::*;

fn entries_of(turn: &Turn) -> &[IncomingEntry] {
    turn.as_input().map_or(&[], |input| input.entries())
}

#[test]
fn given_a_history_when_a_turn_is_appended_then_the_projection_of_the_earlier_turns_is_unchanged() {
    for seed in 0..60 {
        let conversation = history(seed);
        let turns = conversation.turns();
        for agent in 1..=4 {
            let perspective = Perspective::of(aid(agent));
            for kept in 0..turns.len() {
                let shorter = perspective
                    .project_turns(conversation.id(), &turns[..kept])
                    .unwrap();
                let longer = perspective
                    .project_turns(conversation.id(), &turns[..=kept])
                    .unwrap();

                assert_eq!(
                    &longer.turns()[..shorter.turns().len()],
                    shorter.turns(),
                    "seed {seed} agent {agent} prefix {kept}"
                );
            }
        }
    }
}

#[test]
fn given_a_foreign_turn_when_entries_are_appended_then_only_that_turn_gains_entries_at_its_end() {
    let mut conversation = Conversation::from_parts(
        cvid(900),
        vec![
            ai(10, 2, vec![said(11, 12, "complete")]),
            ai(30, 2, vec![said(31, 32, "first")]),
        ],
    )
    .unwrap();
    let perspective = Perspective::of(aid(1));
    let mut before = perspective.project(&conversation);

    for appended in [human(20), said(33, 34, "second")] {
        conversation.push_entry(&tid(30), appended).unwrap();
        let after = perspective.project(&conversation);

        assert_eq!(after.turns()[0], before.turns()[0]);
        let (old, new) = (&before.turns()[1], &after.turns()[1]);
        assert_eq!(old.id(), new.id());
        assert_eq!(&entries_of(new)[..entries_of(old).len()], entries_of(old));
        assert!(entries_of(new).len() > entries_of(old).len());
        before = after;
    }
}
