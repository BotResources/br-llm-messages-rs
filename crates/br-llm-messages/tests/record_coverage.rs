mod common;

use br_llm_messages::{
    AITurn, AgentMail, AgentRequest, BudgetKind, ClosureReason, Conversation, ExecutionStatus,
    ExternalId, FailureDetail, InputTurn, MailCorrelation, ModelResponse, ProviderOperation,
    ProviderToolCall, ProviderToolExecutionRecord, ProviderToolResult, RecordSet,
    RedactedThinkingBlock, RequestOrigin, ResponseEnding, ResponseMetadata, ResultExposure,
    RuntimeToolCall, StructuredBlock, TaskCompleted, TaskFailed, TaskStarted, TextBlock, Turn,
    TurnEntry,
};
use common::records::shuffled;
use common::turns::{close, finished, hosted_record, mount, tid, unmount};
use common::{aid, author, bid, cvid, eid, envelope, kid, plain_text, png, rid, signature, tool};
use serde_json::{Value, json};

fn input(turn: u64, entry: impl Into<br_llm_messages::IncomingEntry>) -> Turn {
    InputTurn::new(tid(turn), entry).unwrap().into()
}

fn ai(turn: u64, entries: Vec<TurnEntry>) -> Turn {
    AITurn::from_parts(tid(turn), aid(1), author("Codex"), entries)
        .unwrap()
        .into()
}

fn incoming_turns() -> Vec<Turn> {
    let request = AgentRequest::new(
        eid(101),
        aid(2),
        author("Claude"),
        cvid(2),
        RequestOrigin::request(rid(1)),
        vec![plain_text(102, "do it")],
    )
    .unwrap();
    let mail = AgentMail::new(
        eid(103),
        aid(2),
        author("Claude"),
        cvid(2),
        vec![plain_text(104, "fyi")],
    )
    .unwrap()
    .with_in_reply_to(MailCorrelation::message(eid(999)));
    let reply = AgentMail::new(
        eid(105),
        aid(2),
        author("Claude"),
        cvid(2),
        vec![plain_text(106, "re")],
    )
    .unwrap()
    .with_in_reply_to(MailCorrelation::request(rid(2)));
    vec![
        input(1, request),
        input(2, mail),
        input(3, reply),
        input(
            4,
            TaskCompleted::new(eid(107), kid(1))
                .with_summary("done")
                .unwrap(),
        ),
        input(5, TaskCompleted::new(eid(108), kid(1))),
        input(
            6,
            TaskFailed::new(
                eid(109),
                kid(1),
                FailureDetail::new("boom").unwrap().with_message("it broke"),
            ),
        ),
        input(
            7,
            TaskStarted::new(eid(110), rid(1), kid(1)).with_conversation(cvid(3)),
        ),
    ]
}

fn exotic_response() -> TurnEntry {
    let call = RuntimeToolCall::new(
        bid(210),
        tool("lookup"),
        common::oj(json!({"q": [1, null]})),
    )
    .unwrap()
    .with_external_id(ExternalId::new("toolu_9").unwrap())
    .with_replay(signature("call-sig"));
    let hosted = ProviderToolCall::new(bid(211))
        .with_external_id(ExternalId::new("srv_1").unwrap())
        .with_operation(ProviderOperation::new("web_search").unwrap())
        .with_arguments(common::oj(Value::Null))
        .unwrap()
        .with_replay(envelope(json!({"k": {"z": 1, "a": 2}})));
    let replay_only =
        ProviderToolResult::replay_only(bid(212), bid(211), signature("opaque")).unwrap();
    let record = |id: u64, exposure: ResultExposure| {
        ProviderToolExecutionRecord::new(bid(id), exposure)
            .unwrap()
            .with_external_id(ExternalId::new("rec").unwrap())
            .with_operation(ProviderOperation::new("code").unwrap())
            .with_status(ExecutionStatus::Incomplete)
    };
    let blocks = vec![
        TextBlock::new(bid(200), "said")
            .unwrap()
            .with_replay(signature("text-sig"))
            .into(),
        StructuredBlock::new(bid(201), common::oj(Value::Null))
            .unwrap()
            .into(),
        png(202).into(),
        RedactedThinkingBlock::new(bid(203), envelope(json!([1, 2]))).into(),
        call.into(),
        hosted.into(),
        replay_only.into(),
        record(220, ResultExposure::opaque(signature("o"))).into(),
        record(
            221,
            ResultExposure::mixed(vec![plain_text(222, "partly")], signature("m")).unwrap(),
        )
        .into(),
        record(223, ResultExposure::not_exposed()).into(),
        hosted_record(224, ExecutionStatus::Pending),
    ];
    ModelResponse::new(
        eid(230),
        blocks,
        ResponseEnding::provider_continuation(),
        ResponseMetadata::new(),
    )
    .unwrap()
    .into()
}

fn ai_turns() -> Vec<Turn> {
    let failure = || FailureDetail::new("code").unwrap();
    vec![
        ai(
            20,
            vec![
                mount(240, "lookup"),
                unmount(241, 240),
                exotic_response(),
                close(242, ClosureReason::cancelled(Some("stop".into())).unwrap()),
            ],
        ),
        ai(
            21,
            vec![
                finished(250),
                close(251, ClosureReason::cancelled(None).unwrap()),
            ],
        ),
        ai(22, vec![close(252, ClosureReason::interrupted(failure()))]),
        ai(
            23,
            vec![close(
                253,
                ClosureReason::budget_exhausted(BudgetKind::Tokens),
            )],
        ),
        ai(24, vec![close(254, ClosureReason::failed(failure()))]),
        ai(
            25,
            vec![finished(255), close(256, ClosureReason::completed())],
        ),
    ]
}

fn conversation() -> Conversation {
    let mut turns = incoming_turns();
    turns.extend(ai_turns());
    Conversation::from_parts(cvid(500), turns).unwrap()
}

#[test]
fn given_every_remaining_entry_and_block_kind_when_decomposed_then_reconstruction_is_equal() {
    let original = conversation();

    let rebuilt = RecordSet::from_conversation(&original)
        .unwrap()
        .into_conversation(original.id())
        .unwrap();

    assert_eq!(rebuilt, original);
}

#[test]
fn given_every_remaining_kind_when_shuffled_then_reconstruction_is_equal() {
    let original = conversation();
    let set = RecordSet::from_conversation(&original).unwrap();

    let rebuilt = shuffled(&set).into_conversation(original.id()).unwrap();

    assert_eq!(rebuilt, original);
}

#[test]
fn given_every_remaining_kind_when_passed_through_json_then_reconstruction_is_equal() {
    let original = conversation();
    let json = RecordSet::from_conversation(&original)
        .unwrap()
        .to_json()
        .unwrap();

    let rebuilt = RecordSet::from_json(&json)
        .unwrap()
        .into_conversation(original.id())
        .unwrap();

    assert_eq!(rebuilt, original);
}
