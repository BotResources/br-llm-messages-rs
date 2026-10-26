mod common;

use br_llm_messages::{
    AITurn, AgentMail, AgentRequest, ClosureReason, Conversation, ExecutionStatus, ExternalId,
    InputTurn, ItemRef, ModelResponse, ProviderOperation, ProviderToolCall,
    ProviderToolExecutionRecord, ProviderToolResult, RecordSet, RedactedThinkingBlock,
    RequestOrigin, ResponseEnding, ResponseMetadata, RuntimeToolCall, StructuredBlock,
    TaskCompleted, TaskFailed, TaskStarted, TextBlock, Turn, TurnEntry,
};
use common::records::interleaved;
use common::turns::{close, finished, hosted_record, tid};
use common::{aid, author, bid, cvid, eid, envelope, kid, plain_text, png, rid, signature, tool};
use serde_json::{Value, json};

fn input(turn: u64, entry: impl Into<br_llm_messages::IncomingEntry>) -> Turn {
    InputTurn::single(tid(turn), entry).unwrap().into()
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
    .with_in_reply_to(eid(999));
    let reply = AgentMail::new(
        eid(105),
        aid(2),
        author("Claude"),
        cvid(2),
        vec![plain_text(106, "re")],
    )
    .unwrap()
    .with_in_reply_to(eid(998));
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
        input(6, TaskFailed::new(eid(109), kid(1), "it broke").unwrap()),
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
    .with_item_ref(ItemRef::new("fc_9").unwrap())
    .with_replay(signature("call-sig"));
    let hosted = ProviderToolCall::new(bid(211))
        .with_external_id(ExternalId::new("srv_1").unwrap())
        .with_item_ref(ItemRef::new("ws_9").unwrap())
        .with_operation(ProviderOperation::new("web_search").unwrap())
        .with_arguments(common::oj(json!({"query": "weather"})))
        .unwrap()
        .with_replay(envelope(json!({"k": {"z": 1, "a": 2}})));
    let replay_only =
        ProviderToolResult::replay_only(bid(212), bid(211), signature("opaque")).unwrap();
    let record = |id: u64| {
        ProviderToolExecutionRecord::new(bid(id))
            .with_external_id(ExternalId::new("rec").unwrap())
            .with_operation(ProviderOperation::new("code").unwrap())
            .with_status(ExecutionStatus::Incomplete)
    };
    let blocks = vec![
        TextBlock::new(bid(200), "said")
            .unwrap()
            .with_item_ref(ItemRef::new("msg_9").unwrap())
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
        record(220).with_replay(signature("o")).into(),
        record(221)
            .with_content(vec![plain_text(222, "partly")])
            .unwrap()
            .with_replay(signature("m"))
            .into(),
        record(223).into(),
        record(225)
            .with_content(vec![plain_text(226, "visible")])
            .unwrap()
            .into(),
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
    vec![
        ai(
            20,
            vec![
                exotic_response(),
                close(242, ClosureReason::cancelled().with_detail("stop").unwrap()),
            ],
        ),
        ai(
            21,
            vec![finished(250), close(251, ClosureReason::cancelled())],
        ),
        ai(
            22,
            vec![close(
                252,
                ClosureReason::interrupted().with_detail("code").unwrap(),
            )],
        ),
        ai(
            23,
            vec![close(
                253,
                ClosureReason::budget_exhausted()
                    .with_detail("tokens")
                    .unwrap(),
            )],
        ),
        ai(24, vec![close(254, ClosureReason::failed("code").unwrap())]),
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
fn given_rows_of_several_parents_interleaved_in_the_slice_when_rebuilt_then_each_parent_keeps_its_row_order()
 {
    let original = conversation();
    let set = RecordSet::from_conversation(&original).unwrap();

    let rebuilt = interleaved(&set).into_conversation(original.id()).unwrap();

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
