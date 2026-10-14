//! The public surface as an importing crate sees it: every public type is
//! reached from the crate root; the enums that may grow are
//! `#[non_exhaustive]` and every other public enum is closed; JSON is the
//! only stored form.

#![deny(unreachable_patterns)]

use std::any::type_name;
use std::marker::PhantomData;

use br_llm_messages::{
    AssistantBlock, Author, AuthorId, AuthorName, Base64Data, BlockKind, Conversation, Entry,
    Image, ImageMime, MessageError, ModelId, Perspective, RedactedThinking, RuntimeKind,
    SCHEMA_VERSION, Signature, Step, StepDraft, StopReason, StreamEvent, Text, Thinking, ToolCall,
    ToolCallId, ToolName, ToolResult, ToolResultBlock, ToolResults, Turn, TurnId, TurnItem,
    TurnState, TurnStateLabel, Usage, UserBlock, UserInput, UserSource, WireMessage, WireUserBlock,
    check_wire_list, lighten, render,
};

/// Whether `$ty` implements `$bound`, decided at compile time by method
/// resolution (autoref specialization), for a concrete type.
macro_rules! implements {
    ($ty:ty: $($bound:tt)+) => {{
        struct Probe<T>(PhantomData<T>);
        #[allow(dead_code)]
        trait Yes {
            fn answer(&self) -> bool {
                true
            }
        }
        impl<T: $($bound)+> Yes for Probe<T> {}
        #[allow(dead_code)]
        trait No {
            fn answer(&self) -> bool {
                false
            }
        }
        impl<T> No for &Probe<T> {}
        (&Probe::<$ty>(PhantomData)).answer()
    }};
}

#[test]
fn given_the_crate_root_when_every_public_type_is_named_then_each_resolves() {
    let names = [
        type_name::<AssistantBlock>(),
        type_name::<Author>(),
        type_name::<AuthorId>(),
        type_name::<AuthorName>(),
        type_name::<Base64Data>(),
        type_name::<BlockKind>(),
        type_name::<Conversation>(),
        type_name::<Entry>(),
        type_name::<Image>(),
        type_name::<ImageMime>(),
        type_name::<MessageError>(),
        type_name::<ModelId>(),
        type_name::<Perspective>(),
        type_name::<RedactedThinking>(),
        type_name::<RuntimeKind>(),
        type_name::<Signature>(),
        type_name::<Step>(),
        type_name::<StepDraft>(),
        type_name::<StopReason>(),
        type_name::<StreamEvent>(),
        type_name::<Text>(),
        type_name::<Thinking>(),
        type_name::<ToolCall>(),
        type_name::<ToolCallId>(),
        type_name::<ToolName>(),
        type_name::<ToolResult>(),
        type_name::<ToolResultBlock>(),
        type_name::<ToolResults>(),
        type_name::<Turn>(),
        type_name::<TurnId>(),
        type_name::<TurnItem>(),
        type_name::<TurnState>(),
        type_name::<TurnStateLabel>(),
        type_name::<Usage>(),
        type_name::<UserBlock>(),
        type_name::<UserInput>(),
        type_name::<UserSource>(),
        type_name::<WireMessage>(),
        type_name::<WireUserBlock>(),
    ];
    assert!(
        names
            .iter()
            .all(|name| name.starts_with("br_llm_messages::"))
    );
    let functions: [&str; 3] = [
        type_name_of(&check_wire_list),
        type_name_of(&lighten),
        type_name_of(&render),
    ];
    assert!(
        functions
            .iter()
            .all(|name| name.contains("br_llm_messages"))
    );
    assert_eq!(SCHEMA_VERSION, "br-llm-messages/2");
}

fn type_name_of<T>(_: &T) -> &'static str {
    type_name::<T>()
}

