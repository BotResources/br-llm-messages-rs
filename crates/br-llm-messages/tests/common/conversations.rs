use br_llm_messages::{
    AITurn, AgentMail, AgentRequest, ClosureReason, Conversation, ExecutionStatus, ExternalId,
    HumanMessage, InputTurn, ItemRef, ModelName, ModelResponse, PlainBlock, ProviderOperation,
    ProviderToolCall, ProviderToolExecutionRecord, ProviderToolResult, QueuePosition,
    RequestAccepted, RequestOrigin, ResponseEnding, ResponseMetadata, RuntimeToolCall,
    RuntimeToolResult, RuntimeToolResults, StructuredBlock, TaskStarted, TextBlock, TextRange,
    ThinkingBlock, TurnClosure, TurnEntry, Usage,
};
use br_llm_messages::{Citation, CitationSource};
use serde_json::json;

use super::turns::tid;
use super::{
    aid, at, author, bid, cid, cvid, eid, hid, kid, plain_text, png, rid, signature, tool,
};

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
            .unwrap()
            .with_observed_at(at(1)),
    )
    .unwrap()
}

fn first_input() -> InputTurn {
    let message = HumanMessage::new(eid(2), hid(1), vec![cited_text(3, 4)])
        .unwrap()
        .with_observed_at(at(2));
    InputTurn::new(tid(1), message)
        .unwrap()
        .with_observed_at(at(3))
}

fn lookup_response() -> TurnEntry {
    let call = RuntimeToolCall::new(bid(15), tool("lookup"), super::oj(json!({"q": "rust"})))
        .unwrap()
        .with_external_id(ExternalId::new("toolu_1").unwrap())
        .with_item_ref(ItemRef::new("fc_1").unwrap())
        .with_observed_at(at(6));
    let hosted = ProviderToolCall::new(bid(16))
        .with_operation(ProviderOperation::new("web_search").unwrap())
        .with_arguments(super::oj(json!({"query": "x"})))
        .unwrap()
        .with_observed_at(at(7));
    let hosted_result = ProviderToolResult::new(bid(17), bid(16), vec![plain_text(18, "found")])
        .unwrap()
        .with_observed_at(at(8));
    let record = ProviderToolExecutionRecord::new(bid(19))
        .with_content(vec![plain_text(20, "log")])
        .unwrap()
        .with_status(ExecutionStatus::Completed)
        .with_observed_at(at(9));
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
            ThinkingBlock::replay_only(bid(13), signature("sig"))
                .with_observed_at(at(4))
                .into(),
            TextBlock::new(bid(14), "calling")
                .unwrap()
                .with_item_ref(ItemRef::new("msg_1").unwrap())
                .with_observed_at(at(5))
                .into(),
            call.into(),
            hosted.into(),
            hosted_result.into(),
            record.into(),
        ],
        ResponseEnding::tool_use(),
        metadata.with_observed_at(at(10)),
    )
    .unwrap()
    .into()
}

fn lookup_results() -> TurnEntry {
    let result = RuntimeToolResult::new(
        bid(22),
        bid(15),
        tool("lookup"),
        false,
        vec![
            cited_text(23, 31),
            PlainBlock::Image(png(24).with_observed_at(at(12))),
            PlainBlock::StructuredContent(
                StructuredBlock::new(bid(25), super::oj(json!({"k": [1, 2]})))
                    .unwrap()
                    .with_observed_at(at(13)),
            ),
        ],
    )
    .unwrap()
    .with_observed_at(at(11));
    RuntimeToolResults::new(eid(21), vec![result])
        .unwrap()
        .with_observed_at(at(14))
        .into()
}

fn working_turn() -> AITurn {
    let mail = AgentMail::new(
        eid(26),
        aid(2),
        author("Claude"),
        cvid(2),
        vec![plain_text(27, "fyi")],
    )
    .unwrap()
    .with_observed_at(at(15));
    let last = ModelResponse::new(
        eid(28),
        vec![
            TextBlock::new(bid(29), "done")
                .unwrap()
                .with_observed_at(at(16))
                .into(),
        ],
        ResponseEnding::finished(),
        ResponseMetadata::new().with_observed_at(at(17)),
    )
    .unwrap();
    AITurn::from_parts(
        tid(10),
        aid(1),
        author("Codex GPT-6.1-Sol high"),
        vec![
            lookup_response(),
            lookup_results(),
            mail.into(),
            last.into(),
            TurnClosure::new(eid(30), ClosureReason::completed())
                .with_observed_at(at(18))
                .into(),
        ],
    )
    .unwrap()
    .with_observed_at(at(19))
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
    .unwrap()
    .with_observed_at(at(20));
    InputTurn::new(tid(40), request)
        .unwrap()
        .with_observed_at(at(21))
}

fn second_agent_turn() -> AITurn {
    AITurn::from_parts(
        tid(50),
        aid(2),
        author("Claude"),
        vec![
            RequestAccepted::new(eid(51), rid(1))
                .with_observed_at(at(22))
                .into(),
            TaskStarted::new(eid(52), rid(1), kid(1))
                .with_conversation(cvid(3))
                .with_observed_at(at(23))
                .into(),
        ],
    )
    .unwrap()
    .with_observed_at(at(24))
}

pub fn rich() -> Conversation {
    let queue = QueuePosition::new(eid(61), rid(1), 2)
        .unwrap()
        .with_observed_at(at(25));
    Conversation::from_parts(
        cvid(1000),
        vec![
            first_input().into(),
            working_turn().into(),
            request_turn().into(),
            second_agent_turn().into(),
            InputTurn::new(tid(60), queue)
                .unwrap()
                .with_observed_at(at(26))
                .into(),
        ],
    )
    .unwrap()
}
