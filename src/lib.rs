pub mod block;
pub mod conversation;
pub mod error;
pub mod step;
pub mod stop_reason;
pub mod stream;
pub mod tool_results;
pub mod turn;
pub mod usage;
pub mod user_input;
pub mod value;

pub const SCHEMA_VERSION: &str = "br-llm-messages/1";

pub use block::{
    AssistantBlock, Image, RedactedThinking, Thinking, ToolCall, ToolResult, ToolResultBlock,
    UserBlock,
};
pub use conversation::{Conversation, Entry};
pub use error::{MessageError, TurnStateLabel};
pub use step::Step;
pub use stop_reason::StopReason;
pub use stream::{BlockKind, StepDraft, StreamEvent};
pub use tool_results::ToolResults;
pub use turn::{Turn, TurnItem, TurnState};
pub use usage::Usage;
pub use user_input::{UserInput, UserSource};
pub use value::{
    Author, Base64Data, ImageMime, ModelId, Signature, Text, ToolCallId, ToolName, TurnId,
};
