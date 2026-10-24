mod common;

use br_llm_messages::{
    AITurn, ClosureReason, Conversation, Perspective, ProjectedTurn, ResponseEnding,
    RuntimeToolCall, StructuredBlock, TextBlock, ThinkingBlock, Turn, TurnEntry,
};
use common::turns::{calling, close, delivery, human, open_turn, pushed, response, tid};
use common::{aid, author, bid, cid, cvid, eid, png, signature, tool};
use serde_json::json;

fn conversation(turns: Vec<Turn>) -> Conversation {
    Conversation::from_parts(cvid(900), turns).unwrap()
}

fn turn_of(id: u64, agent: u64, label: &str, entries: Vec<TurnEntry>) -> Turn {
    AITurn::from_parts(tid(id), aid(agent), author(label), entries)
        .unwrap()
        .into()
}

fn said(entry: u64, block: u64, text: &str) -> TurnEntry {
    response(
        entry,
        vec![TextBlock::new(bid(block), text).unwrap().into()],
        ResponseEnding::finished(),
    )
}

fn others(projection: &br_llm_messages::Projection, index: usize) -> &br_llm_messages::OtherTurn {
    match &projection.turns()[index] {
        ProjectedTurn::Other(other) => other,
        other => panic!("expected another agent's turn, got {other:?}"),
    }
}

#[test]
fn given_two_responses_of_another_agent_when_projected_then_one_item_each_with_agent_and_author() {
    let conversation = conversation(vec![turn_of(
        1,
        2,
        "Claude Sonnet",
        vec![said(11, 12, "first"), human(20), said(13, 14, "second")],
    )]);

    let projection = Perspective::of(aid(1)).project(&conversation);

    let items = others(&projection, 0).items();
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].entry(), eid(11));
    assert_eq!(items[1].entry(), eid(13));
    for item in items {
        assert_eq!(item.agent(), aid(2));
        assert_eq!(item.author(), &author("Claude Sonnet"));
    }
}

#[test]
fn given_a_response_with_mixed_blocks_when_projected_then_only_its_text_blocks_remain_in_order() {
    let cited = TextBlock::new(bid(13), "cited")
        .unwrap()
        .with_citations(vec![br_llm_messages::Citation::new(
            cid(70),
            br_llm_messages::CitationSource::new("doc").unwrap(),
        )])
        .unwrap();
    let blocks = vec![
        TextBlock::new(bid(11), "one").unwrap().into(),
        ThinkingBlock::new(bid(12), "secret plan").unwrap().into(),
        cited.into(),
        png(14).into(),
        StructuredBlock::new(bid(15), common::oj(json!({"hidden": true})))
            .unwrap()
            .into(),
        RuntimeToolCall::new(bid(16), tool("lookup"), common::oj(json!({})))
            .unwrap()
            .into(),
        TextBlock::new(bid(17), "two")
            .unwrap()
            .with_replay(signature("sig"))
            .into(),
    ];
    let conversation = conversation(vec![turn_of(
        1,
        2,
        "Claude",
        vec![response(10, blocks, ResponseEnding::tool_use())],
    )]);

    let projection = Perspective::of(aid(1)).project(&conversation);

    let texts = others(&projection, 0).items()[0].texts();
    let ids: Vec<_> = texts.iter().map(TextBlock::id).collect();
    assert_eq!(ids, [bid(11), bid(13), bid(17)]);
    assert!(texts.iter().all(|text| text.citations().is_empty()));
    assert!(texts.iter().all(|text| text.replay().is_none()));
    let rendered = format!("{projection:?}");
    assert!(!rendered.contains("hidden") && !rendered.contains("lookup"));
}

#[test]
fn given_incoming_entries_in_another_agents_turn_when_projected_then_none_is_contributed() {
    let conversation = conversation(vec![turn_of(
        1,
        2,
        "Claude",
        vec![human(30), said(11, 12, "answer")],
    )]);

    let projection = Perspective::of(aid(1)).project(&conversation);

    let items = others(&projection, 0).items();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].entry(), eid(11));
}

#[test]
fn given_overlapping_turns_of_two_agents_when_projected_then_order_is_selection_order_not_time() {
    let late = said(11, 12, "later in the list but earlier in time");
    let early = said(21, 22, "earlier in the list");
    let conversation = conversation(vec![
        turn_of(1, 2, "A", vec![early]),
        turn_of(2, 3, "B", vec![late]),
        turn_of(3, 2, "A", vec![said(31, 32, "again")]),
    ]);

    let projection = Perspective::of(aid(1)).project(&conversation);

    let order: Vec<_> = projection.turns().iter().map(ProjectedTurn::id).collect();
    assert_eq!(order, [tid(1), tid(2), tid(3)]);
    assert_eq!(others(&projection, 1).items()[0].agent(), aid(3));
}

#[test]
fn given_a_call_left_unresolved_by_an_abnormal_closure_when_projected_then_it_is_marked_and_no_result_is_created()
 {
    let mut turn = open_turn(1);
    pushed(
        &mut turn,
        vec![
            calling(11, &[(12, "lookup"), (13, "search")]),
            delivery(14, &[(15, 12, "lookup")]),
            close(16, ClosureReason::interrupted()),
        ],
    );
    let original = turn.clone();
    let conversation = conversation(vec![Turn::AITurn(turn)]);

    let projection = Perspective::of(aid(1)).project(&conversation);

    let ProjectedTurn::Own(own) = &projection.turns()[0] else {
        panic!("expected an own turn");
    };
    assert_eq!(own.unresolved_calls(), [bid(13)]);
    assert_eq!(own.turn(), &original);
}

#[test]
fn given_calls_that_are_answered_or_still_running_when_projected_then_none_is_marked_unresolved() {
    let mut running = open_turn(1);
    pushed(&mut running, vec![calling(11, &[(12, "lookup")])]);
    let mut answered = open_turn(2);
    pushed(
        &mut answered,
        vec![
            calling(21, &[(22, "lookup")]),
            delivery(23, &[(24, 22, "lookup")]),
            close(25, ClosureReason::completed()),
        ],
    );
    let conversation = conversation(vec![Turn::AITurn(running), Turn::AITurn(answered)]);

    let projection = Perspective::of(aid(1)).project(&conversation);

    for projected in projection.turns() {
        let ProjectedTurn::Own(own) = projected else {
            panic!("expected own turns");
        };
        assert!(own.unresolved_calls().is_empty());
    }
}
