use crate::value::{ToolCallId, ToolName, TurnId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TurnStateLabel {
    AwaitingToolResults,
    AwaitingStep,
    Finished,
}

impl std::fmt::Display for TurnStateLabel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let label = match self {
            TurnStateLabel::AwaitingToolResults => "awaiting_tool_results",
            TurnStateLabel::AwaitingStep => "awaiting_step",
            TurnStateLabel::Finished => "finished",
        };
        f.write_str(label)
    }
}

/// Every refusal of the crate: one variant per rule. It may grow in a later
/// version, so code that matches on it must allow for variants it does not
/// know.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum MessageError {
    Blank {
        field: &'static str,
    },
    InvalidToolName,
    InvalidBase64,
    EmptyStep,
    ToolCallNotAtTail,
    AwaitingToolResultsWithoutCalls,
    ToolCallsWithoutAwaiting,
    DuplicateToolCallId {
        id: ToolCallId,
    },
    DuplicateToolCallIdInTurn {
        id: ToolCallId,
    },
    DuplicateToolCallIdInConversation {
        id: ToolCallId,
    },
    EmptyUserContent,
    DuplicateToolResultId {
        id: ToolCallId,
    },
    ToolResultNotPending {
        given: ToolCallId,
        pending: Vec<ToolCallId>,
    },
    ToolNameMismatch {
        id: ToolCallId,
        call: ToolName,
        result: ToolName,
    },
    EmptyToolResults,
    TurnNotAwaitingResults {
        state: TurnStateLabel,
    },
    TurnNotAwaitingStep {
        state: TurnStateLabel,
    },
    TurnMustStartWithStep,
    /// An input or a turn whose id is already the id of an entry, input or
    /// turn: entry ids are one space, unique in the conversation.
    DuplicateEntryId {
        id: String,
    },
    TurnNotFound {
        id: TurnId,
    },
    SchemaMismatch {
        found: String,
    },
    TurnAwaitingResults,
    RenderStartsWithAssistant,
    EmptyRender,
    NoOpenBlock {
        index: usize,
    },
    BlockStillOpen {
        index: usize,
    },
    UnexpectedBlockIndex {
        expected: usize,
        given: usize,
    },
    BlockKindMismatch {
        index: usize,
    },
    ToolCallNeedsStart,
    EventAfterFinish,
    MissingFinish,
    InvalidJson {
        field: &'static str,
        message: String,
    },
    WireListEmpty,
    WireListNotStartingWithUser {
        message: usize,
    },
    WireToolCallUnanswered {
        message: usize,
        block: usize,
        id: ToolCallId,
    },
    WireToolCallAnsweredTwice {
        message: usize,
        block: usize,
        id: ToolCallId,
    },
    WireToolResultUnmatched {
        message: usize,
        block: usize,
        id: ToolCallId,
    },
    WireToolCallIdRepeated {
        message: usize,
        block: usize,
        id: ToolCallId,
    },
}

