use super::*;

use crate::block::{AssistantBlock, Thinking};
use crate::conversation::Entry;
use crate::error::MessageError;
use crate::render::{WireMessage, render};
use crate::stop_reason::StopReason;
use crate::turn::TurnState;
use crate::value::Author;

use super::test_helpers::*;

#[test]
fn given_inputs_and_own_turns_when_lightened_then_kept_as_they_are() {
    let original = conversation(vec![
        Entry::UserInput(human("question")),
        Entry::Turn(own_finished_turn("t1")),
        Entry::UserInput(runtime_with_image()),
        Entry::Turn(own_awaiting_step_turn("t2")),
    ]);
    let lightened = lighten(&original, &perspective_a());
    assert_eq!(json(&lightened), json(&original));
}

fn entry_ids(conversation: &Conversation) -> Vec<&str> {
    conversation.entries().iter().map(Entry::id).collect()
}

#[test]
fn given_every_kind_of_entry_when_lightened_then_ids_kept_in_order_and_still_unique() {
    let original = conversation(vec![
        Entry::UserInput(human("question")),
        Entry::Turn(own_finished_turn("t1")),
        Entry::Turn(multi_step_turn("t2", Some(agent_b()))),
        Entry::UserInput(runtime_with_image()),
        Entry::Turn(silent_turn("t3", None)),
    ]);
    let mut lightened = lighten(&original, &perspective_a());

    assert_eq!(entry_ids(&lightened), entry_ids(&original));
    assert_eq!(
        entry_ids(&lightened),
        ["in-question", "t1", "t2", "in-runtime-image", "t3"]
    );
    assert!(matches!(lightened.entry("t2"), Some(Entry::Turn(_))));
    assert!(matches!(
        lightened.entry("in-question"),
        Some(Entry::UserInput(_))
    ));
    assert_eq!(
        lightened.push_turn(silent_turn("in-question", None)),
        Err(MessageError::DuplicateEntryId {
            id: "in-question".to_owned()
        })
    );
}

#[test]
fn given_other_agent_multi_step_turn_when_lightened_then_one_end_turn_step_of_texts_and_structured()
{
    let original = conversation(vec![
        Entry::UserInput(human("question")),
        Entry::Turn(multi_step_turn("t9", Some(agent_b()))),
    ]);
    let lightened = lighten(&original, &perspective_a());

    let turn = turn_at(&lightened, 1);
    assert_eq!(turn.id().as_str(), "t9");
    assert_eq!(json(&turn.author()), json(&Some(agent_b())));
    let step = only_step(turn);
    assert_eq!(
        step.content(),
        &[
            text("calling a tool"),
            AssistantBlock::Structured {
                value: serde_json::json!({ "plan": [1, 2] }),
            },
            text("the answer"),
            AssistantBlock::Structured {
                value: serde_json::json!("done"),
            },
        ]
    );
    assert_eq!(step.stop_reason(), &StopReason::EndTurn);
    assert!(step.usage().is_none());
    assert!(step.model().is_none());
    assert_eq!(
        turn.state(),
        TurnState::Finished {
            stop_reason: StopReason::EndTurn
        }
    );
}

#[test]
fn given_other_agent_turn_of_thinking_and_calls_only_when_lightened_then_one_empty_thinking_step() {
    let original = conversation(vec![
        Entry::UserInput(human("question")),
        Entry::Turn(silent_turn("t9", Some(agent_b()))),
    ]);
    let lightened = lighten(&original, &perspective_a());

    let step = only_step(turn_at(&lightened, 1));
    assert_eq!(
        step.content(),
        &[AssistantBlock::Thinking(Thinking {
            text: String::new(),
            signature: None,
        })]
    );
    assert_eq!(step.stop_reason(), &StopReason::EndTurn);
    assert!(step.usage().is_none());
    assert!(step.model().is_none());
}

#[test]
fn given_turn_without_author_when_lightened_then_treated_as_another_agent() {
    let original = conversation(vec![
        Entry::UserInput(human("question")),
        Entry::Turn(multi_step_turn("t9", None)),
    ]);
    let lightened = lighten(&original, &perspective_a());

    let turn = turn_at(&lightened, 1);
    assert!(turn.author().is_none());
    assert_eq!(
        only_step(turn).content(),
        &[
            text("calling a tool"),
            AssistantBlock::Structured {
                value: serde_json::json!({ "plan": [1, 2] }),
            },
            text("the answer"),
            AssistantBlock::Structured {
                value: serde_json::json!("done"),
            },
        ]
    );
}

#[test]
fn given_own_turn_under_an_old_name_when_lightened_then_kept_by_id() {
    let old_name = Author::new("id-agent-a", "Former name").unwrap();
    let original = conversation(vec![
        Entry::UserInput(human("question")),
        Entry::Turn(multi_step_turn("t1", Some(old_name))),
    ]);
    let lightened = lighten(&original, &perspective_a());
    assert_eq!(json(&lightened), json(&original));
}

