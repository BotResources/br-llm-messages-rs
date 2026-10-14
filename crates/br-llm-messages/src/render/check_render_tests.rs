//! A list yielded by the render always passes the wire list check.

use super::*;
use crate::block::{Image, Thinking, ToolResult, UserBlock};
use crate::conversation::Conversation;
use crate::lighten::lighten;
use crate::render::{Perspective, render};
use crate::step::Step;
use crate::stop_reason::StopReason;
use crate::test_walk::{agents, walk};
use crate::turn::Turn;
use crate::user_input::{UserInput, UserSource};
use crate::value::{Author, Base64Data, ImageMime, Text, ToolName, TurnId};

fn agent(name: &str) -> Author {
    Author::new(format!("id-{name}"), name).unwrap()
}

fn mine() -> Perspective {
    Perspective::new(agent("agent-a"))
}

fn call_step(ids: &[&str]) -> Step {
    let mut content = vec![AssistantBlock::Thinking(Thinking {
        text: "plan".to_owned(),
        signature: None,
    })];
    for id in ids {
        content.push(AssistantBlock::ToolCall(ToolCall {
            id: ToolCallId::new(*id).unwrap(),
            name: ToolName::new("search").unwrap(),
            arguments: serde_json::json!({}),
        }));
    }
    Step::new(content, StopReason::AwaitingToolResults, None, None).unwrap()
}

fn end_step(body: &str) -> Step {
    Step::new(
        vec![AssistantBlock::Text {
            text: Text::new(body).unwrap(),
        }],
        StopReason::EndTurn,
        None,
        None,
    )
    .unwrap()
}

fn result(id: &str) -> ToolResult {
    ToolResult::new(
        ToolCallId::new(id).unwrap(),
        ToolName::new("search").unwrap(),
        Vec::new(),
        false,
    )
}

fn human(body: &str) -> UserInput {
    UserInput::new(
        UserSource::Human,
        Some(agent("alice")),
        vec![UserBlock::text(Text::new(body).unwrap())],
    )
    .unwrap()
}

fn runtime_image() -> UserInput {
    UserInput::new(
        UserSource::runtime("upload").unwrap(),
        None,
        vec![UserBlock::Image(Image {
            mime: ImageMime::Png,
            data: Base64Data::new("aGk=").unwrap(),
        })],
    )
    .unwrap()
}

/// A turn whose steps call the given ids, every call answered, then left
/// awaiting its next step, or finished when `finish` is set.
fn answered_turn(id: &str, who: Option<Author>, rounds: &[&[&str]], finish: bool) -> Turn {
    let (first, rest) = rounds.split_first().unwrap();
    let mut turn = Turn::new(TurnId::new(id).unwrap(), who, call_step(first));
    turn.push_results(first.iter().map(|id| result(id)))
        .unwrap();
    for ids in rest {
        turn.push_step(call_step(ids)).unwrap();
        turn.push_results(ids.iter().rev().map(|id| result(id)))
            .unwrap();
    }
    if finish {
        turn.push_step(end_step("done")).unwrap();
    }
    turn
}

fn assert_render_passes(
    conversation: &Conversation,
    perspective: &Perspective,
) -> Vec<WireMessage> {
    let list = render(conversation, perspective).unwrap();
    assert_eq!(check_wire_list(&list), Ok(()), "{list:?}");
    let lightened = render(&lighten(conversation, perspective), perspective).unwrap();
    assert_eq!(check_wire_list(&lightened), Ok(()));
    list
}

#[test]
fn given_own_turns_with_several_rounds_when_rendered_then_the_list_passes() {
    let mut conversation = Conversation::new();
    conversation.push_input(human("question"));
    conversation
        .push_turn(answered_turn(
            "t1",
            Some(agent("agent-a")),
            &[&["c1", "c2"], &["c3"]],
            true,
        ))
        .unwrap();
    conversation.push_input(human("again"));
    conversation
        .push_turn(answered_turn(
            "t2",
            Some(agent("agent-a")),
            &[&["c4"]],
            true,
        ))
        .unwrap();
    let list = assert_render_passes(&conversation, &mine());
    assert!(list.len() >= 8);
}

