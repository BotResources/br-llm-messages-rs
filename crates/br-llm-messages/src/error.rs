use crate::value::{ToolCallId, TurnId};

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageError {
    Blank {
        field: &'static str,
    },
    InvalidBase64,
    EmptyStep,
    ToolCallNotAtTail,
    AwaitingToolResultsWithoutCalls,
    ToolCallsWithoutAwaiting,
    DuplicateToolCallId {
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
    TurnNotAwaitingResults {
        state: TurnStateLabel,
    },
    TurnNotAwaitingStep {
        state: TurnStateLabel,
    },
    TurnMustStartWithStep,
    DuplicateTurnId {
        id: TurnId,
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
}

impl std::fmt::Display for MessageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MessageError::Blank { field } => write!(f, "{field} must not be blank"),
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
            MessageError::TurnNotAwaitingResults { state } => {
                write!(f, "the turn does not await results (state {state})")
            }
            MessageError::TurnNotAwaitingStep { state } => {
                write!(f, "the turn does not await a step (state {state})")
            }
            MessageError::TurnMustStartWithStep => {
                f.write_str("a turn must start with a step, not a results item")
            }
            MessageError::DuplicateTurnId { id } => {
                write!(f, "turn id {id} already exists in the conversation")
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
        }
    }
}

impl std::error::Error for MessageError {}