#[test]
fn given_mixed_conversation_when_lightened_then_entries_keep_their_order() {
    let original = conversation(vec![
        Entry::UserInput(human("one")),
        Entry::Turn(multi_step_turn("t-b", Some(agent_b()))),
        Entry::Turn(own_finished_turn("t-a")),
        Entry::UserInput(human("two")),
        Entry::Turn(silent_turn("t-x", None)),
    ]);
    let lightened = lighten(&original, &perspective_a());
    let ids: Vec<Option<&str>> = lightened
        .entries()
        .iter()
        .map(|entry| match entry {
            Entry::Turn(turn) => Some(turn.id().as_str()),
            Entry::UserInput(_) => None,
        })
        .collect();
    assert_eq!(ids, vec![None, Some("t-b"), Some("t-a"), None, Some("t-x")]);
    assert_eq!(json(&lightened.entries()[0]), json(&original.entries()[0]));
    assert_eq!(json(&lightened.entries()[2]), json(&original.entries()[2]));
    assert_eq!(json(&lightened.entries()[3]), json(&original.entries()[3]));
}

#[test]
fn given_plain_conversation_when_lightened_then_render_unchanged() {
    let original = conversation(vec![
        Entry::UserInput(human("question")),
        Entry::Turn(own_finished_turn("t1")),
        Entry::Turn(multi_step_turn("t2", Some(agent_b()))),
        Entry::Turn(silent_turn("t3", None)),
        Entry::UserInput(runtime_with_image()),
    ]);
    assert_render_preserved(&original, &perspective_a());
    assert_render_preserved(&original, &Perspective::new(agent_b()));
}

#[test]
fn given_relay_case_when_lightened_then_render_unchanged() {
    let original = conversation(vec![
        Entry::UserInput(human("question")),
        Entry::Turn(multi_step_turn("t0", Some(agent_b()))),
        Entry::Turn(own_finished_turn("t1")),
        Entry::UserInput(human("follow-up")),
        Entry::Turn(own_awaiting_step_turn("t2")),
        Entry::Turn(multi_step_turn("t3", Some(agent_b()))),
        Entry::Turn(silent_turn("t4", None)),
        Entry::UserInput(runtime_with_image()),
    ]);
    let rendered = render(&original, &perspective_a()).unwrap();
    assert!(matches!(rendered.last(), Some(WireMessage::Relay { .. })));
    assert_render_preserved(&original, &perspective_a());
}

#[test]
fn given_own_turn_awaiting_results_when_lightened_then_same_refusal() {
    let original = conversation(vec![
        Entry::UserInput(human("question")),
        Entry::Turn(multi_step_turn("t0", Some(agent_b()))),
        Entry::Turn(silent_turn("t1", Some(agent_a()))),
        Entry::UserInput(human("more")),
    ]);
    let lightened = lighten(&original, &perspective_a());
    assert_eq!(
        render(&lightened, &perspective_a()),
        Err(MessageError::TurnAwaitingResults)
    );
    assert_render_preserved(&original, &perspective_a());
    assert_render_preserved(&original, &Perspective::new(agent_b()));
}

#[test]
fn given_render_starting_with_own_turn_or_empty_when_lightened_then_same_refusal() {
    let starts_with_own = conversation(vec![
        Entry::Turn(own_finished_turn("t1")),
        Entry::Turn(silent_turn("t2", Some(agent_b()))),
    ]);
    assert_eq!(
        render(&starts_with_own, &perspective_a()),
        Err(MessageError::RenderStartsWithAssistant)
    );
    assert_render_preserved(&starts_with_own, &perspective_a());
    assert_render_preserved(&Conversation::new(), &perspective_a());
}

#[test]
fn given_lightened_conversation_when_serialized_then_reloads_identically() {
    let original = conversation(vec![
        Entry::UserInput(human("question")),
        Entry::Turn(own_finished_turn("t1")),
        Entry::Turn(multi_step_turn("t2", Some(agent_b()))),
        Entry::Turn(silent_turn("t3", None)),
        Entry::Turn(own_awaiting_step_turn("t4")),
    ]);
    let lightened = lighten(&original, &perspective_a());
    let stored = serde_json::to_string(&lightened).unwrap();
    let reloaded: Conversation = serde_json::from_str(&stored).unwrap();
    assert_eq!(reloaded, lightened);
    assert_eq!(json(&reloaded), json(&lightened));
    assert_eq!(
        render(&reloaded, &perspective_a()),
        render(&original, &perspective_a())
    );
}

#[test]
fn given_every_state_of_random_walks_when_lightened_for_each_agent_then_render_unchanged() {
    let mut perspectives: Vec<Perspective> = crate::test_walk::agents()
        .into_iter()
        .map(Perspective::new)
        .collect();
    perspectives.push(Perspective::new(author("id-stranger", "stranger")));
    let (mut rendered, mut refused) = (0, 0);
    for seed in 0..100 {
        crate::test_walk::walk(seed, 40, |_, conversation| {
            for perspective in &perspectives {
                let original = render(conversation, perspective);
                let light = lighten(conversation, perspective);
                assert_eq!(entry_ids(&light), entry_ids(conversation), "seed {seed}");
                let lightened = render(&light, perspective);
                assert_eq!(lightened, original, "seed {seed}");
                match original {
                    Ok(_) => rendered += 1,
                    Err(_) => refused += 1,
                }
            }
        });
    }
    assert!(
        rendered > 1_000 && refused > 1_000,
        "{rendered} / {refused}"
    );
}
