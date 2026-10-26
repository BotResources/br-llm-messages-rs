mod common;
use common::Raw as _;

use std::fmt::Debug;

use br_llm_messages::{
    AITurn, AgentMail, AgentRequest, ClosureReason, ErrorCode, HumanMessage, ImageBlock, InputTurn,
    ModelResponse, ObservedAt, PlainBlock, ProviderToolCall, ProviderToolExecutionRecord,
    ProviderToolResult, QueuePosition, RedactedThinkingBlock, RequestAccepted, RequestOrigin,
    ResponseEnding, ResponseMetadata, RuntimeToolCall, RuntimeToolResult, RuntimeToolResults,
    StructuredBlock, TaskCompleted, TaskFailed, TaskStarted, TextBlock, ThinkingBlock, TurnClosure,
};
use common::turns::tid;
use common::{
    aid, at, author, bid, call, cvid, decode, eid, hid, kid, oj, plain_text, png, result, rid,
    signature, text_block,
};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

const BAD_TIMES: [&str; 5] = [
    "2026-10-23T09:00:00+01:00",
    "2026-10-23t08:00:00Z",
    "2026-10-23T08:00:00z",
    "2026-02-30T08:00:00Z",
    "2026-10-23T08:00:00.1234567890Z",
];

fn json_of<T: Serialize>(value: &T) -> Value {
    oj(serde_json::to_value(value).unwrap()).raw()
}

fn verify<T>(
    label: &str,
    build: impl Fn(Option<ObservedAt>) -> T,
    get: impl Fn(&T) -> Option<&ObservedAt>,
) where
    T: Serialize + DeserializeOwned + PartialEq + Debug,
{
    let stamped = build(Some(at(7)));
    let plain = build(None);

    assert_eq!(get(&stamped), Some(&at(7)), "{label}: getter");
    assert_eq!(get(&plain), None, "{label}: absent getter");
    assert_ne!(stamped, plain, "{label}: part of equality");
    let document = json_of(&stamped);
    assert_eq!(document["observed_at"], at(7).as_str(), "{label}: wire");
    assert!(
        json_of(&plain).get("observed_at").is_none(),
        "{label}: absent on the wire"
    );
    let reloaded: T = decode(&document.to_string()).unwrap();
    assert_eq!(reloaded, stamped, "{label}: round trip");
    assert_eq!(json_of(&reloaded), document, "{label}: stable");
    for bad in BAD_TIMES
        .into_iter()
        .map(Value::from)
        .chain([json!(1), json!(true)])
    {
        let mut broken = document.clone();
        broken["observed_at"] = bad.clone();
        let error = decode::<T>(&broken.to_string()).err();
        assert_eq!(
            error.map(|e| e.code()),
            Some(ErrorCode::InvalidContent),
            "{label}: {bad}"
        );
    }
}

macro_rules! stamp {
    ($value:expr, $stamp:expr) => {
        match $stamp {
            Some(time) => $value.with_observed_at(time),
            None => $value,
        }
    };
}

fn content() -> Vec<PlainBlock> {
    vec![plain_text(2, "x")]
}

#[test]
fn given_every_incoming_entry_kind_when_stamped_then_it_serializes_loads_back_and_refuses_bad_times()
 {
    verify(
        "human_message",
        |s| stamp!(HumanMessage::new(eid(1), hid(1), content()).unwrap(), s),
        HumanMessage::observed_at,
    );
    verify(
        "agent_request",
        |s| {
            let request = AgentRequest::new(
                eid(1),
                aid(1),
                author("A"),
                cvid(1),
                RequestOrigin::request(rid(1)),
                content(),
            )
            .unwrap();
            stamp!(request, s)
        },
        AgentRequest::observed_at,
    );
    verify(
        "agent_mail",
        |s| {
            let mail = AgentMail::new(eid(1), aid(1), author("A"), cvid(1), content()).unwrap();
            stamp!(mail, s)
        },
        AgentMail::observed_at,
    );
    verify(
        "request_accepted",
        |s| stamp!(RequestAccepted::new(eid(1), rid(1)), s),
        RequestAccepted::observed_at,
    );
    verify(
        "queue_position",
        |s| stamp!(QueuePosition::new(eid(1), rid(1), 1).unwrap(), s),
        QueuePosition::observed_at,
    );
    verify(
        "task_started",
        |s| stamp!(TaskStarted::new(eid(1), rid(1), kid(1)), s),
        TaskStarted::observed_at,
    );
    verify(
        "task_completed",
        |s| stamp!(TaskCompleted::new(eid(1), kid(1)), s),
        TaskCompleted::observed_at,
    );
    verify(
        "task_failed",
        |s| stamp!(TaskFailed::new(eid(1), kid(1), "boom").unwrap(), s),
        TaskFailed::observed_at,
    );
}

