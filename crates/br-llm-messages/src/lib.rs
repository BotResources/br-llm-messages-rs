//! Provider-neutral model of an LLM conversation. Every public type is
//! reached from the crate root, and only from it.

#![deny(unreachable_pub)]

mod block;
mod conversation;
mod error;
mod lighten;
mod render;
mod step;
mod stop_reason;
mod stream;
mod tool_results;
mod turn;
mod usage;
mod user_input;
mod value;

#[cfg(doctest)]
mod only_from_the_root;

#[cfg(test)]
mod test_walk;

pub const SCHEMA_VERSION: &str = "br-llm-messages/2";

pub use block::{
    AssistantBlock, Image, RedactedThinking, Thinking, ToolCall, ToolResult, ToolResultBlock,
    UserBlock,
};
pub use conversation::{Conversation, Entry};
pub use error::{MessageError, TurnStateLabel};
pub use lighten::lighten;
pub use render::{Perspective, WireMessage, WireUserBlock, check_wire_list, render};
pub use step::Step;
pub use stop_reason::StopReason;
pub use stream::{BlockKind, StepDraft, StreamEvent};
pub use tool_results::ToolResults;
pub use turn::{Turn, TurnItem, TurnState};
pub use usage::Usage;
pub use user_input::{UserInput, UserSource};
pub use value::{
    Author, AuthorId, AuthorName, Base64Data, ImageMime, ModelId, RuntimeKind, Signature, Text,
    ToolCallId, ToolName, TurnId,
};
