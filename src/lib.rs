pub mod error;
pub mod value;

pub const SCHEMA_VERSION: &str = "br-llm-messages/1";

pub use error::{MessageError, TurnStateLabel};
pub use value::{
    Author, Base64Data, ImageMime, ModelId, Signature, Text, ToolCallId, ToolName, TurnId,
};