#[test]
fn given_every_other_entry_kind_when_stamped_then_it_serializes_loads_back_and_refuses_bad_times() {
    verify(
        "model_response",
        |s| {
            let metadata = stamp!(ResponseMetadata::new(), s);
            ModelResponse::new(
                eid(1),
                vec![text_block(2, "x").into()],
                ResponseEnding::finished(),
                metadata,
            )
            .unwrap()
        },
        ModelResponse::observed_at,
    );
    verify(
        "runtime_tool_results",
        |s| {
            stamp!(
                RuntimeToolResults::new(eid(1), vec![result(2, 3, "a")]).unwrap(),
                s
            )
        },
        RuntimeToolResults::observed_at,
    );
    verify(
        "turn_closure",
        |s| stamp!(TurnClosure::new(eid(1), ClosureReason::completed()), s),
        TurnClosure::observed_at,
    );
}

#[test]
fn given_every_block_kind_when_stamped_then_it_serializes_loads_back_and_refuses_bad_times() {
    verify(
        "text",
        |s| stamp!(text_block(1, "x"), s),
        TextBlock::observed_at,
    );
    verify("image", |s| stamp!(png(1), s), ImageBlock::observed_at);
    verify(
        "structured_content",
        |s| {
            stamp!(
                StructuredBlock::new(bid(1), oj(json!({"a": 1}))).unwrap(),
                s
            )
        },
        StructuredBlock::observed_at,
    );
    verify(
        "thinking",
        |s| stamp!(ThinkingBlock::new(bid(1), "plan").unwrap(), s),
        ThinkingBlock::observed_at,
    );
    verify(
        "redacted_thinking",
        |s| stamp!(RedactedThinkingBlock::new(bid(1), signature("s")), s),
        RedactedThinkingBlock::observed_at,
    );
    verify(
        "runtime_tool_call",
        |s| stamp!(call(1, "lookup"), s),
        RuntimeToolCall::observed_at,
    );
    verify(
        "runtime_tool_result",
        |s| stamp!(result(1, 2, "lookup"), s),
        RuntimeToolResult::observed_at,
    );
    verify(
        "provider_tool_call",
        |s| stamp!(ProviderToolCall::new(bid(1)), s),
        ProviderToolCall::observed_at,
    );
    verify(
        "provider_tool_result",
        |s| {
            let hosted = ProviderToolResult::new(bid(1), bid(2), content()).unwrap();
            stamp!(hosted, s)
        },
        ProviderToolResult::observed_at,
    );
    verify(
        "provider_tool_execution_record",
        |s| stamp!(ProviderToolExecutionRecord::new(bid(1)), s),
        ProviderToolExecutionRecord::observed_at,
    );
}

#[test]
fn given_both_turn_kinds_when_stamped_then_they_serialize_load_back_and_refuse_bad_times() {
    verify(
        "input_turn",
        |s| {
            stamp!(
                InputTurn::new(tid(1), vec![common::turns::human_message(2).into()]).unwrap(),
                s
            )
        },
        InputTurn::observed_at,
    );
    verify(
        "ai_turn",
        |s| stamp!(AITurn::open(tid(1), aid(1), author("A")), s),
        AITurn::observed_at,
    );
}
