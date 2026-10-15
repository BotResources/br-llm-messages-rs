//! Doctests only (`cfg(doctest)`): every public type is reached from the
//! crate root, and only from it. Each module that was public up to 0.3 is
//! private: naming an item through it does not compile (E0603, private
//! module; rustdoc checks the code itself only on nightly), while the same
//! items named from the root compile. CI runs them with `cargo test --doc`.
//!
//! ```
//! use br_llm_messages::{
//!     Conversation, Image, InputId, MessageError, Step, StopReason, StreamEvent, Text,
//!     ToolResults, Turn, Usage, UserInput, WireMessage, lighten,
//! };
//! let _ = lighten;
//! fn _named(
//!     _: Conversation, _: Image, _: InputId, _: MessageError, _: Step, _: StopReason,
//!     _: StreamEvent, _: Text, _: ToolResults, _: Turn, _: Usage, _: UserInput, _: WireMessage,
//! ) {}
//! ```
//!
//! ```compile_fail,E0603
//! use br_llm_messages::block::Image;
//! ```
//!
//! ```compile_fail,E0603
//! use br_llm_messages::conversation::Conversation;
//! ```
//!
//! ```compile_fail,E0603
//! use br_llm_messages::error::MessageError;
//! ```
//!
//! ```compile_fail,E0603
//! use br_llm_messages::lighten::lighten;
//! ```
//!
//! ```compile_fail,E0603
//! use br_llm_messages::render::WireMessage;
//! ```
//!
//! ```compile_fail,E0603
//! use br_llm_messages::step::Step;
//! ```
//!
//! ```compile_fail,E0603
//! use br_llm_messages::stop_reason::StopReason;
//! ```
//!
//! ```compile_fail,E0603
//! use br_llm_messages::stream::StreamEvent;
//! ```
//!
//! ```compile_fail,E0603
//! use br_llm_messages::tool_results::ToolResults;
//! ```
//!
//! ```compile_fail,E0603
//! use br_llm_messages::turn::Turn;
//! ```
//!
//! ```compile_fail,E0603
//! use br_llm_messages::usage::Usage;
//! ```
//!
//! ```compile_fail,E0603
//! use br_llm_messages::user_input::UserInput;
//! ```
//!
//! ```compile_fail,E0603
//! use br_llm_messages::value::Text;
//! ```
//!
//! ```compile_fail,E0603
//! use br_llm_messages::value::InputId;
//! ```
