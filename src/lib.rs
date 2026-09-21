pub mod block;
pub mod error;
pub mod step;
pub mod stop_reason;
pub mod usage;
pub mod value;

pub const SCHEMA_VERSION: &str = "br-llm-messages/1";

pub use block::{
    AssistantBlock, Image, RedactedThinking, Thinking, ToolCall, ToolResult, ToolResultBlock,
    UserBlock,
};
pub use error::{MessageError, TurnStateLabel};
pub use step::Step;
pub use stop_reason::StopReason;
pub use usage::Usage;
pub use value::{
    Author, Base64Data, ImageMime, ModelId, Signature, Text, ToolCallId, ToolName, TurnId,
};