#[test]
fn given_framed_entries_of_other_agents_when_rendered_then_the_list_passes() {
    let mut conversation = Conversation::new();
    conversation.push_input(runtime_image());
    conversation
        .push_turn(answered_turn(
            "t1",
            Some(agent("agent-b")),
            &[&["c1"]],
            false,
        ))
        .unwrap();
    conversation
        .push_turn(Turn::new(
            TurnId::new("t2").unwrap(),
            None,
            call_step(&["c2"]),
        ))
        .unwrap();
    conversation.push_input(human("question"));
    let list = assert_render_passes(&conversation, &mine());
    assert_eq!(list.len(), 1);
}

#[test]
fn given_own_open_turn_followed_by_entries_when_rendered_then_the_relay_list_passes() {
    let mut conversation = Conversation::new();
    conversation.push_input(human("question"));
    conversation
        .push_turn(answered_turn(
            "t1",
            Some(agent("agent-a")),
            &[&["c1", "c2"]],
            false,
        ))
        .unwrap();
    conversation.push_input(runtime_image());
    conversation
        .push_turn(Turn::new(
            TurnId::new("t2").unwrap(),
            Some(agent("agent-b")),
            end_step("meanwhile"),
        ))
        .unwrap();
    let list = assert_render_passes(&conversation, &mine());
    assert!(matches!(list.last(), Some(WireMessage::Relay { .. })));
}

#[test]
fn given_own_results_followed_by_framed_entries_when_rendered_then_the_merged_list_passes() {
    let mut conversation = Conversation::new();
    conversation.push_input(human("question"));
    conversation
        .push_turn(answered_turn(
            "t1",
            Some(agent("agent-a")),
            &[&["c1"]],
            false,
        ))
        .unwrap();
    conversation.push_input(human("interjection"));
    conversation
        .push_turn(answered_turn(
            "t2",
            Some(agent("agent-b")),
            &[&["c2"]],
            true,
        ))
        .unwrap();
    conversation
        .push_turn(Turn::new(
            TurnId::new("t3").unwrap(),
            Some(agent("agent-a")),
            end_step("final"),
        ))
        .unwrap();
    let list = assert_render_passes(&conversation, &mine());
    let merged = list.iter().any(|message| match message {
        WireMessage::User { content } => {
            content
                .iter()
                .any(|block| matches!(block, WireUserBlock::ToolResult(_)))
                && content
                    .iter()
                    .any(|block| matches!(block, WireUserBlock::Text { .. }))
        }
        WireMessage::Assistant { .. } | WireMessage::Relay { .. } => false,
    });
    assert!(merged, "{list:?}");
}

/// What the walks covered, so that the property is not vacuous.
#[derive(Default)]
struct Coverage {
    rendered: usize,
    with_calls: usize,
    with_relay: usize,
    with_merged_results: usize,
}

impl Coverage {
    fn record(&mut self, list: &[WireMessage]) {
        self.rendered += 1;
        let has = |test: &dyn Fn(&WireMessage) -> bool| list.iter().any(test);
        if has(&|message| match message {
            WireMessage::Assistant { content } => content.iter().any(AssistantBlock::is_tool_call),
            WireMessage::User { .. } | WireMessage::Relay { .. } => false,
        }) {
            self.with_calls += 1;
        }
        if has(&|message| matches!(message, WireMessage::Relay { .. })) {
            self.with_relay += 1;
        }
        if has(&|message| match message {
            WireMessage::User { content } => {
                content
                    .iter()
                    .any(|block| matches!(block, WireUserBlock::ToolResult(_)))
                    && content
                        .iter()
                        .any(|block| !matches!(block, WireUserBlock::ToolResult(_)))
            }
            WireMessage::Assistant { .. } | WireMessage::Relay { .. } => false,
        }) {
            self.with_merged_results += 1;
        }
    }
}

#[test]
fn given_every_state_of_random_walks_when_rendered_for_each_agent_then_the_list_passes() {
    let mut perspectives: Vec<Perspective> = agents().into_iter().map(Perspective::new).collect();
    perspectives.push(Perspective::new(agent("stranger")));
    let mut coverage = Coverage::default();
    for seed in 0..100 {
        walk(seed, 40, |_, conversation| {
            for perspective in &perspectives {
                for candidate in [conversation.clone(), lighten(conversation, perspective)] {
                    if let Ok(list) = render(&candidate, perspective) {
                        assert_eq!(check_wire_list(&list), Ok(()), "seed {seed}: {list:?}");
                        coverage.record(&list);
                    }
                }
            }
        });
    }
    assert!(coverage.rendered > 1_000);
    assert!(coverage.with_calls > 100);
    assert!(coverage.with_relay > 100);
    assert!(coverage.with_merged_results > 100);
}