impl std::fmt::Display for MessageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MessageError::Blank { field } => write!(f, "{field} must not be blank"),
            MessageError::InvalidToolName => f.write_str(
                "tool name must be 1 to 64 ascii letters, digits, '_' or '-', the first a letter or '_'",
            ),
            MessageError::InvalidBase64 => f.write_str("value is not valid standard base64"),
            MessageError::EmptyStep => f.write_str("a step must carry at least one block"),
            MessageError::ToolCallNotAtTail => {
                f.write_str("tool-call blocks must be the tail of a step")
            }
            MessageError::AwaitingToolResultsWithoutCalls => {
                f.write_str("stop reason awaiting_tool_results requires at least one tool call")
            }
            MessageError::ToolCallsWithoutAwaiting => {
                f.write_str("tool calls require the stop reason awaiting_tool_results")
            }
            MessageError::DuplicateToolCallId { id } => {
                write!(f, "tool-call id {id} appears twice in the step")
            }
            MessageError::DuplicateToolCallIdInTurn { id } => {
                write!(f, "tool-call id {id} is already used by a step of the turn")
            }
            MessageError::DuplicateToolCallIdInConversation { id } => {
                write!(
                    f,
                    "tool-call id {id} is already used by a call of the conversation"
                )
            }
            MessageError::EmptyUserContent => {
                f.write_str("a user input must carry at least one block")
            }
            MessageError::DuplicateToolResultId { id } => {
                write!(f, "a result for tool-call id {id} was already collected")
            }
            MessageError::ToolResultNotPending { given, pending } => {
                let pending: Vec<&str> = pending.iter().map(ToolCallId::as_str).collect();
                write!(
                    f,
                    "tool-call id {given} is not pending; pending are [{}]",
                    pending.join(", ")
                )
            }
            MessageError::ToolNameMismatch { id, call, result } => write!(
                f,
                "the result for tool-call id {id} names tool {result}, the call names {call}"
            ),
            MessageError::EmptyToolResults => {
                f.write_str("a tool results item must carry at least one result")
            }
            MessageError::TurnNotAwaitingResults { state } => {
                write!(f, "the turn does not await results (state {state})")
            }
            MessageError::TurnNotAwaitingStep { state } => {
                write!(f, "the turn does not await a step (state {state})")
            }
            MessageError::TurnMustStartWithStep => {
                f.write_str("a turn must start with a step, not a results item")
            }
            MessageError::DuplicateEntryId { id } => {
                write!(
                    f,
                    "entry id {id} is already used by an entry of the conversation"
                )
            }
            MessageError::TurnNotFound { id } => {
                write!(f, "no turn with id {id} in the conversation")
            }
            MessageError::SchemaMismatch { found } => {
                write!(f, "unexpected schema {found:?}")
            }
            MessageError::TurnAwaitingResults => {
                f.write_str("the perspective's own last turn still awaits tool results")
            }
            MessageError::RenderStartsWithAssistant => {
                f.write_str("the rendered wire list must not start with an assistant message")
            }
            MessageError::EmptyRender => f.write_str("the rendered wire list must not be empty"),
            MessageError::NoOpenBlock { index } => {
                write!(f, "no open block at index {index}")
            }
            MessageError::BlockStillOpen { index } => {
                write!(f, "block at index {index} is still open")
            }
            MessageError::UnexpectedBlockIndex { expected, given } => {
                write!(f, "expected block index {expected}, got {given}")
            }
            MessageError::BlockKindMismatch { index } => {
                write!(f, "delta does not match the kind of block at index {index}")
            }
            MessageError::ToolCallNeedsStart => {
                f.write_str("a tool-call block must be opened with tool_call_start")
            }
            MessageError::EventAfterFinish => {
                f.write_str("no stream event is accepted after finish")
            }
            MessageError::MissingFinish => {
                f.write_str("the draft cannot finish before a finish event")
            }
            MessageError::InvalidJson { field, message } => {
                write!(f, "{field} is not valid json: {message}")
            }
            MessageError::WireListEmpty => f.write_str("the wire list must not be empty"),
            MessageError::WireListNotStartingWithUser { message } => write!(
                f,
                "the wire list must start with a user message (message {message})"
            ),
            MessageError::WireToolCallUnanswered { message, block, id } => write!(
                f,
                "tool call {id} (message {message}, block {block}) is not answered in the next user message"
            ),
            MessageError::WireToolCallAnsweredTwice { message, block, id } => write!(
                f,
                "tool call {id} is answered more than once (message {message}, block {block})"
            ),
            MessageError::WireToolResultUnmatched { message, block, id } => write!(
                f,
                "tool result {id} (message {message}, block {block}) answers no tool call of the assistant message just before it"
            ),
            MessageError::WireToolCallIdRepeated { message, block, id } => write!(
                f,
                "tool-call id {id} appears more than once in the wire list (message {message}, block {block})"
            ),
        }
    }
}

impl std::error::Error for MessageError {}
