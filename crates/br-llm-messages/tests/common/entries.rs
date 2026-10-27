use br_llm_messages::{
    AgentMail, AgentRequest, ClosureReason, HumanMessage, ModelResponse, QueuePosition,
    RequestAccepted, RequestOrigin, ResponseEnding, ResponseMetadata, RuntimeNotice,
    RuntimeToolResults, TaskCompleted, TaskFailed, TaskStarted, TurnClosure, TurnEntry,
};

use super::{aid, author, cvid, eid, hid, kid, plain_text, result, rid};

pub fn human(n: u64) -> HumanMessage {
    HumanMessage::new(eid(n), hid(1), vec![plain_text(n + 100, "hi")]).unwrap()
}

pub fn all_entries() -> Vec<TurnEntry> {
    let content = || vec![plain_text(500, "x")];
    vec![
        human(1).into(),
        AgentRequest::new(
            eid(2),
            aid(1),
            author("A"),
            cvid(1),
            RequestOrigin::request(rid(1)),
            content(),
        )
        .unwrap()
        .into(),
        AgentMail::new(eid(3), aid(1), author("A"), cvid(1), content())
            .unwrap()
            .into(),
        RequestAccepted::new(eid(4), rid(1)).into(),
        QueuePosition::new(eid(5), rid(1), 1).unwrap().into(),
        TaskStarted::new(eid(6), rid(1), kid(1)).into(),
        TaskCompleted::new(eid(7), kid(1)).into(),
        TaskFailed::new(eid(8), kid(1), "boom").unwrap().into(),
        RuntimeNotice::new(eid(12), "restarting in 5 minutes")
            .unwrap()
            .into(),
        ModelResponse::new(
            eid(9),
            vec![],
            ResponseEnding::finished(),
            ResponseMetadata::new(),
        )
        .unwrap()
        .into(),
        RuntimeToolResults::new(eid(10), vec![result(600, 601, "t")])
            .unwrap()
            .into(),
        TurnClosure::new(eid(11), ClosureReason::completed()).into(),
    ]
}
