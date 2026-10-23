use br_llm_messages::{
    AITurn, AgentMail, AgentRequest, ClosureReason, ContentBlock, ExecutionStatus, FailureDetail,
    HumanMessage, InputTurn, ModelResponse, ProviderToolCall, ProviderToolExecutionRecord,
    ProviderToolResult, QueuePosition, RequestAccepted, RequestOrigin, ResponseEnding,
    ResponseMetadata, ResultExposure, ResultStatus, RuntimeToolResult, RuntimeToolResults,
    TaskCompleted, TaskFailed, TaskStarted, ToolDefinition, ToolMounted, ToolUnmounted,
    TurnClosure, TurnEntry, TurnId,
};

use super::{aid, author, bid, call, cvid, eid, hid, kid, plain_text, rid, tool, v7};

pub fn tid(n: u64) -> TurnId {
    TurnId::from_uuid(v7(n)).unwrap()
}

pub fn open_turn(n: u64) -> AITurn {
    AITurn::open(tid(n), aid(1), author("Codex GPT-6.1-Sol high"))
}

pub fn input_turn(turn: u64, entry: u64) -> InputTurn {
    InputTurn::new(tid(turn), human_message(entry)).unwrap()
}

pub fn human_message(n: u64) -> HumanMessage {
    HumanMessage::new(eid(n), hid(1), vec![plain_text(n + 5000, "hello")]).unwrap()
}

pub fn human(n: u64) -> TurnEntry {
    human_message(n).into()
}

pub fn response(entry: u64, blocks: Vec<ContentBlock>, ending: ResponseEnding) -> TurnEntry {
    ModelResponse::new(eid(entry), blocks, ending, ResponseMetadata::new())
        .unwrap()
        .into()
}

pub fn finished(entry: u64) -> TurnEntry {
    response(entry, vec![], ResponseEnding::finished())
}

pub fn calling(entry: u64, calls: &[(u64, &str)]) -> TurnEntry {
    let blocks = calls
        .iter()
        .map(|(id, name)| call(*id, name).into())
        .collect();
    response(entry, blocks, ResponseEnding::tool_results_requested())
}

pub fn delivery(entry: u64, results: &[(u64, u64, &str)]) -> TurnEntry {
    let results = results
        .iter()
        .map(|(id, target, name)| {
            RuntimeToolResult::new(
                bid(*id),
                bid(*target),
                tool(name),
                ResultStatus::Success,
                vec![],
            )
            .unwrap()
        })
        .collect();
    RuntimeToolResults::new(eid(entry), results).unwrap().into()
}

pub fn close(entry: u64, reason: ClosureReason) -> TurnEntry {
    TurnClosure::new(eid(entry), reason).into()
}

pub fn mount(entry: u64, name: &str) -> TurnEntry {
    ToolMounted::new(
        eid(entry),
        ToolDefinition::versioned(tool(name), "1").unwrap(),
    )
    .into()
}

pub fn unmount(entry: u64, mounting: u64) -> TurnEntry {
    ToolUnmounted::new(eid(entry), eid(mounting)).into()
}

pub fn hosted_call(n: u64) -> ContentBlock {
    ProviderToolCall::new(bid(n)).into()
}

pub fn hosted_result(n: u64, call: u64) -> ContentBlock {
    ProviderToolResult::new(bid(n), bid(call), vec![plain_text(n + 3000, "found")])
        .unwrap()
        .into()
}

pub fn hosted_record(n: u64, status: ExecutionStatus) -> ContentBlock {
    ProviderToolExecutionRecord::new(bid(n), ResultExposure::not_exposed())
        .unwrap()
        .with_status(status)
        .into()
}

pub fn pushed(turn: &mut AITurn, entries: Vec<TurnEntry>) {
    for entry in entries {
        turn.push_entry(entry).unwrap();
    }
}

pub fn incoming_batch(base: u64) -> Vec<TurnEntry> {
    let content = |k: u64| vec![plain_text(base + 500 + k, "x")];
    vec![
        HumanMessage::new(eid(base), hid(1), content(0))
            .unwrap()
            .into(),
        AgentRequest::new(
            eid(base + 1),
            aid(3),
            author("A"),
            cvid(1),
            RequestOrigin::request(rid(1)),
            content(1),
        )
        .unwrap()
        .into(),
        AgentMail::new(eid(base + 2), aid(3), author("A"), cvid(1), content(2))
            .unwrap()
            .into(),
        RequestAccepted::new(eid(base + 3), rid(1)).into(),
        QueuePosition::new(eid(base + 4), rid(1), 1).unwrap().into(),
        TaskStarted::new(eid(base + 5), rid(1), kid(1)).into(),
        TaskCompleted::new(eid(base + 6), kid(1)).into(),
        TaskFailed::new(eid(base + 7), kid(1), FailureDetail::new("boom").unwrap()).into(),
    ]
}
