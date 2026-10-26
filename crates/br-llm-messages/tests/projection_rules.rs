mod common;

use br_llm_messages::{
    AITurn, Conversation, ErrorCode, Perspective, ProviderToolCall, ProviderToolResult,
    ResponseEnding, TextBlock, Turn, TurnEntry,
};
use common::turns::response;
use common::turns::tid;
use common::{aid, author, bid, cvid, plain_text, refused};

const SENTINEL: &str = "SENTINEL_PRIVATE_OUTPUT";

fn outputs(turn: &Turn) -> Vec<&br_llm_messages::AgentOutput> {
    let Turn::InputTurn(input) = turn else {
        panic!("expected an input turn, got {turn:?}");
    };
    input
        .entries()
        .iter()
        .map(|entry| match entry {
            br_llm_messages::IncomingEntry::AgentOutput(output) => output,
            other => panic!("expected an agent output, got {other:?}"),
        })
        .collect()
}

fn turn_of(id: u64, agent: u64, label: &str, entries: Vec<TurnEntry>) -> Turn {
    AITurn::from_parts(tid(id), aid(agent), author(label), entries)
        .unwrap()
        .into()
}

fn conversation(turns: Vec<Turn>) -> Conversation {
    Conversation::from_parts(cvid(900), turns).unwrap()
}

fn said(entry: u64, block: u64, text: &str) -> TurnEntry {
    response(
        entry,
        vec![TextBlock::new(bid(block), text).unwrap().into()],
        ResponseEnding::finished(),
    )
}

#[test]
fn given_the_same_turn_selected_twice_when_projected_then_reused_identity() {
    let turn = turn_of(1, 1, "Codex", vec![said(11, 12, "mine")]);

    let error = refused(Perspective::of(aid(1)).project_turns(cvid(900), &[turn.clone(), turn]));

    assert_eq!(error.code(), ErrorCode::ReusedIdentity);
    assert_eq!(error.location(), Some("turns[1]"));
}

#[test]
fn given_a_projected_input_turn_when_pushed_in_a_conversation_then_disallowed_kind() {
    let thread = Perspective::of(aid(1)).project(&conversation(vec![turn_of(
        1,
        2,
        "Claude",
        vec![said(11, 12, "x")],
    )]));
    let mut target = Conversation::new(cvid(901));

    let error = refused(target.push_turn(thread.turns()[0].clone()));

    assert_eq!(error.code(), ErrorCode::DisallowedKind);
    assert_eq!(error.location(), Some("entries[0]"));
    assert!(target.turns().is_empty());
}

#[test]
fn given_a_projected_entry_when_put_in_an_input_turn_or_an_ai_turn_then_disallowed_kind() {
    let thread = Perspective::of(aid(1)).project(&conversation(vec![turn_of(
        1,
        2,
        "Claude",
        vec![said(11, 12, "x")],
    )]));
    let Turn::InputTurn(input) = &thread.turns()[0] else {
        panic!("input turn expected");
    };
    let entry = input.entries()[0].clone();

    let rebuilt = refused(br_llm_messages::InputTurn::single(tid(5), entry.clone()));
    let pushed = refused(TurnEntry::try_from(entry));

    assert_eq!(rebuilt.code(), ErrorCode::DisallowedKind);
    assert_eq!(pushed.code(), ErrorCode::DisallowedKind);
}

#[test]
fn given_a_call_left_unresolved_by_an_abnormal_closure_when_projected_then_the_turn_is_kept_as_recorded()
 {
    use common::turns::{calling, close, delivery, open_turn, pushed};
    let mut turn = open_turn(1);
    pushed(
        &mut turn,
        vec![
            calling(11, &[(12, "lookup"), (13, "search")]),
            delivery(14, &[(15, 12, "lookup")]),
            close(16, br_llm_messages::ClosureReason::interrupted()),
        ],
    );
    let conversation = conversation(vec![Turn::AITurn(turn.clone())]);

    let thread = Perspective::of(aid(1)).project(&conversation);

    let kept = thread.turns()[0].as_ai().expect("own turn");
    assert_eq!(kept, &turn);
    assert_eq!(kept.state().outstanding_runtime_calls(), [bid(13)]);
    assert_eq!(kept.entries().len(), 3);
}

#[test]
fn given_nested_tool_output_when_projected_then_no_nested_content_leaks() {
    let hosted = ProviderToolResult::new(bid(22), bid(21), vec![plain_text(23, SENTINEL)]).unwrap();
    let blocks = vec![
        ProviderToolCall::new(bid(21)).into(),
        hosted.into(),
        TextBlock::new(bid(24), "public").unwrap().into(),
    ];
    let conversation = conversation(vec![turn_of(
        1,
        2,
        "Claude",
        vec![response(11, blocks, ResponseEnding::finished())],
    )]);

    let thread = Perspective::of(aid(1)).project(&conversation);

    assert!(!format!("{thread:?}").contains(SENTINEL));
    assert_eq!(outputs(&thread.turns()[0])[0].content().len(), 1);
}