/// Exhaustive matches without a wildcard: they compile only while these
/// enums are closed.
#[test]
fn given_closed_enums_when_matched_exhaustively_then_no_wildcard_is_needed() {
    let closed: [fn() -> &'static str; 10] = [
        || match UserBlock::text(Text::new("x").unwrap()) {
            UserBlock::Text { .. } | UserBlock::Image(_) => "user block",
        },
        || match (WireMessage::Relay { content: vec![] }) {
            WireMessage::User { .. }
            | WireMessage::Assistant { .. }
            | WireMessage::Relay { .. } => "wire message",
        },
        || match (WireUserBlock::Text {
            text: Text::new("x").unwrap(),
        }) {
            WireUserBlock::Text { .. } | WireUserBlock::Image(_) | WireUserBlock::ToolResult(_) => {
                "wire user block"
            }
        },
        || match StopReason::EndTurn {
            StopReason::EndTurn
            | StopReason::AwaitingToolResults
            | StopReason::MaxTokens
            | StopReason::StopSequence
            | StopReason::Refusal
            | StopReason::ContentFilter
            | StopReason::Other { .. } => "stop reason",
        },
        || match TurnState::AwaitingStep {
            TurnState::AwaitingToolResults { .. }
            | TurnState::AwaitingStep
            | TurnState::Finished { .. } => "turn state",
        },
        || match TurnStateLabel::Finished {
            TurnStateLabel::AwaitingToolResults
            | TurnStateLabel::AwaitingStep
            | TurnStateLabel::Finished => "turn state label",
        },
        || match TurnItem::ToolResults(ToolResults::empty()) {
            TurnItem::Step(_) | TurnItem::ToolResults(_) => "turn item",
        },
        || match Entry::UserInput(
            UserInput::new(
                UserSource::Human,
                None,
                vec![UserBlock::text(Text::new("x").unwrap())],
            )
            .unwrap(),
        ) {
            Entry::UserInput(_) | Entry::Turn(_) => "entry",
        },
        || match UserSource::Human {
            UserSource::Human | UserSource::Runtime { .. } => "user source",
        },
        || match ImageMime::Png {
            ImageMime::Jpeg | ImageMime::Png | ImageMime::Gif | ImageMime::Webp => "image mime",
        },
    ];
    assert!(closed.iter().all(|matcher| !matcher().is_empty()));
}

/// Matches over every known variant plus a wildcard: under
/// `deny(unreachable_patterns)` they compile only while these enums are
/// `#[non_exhaustive]`.
#[test]
fn given_enums_that_may_grow_when_matched_then_a_wildcard_is_required() {
    let error = match MessageError::EmptyStep {
        MessageError::EmptyStep => "known",
        _ => "unknown",
    };
    let kind = match BlockKind::Text {
        BlockKind::Text
        | BlockKind::Thinking
        | BlockKind::RedactedThinking
        | BlockKind::Structured
        | BlockKind::ToolCall => "known",
        _ => "unknown",
    };
    let event = match (StreamEvent::BlockEnd { index: 0 }) {
        StreamEvent::BlockStart { .. }
        | StreamEvent::TextDelta { .. }
        | StreamEvent::ThinkingDelta { .. }
        | StreamEvent::SignatureDelta { .. }
        | StreamEvent::RedactedThinkingData { .. }
        | StreamEvent::StructuredDelta { .. }
        | StreamEvent::ToolCallStart { .. }
        | StreamEvent::ToolCallArgumentsDelta { .. }
        | StreamEvent::BlockEnd { .. }
        | StreamEvent::Finish { .. } => "known",
        _ => "unknown",
    };
    let block = match (AssistantBlock::Structured {
        value: serde_json::json!(1),
    }) {
        AssistantBlock::Text { .. }
        | AssistantBlock::Thinking(_)
        | AssistantBlock::RedactedThinking(_)
        | AssistantBlock::Structured { .. }
        | AssistantBlock::ToolCall(_) => "known",
        _ => "unknown",
    };
    let result_block = match ToolResultBlock::text(Text::new("x").unwrap()) {
        ToolResultBlock::Text { .. } => "known",
        _ => "unknown",
    };
    assert_eq!([error, kind, event, block, result_block], ["known"; 5]);
}

#[test]
fn given_the_stored_objects_when_probed_then_json_is_their_only_stored_form() {
    assert!(!implements!(Conversation: std::fmt::Display));
    assert!(!implements!(Turn: std::fmt::Display));
    assert!(!implements!(UserInput: std::fmt::Display));

    for (stored, serialize, deserialize) in [
        (
            "conversation",
            implements!(Conversation: serde::Serialize),
            implements!(Conversation: serde::de::DeserializeOwned),
        ),
        (
            "turn",
            implements!(Turn: serde::Serialize),
            implements!(Turn: serde::de::DeserializeOwned),
        ),
        (
            "user input",
            implements!(UserInput: serde::Serialize),
            implements!(UserInput: serde::de::DeserializeOwned),
        ),
        (
            "step",
            implements!(Step: serde::Serialize),
            implements!(Step: serde::de::DeserializeOwned),
        ),
        (
            "tool result",
            implements!(ToolResult: serde::Serialize),
            implements!(ToolResult: serde::de::DeserializeOwned),
        ),
    ] {
        assert!(serialize && deserialize, "{stored}");
    }
}

#[test]
fn given_computed_and_transient_values_when_probed_then_not_loaded_or_not_serialized() {
    assert!(!implements!(TurnState: serde::de::DeserializeOwned));
    assert!(!implements!(Perspective: serde::de::DeserializeOwned));
    assert!(!implements!(BlockKind: serde::de::DeserializeOwned));
    assert!(!implements!(StreamEvent: serde::Serialize));
    assert!(implements!(StreamEvent: serde::de::DeserializeOwned));
}
