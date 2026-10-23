use std::fmt::Debug;

use br_llm_messages::{
    AITurn, AgentMail, AgentRequest, ClosureReason, ContentBlock, Conversation, HumanMessage,
    ImageBlock, IncomingEntry, InputTurn, ModelResponse, OpaqueJson, PlainBlock, ProviderToolCall,
    ProviderToolExecutionRecord, ProviderToolResult, QueuePosition, RedactedThinkingBlock,
    RequestAccepted, RuntimeToolCall, RuntimeToolResult, RuntimeToolResults, StructuredBlock,
    TaskCompleted, TaskFailed, TaskStarted, TextBlock, ThinkingBlock, ToolMounted, ToolUnmounted,
    Turn, TurnClosure, TurnEntry, TurnState,
};

fn assert_value_traits<T: Clone + PartialEq + Debug + Send + Sync>() {}

macro_rules! assert_all {
    ($($ty:ty),+ $(,)?) => {
        $(assert_value_traits::<$ty>();)+
    };
}

#[test]
fn given_the_conversation_types_when_checked_statically_then_they_are_clone_eq_debug_send_sync() {
    assert_all!(
        Conversation,
        OpaqueJson,
        Turn,
        InputTurn,
        AITurn,
        TurnState,
        ClosureReason
    );
}

#[test]
fn given_every_entry_type_when_checked_statically_then_it_is_clone_eq_debug_send_sync() {
    assert_all!(
        TurnEntry,
        IncomingEntry,
        HumanMessage,
        AgentRequest,
        AgentMail,
        RequestAccepted,
        QueuePosition,
        TaskStarted,
        TaskCompleted,
        TaskFailed,
        ModelResponse,
        RuntimeToolResults,
        ToolMounted,
        ToolUnmounted,
        TurnClosure,
    );
}

#[test]
fn given_every_block_type_when_checked_statically_then_it_is_clone_eq_debug_send_sync() {
    assert_all!(
        ContentBlock,
        PlainBlock,
        TextBlock,
        ImageBlock,
        StructuredBlock,
        ThinkingBlock,
        RedactedThinkingBlock,
        RuntimeToolCall,
        RuntimeToolResult,
        ProviderToolCall,
        ProviderToolResult,
        ProviderToolExecutionRecord,
    );
}
