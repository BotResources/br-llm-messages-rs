//! No entry id appears in a render: frames carry the author's name, the
//! role and the kind, and wire messages carry no id.

use super::test_helpers::*;
use super::*;

use crate::block::UserBlock;
use crate::lighten::lighten;
use crate::test_walk::{agents, walk};
use crate::user_input::{UserInput, UserSource};
use crate::value::{Author, Base64Data, ImageMime, InputId, TurnId};

const HUMAN: &str = "id-of-the-human-input-7f3a";
const RUNTIME: &str = "id-of-the-runtime-input-7f3a";
const OWN: &str = "id-of-the-own-turn-7f3a";
const OTHER: &str = "id-of-the-other-turn-7f3a";
const RELAYED: &str = "id-of-the-relayed-input-7f3a";

fn input(id: &str, source: UserSource, blocks: Vec<UserBlock>) -> UserInput {
    UserInput::new(InputId::new(id).unwrap(), source, None, blocks).unwrap()
}

fn text(body: &str) -> UserBlock {
    UserBlock::text(Text::new(body).unwrap())
}

/// Every kind of entry, each with an id that can appear nowhere else: a
/// human input, an own finished turn, another agent's turn, a runtime input
/// with an image, an own turn awaiting its next step, and a relayed input.
fn conversation() -> Conversation {
    let mut finished = Turn::new(TurnId::new(OWN).unwrap(), Some(agent()), call_step("c1"));
    finished.push_result(result("c1")).unwrap();
    finished.push_step(end_step()).unwrap();
    let other = Turn::new(
        TurnId::new(OTHER).unwrap(),
        Some(Author::new("id-agent-b", "agent-b").unwrap()),
        end_step(),
    );
    let mut open = Turn::new(
        TurnId::new("t-open").unwrap(),
        Some(agent()),
        call_step("c2"),
    );
    open.push_result(result("c2")).unwrap();

    let mut conversation = Conversation::new();
    conversation
        .push_input(input(HUMAN, UserSource::Human, vec![text("question")]))
        .unwrap();
    conversation.push_turn(finished).unwrap();
    conversation.push_turn(other).unwrap();
    let image = UserBlock::image(ImageMime::Png, Base64Data::new("aGk=").unwrap());
    let runtime = UserSource::runtime("upload").unwrap();
    conversation
        .push_input(input(RUNTIME, runtime, vec![text("a file"), image]))
        .unwrap();
    conversation.push_turn(open).unwrap();
    conversation
        .push_input(input(RELAYED, UserSource::Human, vec![text("meanwhile")]))
        .unwrap();
    conversation
}

#[test]
fn given_entries_of_every_kind_when_rendered_then_no_entry_id_appears() {
    let conversation = conversation();
    let wire = render(&conversation, &perspective()).unwrap();

    assert_eq!(
        frame_text(&wire[0]),
        "<message role=\"user\">question</message>"
    );
    assert!(matches!(wire.last(), Some(WireMessage::Relay { .. })));
    let rendered = serde_json::to_string(&wire).unwrap();
    for id in [HUMAN, RUNTIME, OWN, OTHER, "t-open", RELAYED] {
        assert!(!rendered.contains(id), "{id} in {rendered}");
    }
}

#[test]
fn given_the_same_entries_under_other_ids_when_rendered_then_the_render_is_the_same() {
    let renamed = Conversation::from_entries(
        conversation()
            .entries()
            .iter()
            .enumerate()
            .map(|(position, entry)| {
                let mut stored = serde_json::to_value(entry).unwrap();
                stored["id"] = serde_json::json!(format!("other-{position}"));
                serde_json::from_value(stored).unwrap()
            })
            .collect(),
    );
    assert!(renamed.entry("other-1").is_some() && renamed.entry(OWN).is_none());
    assert_eq!(
        render(&renamed, &perspective()),
        render(&conversation(), &perspective())
    );
}

#[test]
fn given_every_state_of_random_walks_when_rendered_then_no_entry_id_appears() {
    let mut perspectives: Vec<Perspective> = agents().into_iter().map(Perspective::new).collect();
    perspectives.push(Perspective::new(
        Author::new("id-stranger", "stranger").unwrap(),
    ));
    let mut rendered_states = 0;
    for seed in 0..100 {
        walk(seed, 40, |_, conversation| {
            for perspective in &perspectives {
                for candidate in [conversation.clone(), lighten(conversation, perspective)] {
                    let Ok(wire) = render(&candidate, perspective) else {
                        continue;
                    };
                    let rendered = serde_json::to_string(&wire).unwrap();
                    for entry in candidate.entries() {
                        assert!(!rendered.contains(entry.id()), "seed {seed}: {rendered}");
                    }
                    rendered_states += 1;
                }
            }
        });
    }
    assert!(rendered_states > 1_000);
}
