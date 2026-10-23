mod common;

use br_llm_messages::{
    AITurn, Conversation, OtherItem, Perspective, ProjectedTurn, ProviderToolCall,
    ProviderToolResult, ResponseEnding, RuntimeToolCall, StructuredBlock, TextBlock, ThinkingBlock,
    Turn, TurnEntry,
};
use common::conversations::rich;
use common::turns::{human, incoming_batch, response, tid};
use common::{aid, author, bid, cid, cvid, eid, plain_text, png, signature, tool};
use serde_json::json;

const SENTINEL: &str = "SENTINEL_PRIVATE_OUTPUT";

fn turn_of(id: u64, agent: u64, label: &str, entries: Vec<TurnEntry>) -> Turn {
    AITurn::from_parts(tid(id), aid(agent), author(label), entries)
        .unwrap()
        .into()
}

fn conversation(turns: Vec<Turn>) -> Conversation {
    Conversation::from_parts(cvid(900), turns).unwrap()
}

fn public_text(entry: u64, block: u64, text: &str) -> TurnEntry {
    response(
        entry,
        vec![TextBlock::new(bid(block), text).unwrap().into()],
        ResponseEnding::finished(),
    )
}

#[test]
fn given_same_label_for_two_agents_when_projected_then_only_the_target_agent_turn_is_own() {
    let conversation = conversation(vec![
        turn_of(1, 1, "Codex", vec![public_text(11, 12, "mine")]),
        turn_of(2, 2, "Codex", vec![public_text(21, 22, "theirs")]),
    ]);

    let projection = Perspective::of(aid(1)).project(&conversation);

    assert!(matches!(projection.turns()[0], ProjectedTurn::Own(_)));
    assert!(matches!(projection.turns()[1], ProjectedTurn::Other(_)));
}

#[test]
fn given_one_agent_with_two_labels_when_projected_then_both_turns_are_own() {
    let conversation = conversation(vec![
        turn_of(1, 1, "Codex GPT-6 high", vec![public_text(11, 12, "a")]),
        turn_of(2, 1, "Codex GPT-6 low", vec![public_text(21, 22, "b")]),
    ]);

    let projection = Perspective::of(aid(1)).project(&conversation);

    assert!(
        projection
            .turns()
            .iter()
            .all(|turn| matches!(turn, ProjectedTurn::Own(_)))
    );
}

#[test]
fn given_own_turn_with_private_content_when_projected_then_it_is_kept_whole_in_order() {
    let working = rich();
    let original: Vec<Turn> = working.turns().to_vec();

    let projection = Perspective::of(aid(1)).project(&working);

    let own: Vec<&AITurn> = projection
        .turns()
        .iter()
        .filter_map(|turn| match turn {
            ProjectedTurn::Own(turn) => Some(turn),
            _ => None,
        })
        .collect();
    assert_eq!(own.len(), 1);
    assert_eq!(&Turn::from(own[0].clone()), &original[1]);
    let ids: Vec<_> = own[0].entries().iter().map(TurnEntry::id).collect();
    let expected: Vec<_> = original[1]
        .as_ai()
        .unwrap()
        .entries()
        .iter()
        .map(TurnEntry::id)
        .collect();
    assert_eq!(ids, expected);
}

#[test]
fn given_other_agent_response_when_projected_then_only_public_content_remains() {
    let cited = common::conversations::rich();
    let projection = Perspective::of(aid(2)).project(&cited);

    let other = projection
        .turns()
        .iter()
        .find_map(|turn| match turn {
            ProjectedTurn::Other(turn) if turn.id() == tid(10) => Some(turn),
            _ => None,
        })
        .expect("the working turn belongs to the other agent");
    let responses: Vec<_> = other
        .items()
        .iter()
        .filter_map(|item| match item {
            OtherItem::Public(public) => Some(public),
            _ => None,
        })
        .collect();

    assert_eq!(responses[0].entry(), eid(12));
    assert_eq!(responses[0].blocks().len(), 1);
    assert_eq!(responses[0].blocks()[0].id(), bid(14));
    assert_eq!(responses[1].entry(), eid(28));
}

#[test]
fn given_text_with_replay_and_citation_when_projected_then_replay_is_stripped_and_citation_kept() {
    let cite = br_llm_messages::Citation::new(
        cid(70),
        br_llm_messages::CitationSource::new("doc").unwrap(),
    )
    .with_range(br_llm_messages::TextRange::new(0, 3).unwrap());
    let text = TextBlock::new(bid(71), "see it")
        .unwrap()
        .with_citations(vec![cite.clone()])
        .unwrap()
        .with_item_ref(br_llm_messages::ItemRef::new("msg_1").unwrap())
        .with_replay(signature("sig"));
    let conversation = conversation(vec![turn_of(
        1,
        2,
        "Claude",
        vec![response(
            11,
            vec![text.into(), png(72).into()],
            ResponseEnding::finished(),
        )],
    )]);

    let projection = Perspective::of(aid(1)).project(&conversation);

    let ProjectedTurn::Other(other) = &projection.turns()[0] else {
        panic!("expected an other-agent turn");
    };
    let OtherItem::Public(public) = &other.items()[0] else {
        panic!("expected a public response");
    };
    let br_llm_messages::PlainBlock::Text(kept) = &public.blocks()[0] else {
        panic!("expected text");
    };
    assert!(kept.replay().is_none());
    assert!(kept.item_ref().is_none());
    assert_eq!(kept.citations(), &[cite]);
    assert_eq!(public.blocks().len(), 2);
}

