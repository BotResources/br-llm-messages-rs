use super::*;

use crate::block::{
    AssistantBlock, Image, RedactedThinking, Thinking, ToolCall, ToolResult, ToolResultBlock,
    UserBlock,
};
use crate::conversation::Entry;
use crate::render::render;
use crate::step::Step;
use crate::stop_reason::StopReason;
use crate::turn::{Turn, TurnItem};
use crate::usage::Usage;
use crate::user_input::{UserInput, UserSource};
use crate::value::{
    Author, Base64Data, ImageMime, ModelId, Signature, Text, ToolCallId, ToolName, TurnId,
};

pub(super) fn author(id: &str, name: &str) -> Author {
    Author::new(id, name).unwrap()
}

pub(super) fn agent_a() -> Author {
    author("id-agent-a", "Agent A")
}

pub(super) fn agent_b() -> Author {
    author("id-agent-b", "Agent B")
}

pub(super) fn perspective_a() -> Perspective {
    Perspective::new(agent_a())
}

pub(super) fn text(body: &str) -> AssistantBlock {
    AssistantBlock::Text {
        text: Text::new(body).unwrap(),
    }
}

pub(super) fn thinking(body: &str) -> AssistantBlock {
    AssistantBlock::Thinking(Thinking {
        text: body.to_owned(),
        signature: Some(Signature::new("sig").unwrap()),
    })
}

pub(super) fn call(id: &str) -> AssistantBlock {
    AssistantBlock::ToolCall(ToolCall {
        id: ToolCallId::new(id).unwrap(),
        name: ToolName::new("search").unwrap(),
        arguments: serde_json::json!({ "q": "rust" }),
    })
}

pub(super) fn usage() -> Option<Usage> {
    Some(Usage {
        input_tokens: 10,
        output_tokens: 5,
        cache_read_tokens: None,
        cache_write_tokens: None,
        thinking_tokens: Some(3),
    })
}

pub(super) fn model() -> Option<ModelId> {
    Some(ModelId::new("model-x").unwrap())
}

pub(super) fn step(content: Vec<AssistantBlock>, stop_reason: StopReason) -> Step {
    Step::new(content, stop_reason, usage(), model()).unwrap()
}

pub(super) fn result(id: &str) -> ToolResult {
    ToolResult::new(
        ToolCallId::new(id).unwrap(),
        ToolName::new("search").unwrap(),
        vec![ToolResultBlock::text(Text::new("42").unwrap())],
        false,
    )
}

pub(super) fn human(body: &str) -> UserInput {
    UserInput::new(
        UserSource::Human,
        Some(author("id-user", "Alice")),
        vec![UserBlock::text(Text::new(body).unwrap())],
    )
    .unwrap()
}

pub(super) fn runtime_with_image() -> UserInput {
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
/// result, then redacted thinking, text and structured. Call ids are prefixed
/// by the turn id, unique in the conversation.
pub(super) fn multi_step_turn(id: &str, who: Option<Author>) -> Turn {
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
                call(&format!("{id}-c1")),
            ],
            StopReason::AwaitingToolResults,
        ),
    );
    turn.push_result(result(&format!("{id}-c1"))).unwrap();
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
pub(super) fn silent_turn(id: &str, who: Option<Author>) -> Turn {
    Turn::new(
        TurnId::new(id).unwrap(),
        who,
        step(
            vec![
                thinking("thinking only"),
                AssistantBlock::RedactedThinking(RedactedThinking::new("opaque").unwrap()),
                call(&format!("{id}-c7")),
                call(&format!("{id}-c8")),
            ],
            StopReason::AwaitingToolResults,
        ),
    )
}

pub(super) fn own_finished_turn(id: &str) -> Turn {
    multi_step_turn(id, Some(agent_a()))
}

/// The agent's own turn, its results in, waiting for its next step.
pub(super) fn own_awaiting_step_turn(id: &str) -> Turn {
    let mut turn = Turn::new(
        TurnId::new(id).unwrap(),
        Some(agent_a()),
        step(
            vec![thinking("mine"), call(&format!("{id}-c5"))],
            StopReason::AwaitingToolResults,
        ),
    );
    turn.push_result(result(&format!("{id}-c5"))).unwrap();
    turn
}

pub(super) fn conversation(entries: Vec<Entry>) -> Conversation {
    let mut conversation = Conversation::new();
    for entry in entries {
        match entry {
            Entry::UserInput(input) => conversation.push_input(input),
            Entry::Turn(turn) => conversation.push_turn(turn).unwrap(),
        }
    }
    conversation
}

pub(super) fn json<T: serde::Serialize>(value: &T) -> serde_json::Value {
    serde_json::to_value(value).unwrap()
}

pub(super) fn only_step(turn: &Turn) -> &Step {
    match turn.items() {
        [TurnItem::Step(step)] => step,
        other => panic!("expected exactly one step, got {other:?}"),
    }
}

pub(super) fn turn_at(conversation: &Conversation, index: usize) -> &Turn {
    match &conversation.entries()[index] {
        Entry::Turn(turn) => turn,
        Entry::UserInput(_) => panic!("expected a turn at {index}"),
    }
}

pub(super) fn assert_render_preserved(conversation: &Conversation, perspective: &Perspective) {
    let lightened = lighten(conversation, perspective);
    assert_eq!(
        render(&lightened, perspective),
        render(conversation, perspective)
    );
}
