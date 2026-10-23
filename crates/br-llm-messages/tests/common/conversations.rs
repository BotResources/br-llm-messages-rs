use br_llm_messages::{
    AITurn, AgentMail, AgentRequest, ClosureReason, Conversation, ExecutionStatus, ExternalId,
    HumanMessage, InputTurn, ModelName, ModelResponse, PlainBlock, ProviderOperation,
    ProviderToolCall, ProviderToolExecutionRecord, ProviderToolResult, QueuePosition,
    RequestAccepted, RequestOrigin, ResponseEnding, ResponseMetadata, ResultExposure, ResultStatus,
    RuntimeToolCall, RuntimeToolResult, RuntimeToolResults, StructuredBlock, TaskStarted,
    TextBlock, TextRange, ThinkingBlock, ToolDefinition, ToolMounted, TurnClosure, TurnEntry,
    Usage,
};
use br_llm_messages::{Citation, CitationSource};
use serde_json::json;

use super::turns::tid;
use super::{aid, author, bid, cid, cvid, eid, hid, kid, plain_text, png, rid, signature, tool};

fn cited_text(block: u64, citation: u64) -> PlainBlock {
    let cite = Citation::new(
        cid(citation),
        CitationSource::new("doc-1")
            .unwrap()
            .with_title("A document"),
    )
    .with_range(TextRange::new(0, 5).unwrap());
    PlainBlock::try_from(
        TextBlock::new(bid(block), "cited text")
            .unwrap()
            .with_citations(vec![cite])
            .unwrap(),
    )
    .unwrap()
}

fn first_input() -> InputTurn {
    let message = HumanMessage::new(eid(2), hid(1), vec![cited_text(3, 4)]).unwrap();
    InputTurn::new(tid(1), message).unwrap()
}

fn lookup_response() -> TurnEntry {
    let call = RuntimeToolCall::new(bid(15), tool("lookup"), super::oj(json!({"q": "rust"})))
        .unwrap()
        .with_external_id(ExternalId::new("toolu_1").unwrap());
    let hosted = ProviderToolCall::new(bid(16))
        .with_operation(ProviderOperation::new("web_search").unwrap())
        .with_arguments(super::oj(json!({"query": "x"})))
        .unwrap();
    let hosted_result =
        ProviderToolResult::new(bid(17), bid(16), vec![plain_text(18, "found")]).unwrap();
    let record = ProviderToolExecutionRecord::new(
        bid(19),
        ResultExposure::readable(vec![plain_text(20, "log")]).unwrap(),
    )
    .unwrap()
    .with_status(ExecutionStatus::Completed);
    let usage = Usage::new()
        .with_input_tokens(10)
        .with_output_tokens(u64::MAX);
    let metadata = ResponseMetadata::new()
        .with_usage(usage)
        .with_model(ModelName::new("gpt-6.1").unwrap())
        .with_provider_response_id(ExternalId::new("resp_1").unwrap());
    ModelResponse::new(
        eid(12),
        vec![
            ThinkingBlock::new(bid(13), "")
                .with_replay(signature("sig"))
                .into(),
            TextBlock::new(bid(14), "calling").unwrap().into(),
            call.into(),
            hosted.into(),
            hosted_result.into(),
            record.into(),
        ],
        ResponseEnding::tool_results_requested(),
        metadata,
    )
    .unwrap()
    .into()
}

fn lookup_results() -> TurnEntry {
    let result = RuntimeToolResult::new(
        bid(22),
        bid(15),
        tool("lookup"),
        ResultStatus::Success,
        vec![
            cited_text(23, 31),
            PlainBlock::Image(png(24)),
            PlainBlock::StructuredContent(
                StructuredBlock::new(bid(25), super::oj(json!({"k": [1, 2]}))).unwrap(),
            ),
        ],
    )
    .unwrap();
    RuntimeToolResults::new(eid(21), vec![result])
        .unwrap()
        .into()
}

fn working_turn() -> AITurn {
    let definition = ToolDefinition::inline(
        tool("lookup"),
        "finds things",
        super::oj(json!({"type": "object"})),
    )
    .unwrap();
    let mail = AgentMail::new(
        eid(26),
        aid(2),
        author("Claude"),
        cvid(2),
        vec![plain_text(27, "fyi")],
    )
    .unwrap();
    let last = ModelResponse::new(
        eid(28),
        vec![TextBlock::new(bid(29), "done").unwrap().into()],
        ResponseEnding::finished(),
        ResponseMetadata::new(),
    )
    .unwrap();
    AITurn::from_parts(
        tid(10),
        aid(1),
        author("Codex GPT-6.1-Sol high"),
        vec![
            ToolMounted::new(eid(11), definition).into(),
            lookup_response(),
            lookup_results(),
            mail.into(),
            last.into(),
            TurnClosure::new(eid(30), ClosureReason::completed()).into(),
        ],
    )
    .unwrap()
}

fn request_turn() -> InputTurn {
    let request = AgentRequest::new(
        eid(41),
        aid(2),
        author("Claude"),
        cvid(2),
        RequestOrigin::tool_call(bid(900)),
        vec![plain_text(42, "please help")],
    )
    .unwrap();
    InputTurn::new(tid(40), request).unwrap()
}

fn second_agent_turn() -> AITurn {
    AITurn::from_parts(
        tid(50),
        aid(2),
        author("Claude"),
        vec![
            RequestAccepted::new(eid(51), rid(1)).into(),
            TaskStarted::new(eid(52), rid(1), kid(1))
                .with_conversation(cvid(3))
                .into(),
        ],
    )
    .unwrap()
}

pub fn rich() -> Conversation {
    let queue = QueuePosition::new(eid(61), rid(1), 2).unwrap();
    Conversation::from_parts(
        cvid(1000),
        vec![
            first_input().into(),
            working_turn().into(),
            request_turn().into(),
            second_agent_turn().into(),
            InputTurn::new(tid(60), queue).unwrap().into(),
        ],
    )
    .unwrap()
}
