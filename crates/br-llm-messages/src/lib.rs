#[macro_use]
mod tagged;

mod block;
mod conversation;
mod entry;
mod error;
mod id;
mod json;
mod opaque_json;
mod projection;
mod record;
mod stream;
mod strict_serializer;
mod turn;
mod value;

pub use block::{
    ContentBlock, ContentBlockKind, ImageBlock, PlainBlock, ProviderToolCall,
    ProviderToolExecutionRecord, ProviderToolResult, RedactedThinkingBlock, ResultExposure,
    RuntimeToolCall, RuntimeToolResult, StructuredBlock, TextBlock, ThinkingBlock,
};
pub use conversation::{
    BlockRef, CONVERSATION_SCHEMA, Conversation, EntryRef, EntryView, PlacedEntry, Position,
};
pub use entry::{
    AgentMail, AgentOutput, AgentRequest, ClosureReason, HumanMessage, IncomingEntry,
    ModelResponse, PartialAgentOutput, QueuePosition, RequestAccepted, RequestOrigin,
    ResponseMetadata, RuntimeNotice, RuntimeToolResults, TaskCompleted, TaskFailed, TaskStarted,
    TurnClosure, TurnEntry, TurnEntryKind,
};
pub use error::{Error, ErrorCode};
pub use id::{
    AgentId, BlockId, CitationId, ConversationId, EntryId, HumanId, RequestId, TaskId, TurnId,
};
pub use opaque_json::OpaqueJson;
pub use projection::{PROJECTED_THREAD_SCHEMA, PendingResponse, Perspective, ProjectedThread};
pub use record::{
    ActorRef, BlockRecord, EntryRecord, RECORD_SCHEMA, RecordSet, Recovered, RepresentationState,
    TurnRecord, Unfinished,
};
pub use stream::{
    BlockMetadataUpdate, BlockStart, DELTA_SCHEMA, DRAFT_SCHEMA, DeltaOp, DeltaOutcome,
    DeltaStream, DraftPhase, DraftSnapshot, DraftView, Finalization, InterruptionKind,
    InterruptionReason, ResponseDelta, ResponseDeltaKind, ResponseDraft, ResponseMetadataUpdate,
    SlotLifecycle, SlotView,
};
pub use turn::{AITurn, InputTurn, Turn, TurnKind, TurnState};
pub use uuid::Uuid;
pub use value::{
    Author, Base64Data, Citation, CitationSource, ExecutionStatus, ExternalId, ImageSource,
    ItemRef, MediaType, ModelName, ObservedAt, PlainText, ProviderOperation, ReplayFormat,
    ReplayMaterial, ReplayPayload, ResponseEnding, TextRange, ToolName, Usage,
};