#[test]
fn given_private_blocks_only_when_projected_then_the_response_is_omitted() {
    let private = response(
        11,
        vec![
            ThinkingBlock::new(bid(12), "plan")
                .unwrap()
                .with_replay(signature("sig"))
                .into(),
            RuntimeToolCall::new(bid(13), tool("lookup"), common::oj(json!({})))
                .unwrap()
                .into(),
            ProviderToolCall::new(bid(14)).into(),
        ],
        ResponseEnding::tool_use(),
    );
    let conversation = conversation(vec![turn_of(1, 2, "Claude", vec![private])]);

    let projection = Perspective::of(aid(1)).project(&conversation);

    let ProjectedTurn::Other(other) = &projection.turns()[0] else {
        panic!("expected an other-agent turn");
    };
    assert!(other.items().is_empty());
}

#[test]
fn given_nested_tool_output_when_projected_then_no_nested_content_leaks() {
    let hosted = ProviderToolResult::new(bid(22), bid(21), vec![plain_text(23, SENTINEL)]).unwrap();
    let blocks = vec![
        ProviderToolCall::new(bid(21)).into(),
        hosted.into(),
        StructuredBlock::new(bid(24), common::oj(json!({"public": true})))
            .unwrap()
            .into(),
    ];
    let conversation = conversation(vec![turn_of(
        1,
        2,
        "Claude",
        vec![response(11, blocks, ResponseEnding::finished())],
    )]);

    let projection = Perspective::of(aid(1)).project(&conversation);

    assert!(!format!("{projection:?}").contains(SENTINEL));
    let ProjectedTurn::Other(other) = &projection.turns()[0] else {
        panic!("expected an other-agent turn");
    };
    let OtherItem::Public(public) = &other.items()[0] else {
        panic!("expected a public response");
    };
    assert_eq!(public.blocks().len(), 1);
}

#[test]
fn given_incoming_entries_of_every_concrete_kind_when_projected_then_kind_and_attribution_survive()
{
    let mut entries = incoming_batch(100);
    entries.push(public_text(200, 201, "hi"));
    let conversation = conversation(vec![turn_of(1, 2, "Claude", entries.clone())]);

    let projection = Perspective::of(aid(1)).project(&conversation);

    let ProjectedTurn::Other(other) = &projection.turns()[0] else {
        panic!("expected an other-agent turn");
    };
    let incoming: Vec<_> = other
        .items()
        .iter()
        .filter_map(|item| match item {
            OtherItem::Incoming(entry) => Some(entry.clone()),
            _ => None,
        })
        .collect();
    let expected: Vec<_> = entries
        .into_iter()
        .filter_map(|entry| br_llm_messages::IncomingEntry::try_from(entry).ok())
        .collect();
    assert_eq!(incoming, expected);
    assert_eq!(incoming.len(), 8);
}

#[test]
fn given_input_turns_when_projected_then_they_pass_through_unchanged() {
    let original = rich();

    let projection = Perspective::of(aid(1)).project(&original);

    let inputs: Vec<_> = projection
        .turns()
        .iter()
        .filter_map(|turn| match turn {
            ProjectedTurn::Input(turn) => Some(Turn::from(turn.clone())),
            _ => None,
        })
        .collect();
    let expected: Vec<_> = original
        .turns()
        .iter()
        .filter(|turn| turn.as_input().is_some())
        .cloned()
        .collect();
    assert_eq!(inputs, expected);
}

#[test]
fn given_a_conversation_when_projected_twice_then_it_is_unchanged_ordered_and_equal() {
    let original = rich();
    let before = original.clone();
    let perspective = Perspective::of(aid(2));

    let first = perspective.project(&original);
    let second = perspective.project_turns(original.turns());

    assert_eq!(original, before);
    assert_eq!(first, second);
    let order: Vec<_> = first.turns().iter().map(ProjectedTurn::id).collect();
    let expected: Vec<_> = original.turns().iter().map(Turn::id).collect();
    assert_eq!(order, expected);
}

#[test]
fn given_a_human_input_and_no_response_when_projected_then_no_response_is_synthesized() {
    let conversation = conversation(vec![
        common::turns::input_turn(1, 2).into(),
        turn_of(3, 1, "Codex", vec![human(31)]),
    ]);

    let projection = Perspective::of(aid(1)).project(&conversation);

    assert_eq!(projection.len(), 2);
    let ProjectedTurn::Own(own) = &projection.turns()[1] else {
        panic!("expected an own turn");
    };
    assert_eq!(own.entries().len(), 1);
}
