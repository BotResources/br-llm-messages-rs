use super::*;

use crate::block::{
    AssistantBlock, Image, RedactedThinking, Thinking, ToolCall, ToolResult, ToolResultBlock,
    UserBlock,
};
use crate::conversation::Entry;
use crate::error::MessageError;
use crate::render::{WireMessage, render};
use crate::step::Step;
use crate::stop_reason::StopReason;
use crate::turn::{Turn, TurnItem, TurnState};
use crate::usage::Usage;
use crate::user_input::{UserInput, UserSource};
use crate::value::{
    Author, Base64Data, ImageMime, ModelId, Signature, Text, ToolCallId, ToolName, TurnId,
};

fn author(id: &str, name: &str) -> Author {
    Author::new(id, name).unwrap()
}

fn agent_a() -> Author {
    author("id-agent-a", "Agent A")
}

fn agent_b() -> Author {
    author("id-agent-b", "Agent B")
}

fn perspective_a() -> Perspective {
    Perspective::new(agent_a())
}

fn text(body: &str) -> AssistantBlock {
    AssistantBlock::Text {
        text: Text::new(body).unwrap(),
    }
}

fn thinking(body: &str) -> AssistantBlock {
    AssistantBlock::Thinking(Thinking {
        text: body.to_owned(),
        signature: Some(Signature::new("sig").unwrap()),
    })
}

fn call(id: &str) -> AssistantBlock {
    AssistantBlock::ToolCall(ToolCall {
        id: ToolCallId::new(id).unwrap(),
        name: ToolName::new("search").unwrap(),
        arguments: serde_json::json!({ "q": "rust" }),
    })
}

fn usage() -> Option<Usage> {
    Some(Usage {
        input_tokens: 10,
        output_tokens: 5,
        cache_read_tokens: None,
        cache_write_tokens: None,
        thinking_tokens: Some(3),
    })
}

fn model() -> Option<ModelId> {
    Some(ModelId::new("model-x").unwrap())
}

fn step(content: Vec<AssistantBlock>, stop_reason: StopReason) -> Step {
    Step::new(content, stop_reason, usage(), model()).unwrap()
}

fn result(id: &str) -> ToolResult {
    ToolResult::new(
        ToolCallId::new(id).unwrap(),
        ToolName::new("search").unwrap(),
        vec![ToolResultBlock::text(Text::new("42").unwrap())],
        false,
    )
}

fn human(body: &str) -> UserInput {
    UserInput::new(
        UserSource::Human,
        Some(author("id-user", "Alice")),
        vec![UserBlock::text(Text::new(body).unwrap())],
    )
    .unwrap()
}

fn runtime_with_image() -> UserInput {
    UserInput::new(
        UserSource::runtime("upload").unwrap(),
        None,
        vec![
            UserBlock::text(Text::new("a file").unwrap()),
            UserBlock::Image(Image {
                mime: ImageMime::Png,
                data: Base64Data::new("aGk=").unwrap(),
            }),
        ],
    )
    .unwrap()
}

/// A finished multi-step turn: thinking, text, structured and a call, its
/// result, then redacted thinking, text and structured.
fn multi_step_turn(id: &str, who: Option<Author>) -> Turn {
    let mut turn = Turn::new(
        TurnId::new(id).unwrap(),
        who,
        step(
            vec![
                thinking("private reasoning"),
                text("calling a tool"),
                AssistantBlock::Structured {
                    value: serde_json::json!({ "plan": [1, 2] }),
                },
                call("c1"),
            ],
            StopReason::AwaitingToolResults,
        ),
    );
    turn.push_result(result("c1")).unwrap();
    turn.push_step(step(
        vec![
            AssistantBlock::RedactedThinking(RedactedThinking::new("opaque").unwrap()),
            text("the answer"),
            AssistantBlock::Structured {
                value: serde_json::json!("done"),
            },
        ],
        StopReason::EndTurn,
    ))
    .unwrap();
    turn
}

/// A turn with only thinking and tool calls, still awaiting its results.
fn silent_turn(id: &str, who: Option<Author>) -> Turn {
    Turn::new(
        TurnId::new(id).unwrap(),
        who,
        step(
            vec![
                thinking("thinking only"),
                AssistantBlock::RedactedThinking(RedactedThinking::new("opaque").unwrap()),
                call("c7"),
                call("c8"),
            ],
            StopReason::AwaitingToolResults,
        ),
    )
}

fn own_finished_turn(id: &str) -> Turn {
    multi_step_turn(id, Some(agent_a()))
}

/// The agent's own turn, its results in, waiting for its next step.
fn own_awaiting_step_turn(id: &str) -> Turn {
    let mut turn = Turn::new(
        TurnId::new(id).unwrap(),
        Some(agent_a()),
        step(
            vec![thinking("mine"), call("c5")],
            StopReason::AwaitingToolResults,
        ),
    );
    turn.push_result(result("c5")).unwrap();
    turn
}

fn conversation(entries: Vec<Entry>) -> Conversation {
    let mut conversation = Conversation::new();
    for entry in entries {
        match entry {
            Entry::UserInput(input) => conversation.push_input(input),
            Entry::Turn(turn) => conversation.push_turn(turn).unwrap(),
        }
    }
    conversation
}

fn json<T: serde::Serialize>(value: &T) -> serde_json::Value {
    serde_json::to_value(value).unwrap()
}

fn only_step(turn: &Turn) -> &Step {
    match turn.items() {
        [TurnItem::Step(step)] => step,
        other => panic!("expected exactly one step, got {other:?}"),
    }
}

fn turn_at(conversation: &Conversation, index: usize) -> &Turn {
    match &conversation.entries()[index] {
        Entry::Turn(turn) => turn,
        Entry::UserInput(_) => panic!("expected a turn at {index}"),
    }
}

fn assert_render_preserved(conversation: &Conversation, perspective: &Perspective) {
    let lightened = lighten(conversation, perspective);
    assert_eq!(
        render(&lightened, perspective),
        render(conversation, perspective)
    );
}

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
