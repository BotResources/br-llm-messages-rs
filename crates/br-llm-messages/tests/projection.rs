mod common;
use common::Raw as _;

use br_llm_messages::{
    AITurn, Citation, CitationSource, Conversation, IncomingEntry, ItemRef, Perspective,
    PlainBlock, ProviderToolCall, ResponseEnding, RuntimeToolCall, StructuredBlock, TextBlock,
    TextRange, ThinkingBlock, Turn, TurnEntry,
};
use common::conversations::rich;
use common::turns::{human, incoming_batch, response, tid};
use common::{aid, author, bid, cid, cvid, eid, plain_text, png, signature, tool};
use serde_json::json;

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

fn outputs(turn: &Turn) -> Vec<&br_llm_messages::AgentOutput> {
    let Turn::InputTurn(input) = turn else {
        panic!("expected an input turn, got {turn:?}");
    };
    input
        .entries()
        .iter()
        .filter_map(|entry| match entry {
            IncomingEntry::AgentOutput(output) => Some(output),
            _ => None,
        })
        .collect()
}

#[test]
fn given_same_label_for_two_agents_when_projected_then_only_the_target_agent_keeps_its_ai_turn() {
    let conversation = conversation(vec![
        turn_of(1, 1, "Codex", vec![said(11, 12, "mine")]),
        turn_of(2, 2, "Codex", vec![said(21, 22, "theirs")]),
    ]);

    let thread = Perspective::of(aid(1)).project(&conversation);

    assert_eq!(thread.agent(), aid(1));
    assert_eq!(thread.conversation(), cvid(900));
    assert!(matches!(thread.turns()[0], Turn::AITurn(_)));
    assert!(matches!(thread.turns()[1], Turn::InputTurn(_)));
}

#[test]
fn given_one_agent_with_two_labels_when_projected_then_both_turns_stay_as_recorded() {
    let conversation = conversation(vec![
        turn_of(1, 1, "Codex GPT-6 high", vec![said(11, 12, "a")]),
        turn_of(2, 1, "Codex GPT-6 low", vec![said(21, 22, "b")]),
    ]);

    let thread = Perspective::of(aid(1)).project(&conversation);

    assert_eq!(thread.turns(), conversation.turns());
}

#[test]
fn given_own_turn_with_private_content_when_projected_then_it_is_kept_whole_in_order() {
    let working = rich();

    let thread = Perspective::of(aid(1)).project(&working);

    let own: Vec<&Turn> = thread
        .turns()
        .iter()
        .filter(|t| t.as_ai().is_some())
        .collect();
    assert_eq!(own.len(), 1);
    assert_eq!(own[0], &working.turns()[1]);
}

#[test]
fn given_input_turns_when_projected_then_they_pass_through_unchanged() {
    let original = rich();

    let thread = Perspective::of(aid(1)).project(&original);

    let expected: Vec<&Turn> = original
        .turns()
        .iter()
        .filter(|t| t.as_input().is_some())
        .collect();
    let inputs: Vec<&Turn> = thread
        .turns()
        .iter()
        .filter(|turn| expected.iter().any(|input| input.id() == turn.id()))
        .collect();
    assert_eq!(inputs, expected);
}

#[test]
fn given_other_agent_response_when_projected_then_one_agent_output_with_the_same_turn_id() {
    let thread = Perspective::of(aid(2)).project(&rich());

    let translated = thread
        .turns()
        .iter()
        .find(|turn| turn.id() == tid(10))
        .expect("the working turn belongs to the other agent");
    let found = outputs(translated);

    assert_eq!(found.len(), 2);
    assert_eq!(found[0].id(), eid(12));
    assert_eq!(found[0].agent(), aid(1));
    assert_eq!(found[0].content().len(), 1);
    assert_eq!(found[0].content()[0].id(), bid(14));
    assert_eq!(found[1].id(), eid(28));
}

#[test]
fn given_two_responses_of_another_agent_when_projected_then_one_entry_each_with_agent_and_author() {
    let conversation = conversation(vec![turn_of(
        1,
        2,
        "Claude Sonnet",
        vec![said(11, 12, "first"), human(20), said(13, 14, "second")],
    )]);

    let thread = Perspective::of(aid(1)).project(&conversation);

    let found = outputs(&thread.turns()[0]);
    assert_eq!(found.len(), 2);
    assert_eq!(found[0].id(), eid(11));
    assert_eq!(found[1].id(), eid(13));
    for output in found {
        assert_eq!(output.agent(), aid(2));
        assert_eq!(output.author(), &author("Claude Sonnet"));
    }
}

#[test]
fn given_a_response_with_mixed_blocks_when_projected_then_only_bare_text_image_and_structured_blocks_remain_in_order()
 {
    let cite = Citation::new(cid(70), CitationSource::new("doc").unwrap())
        .with_range(TextRange::new(0, 3).unwrap());
    let cited = TextBlock::new(bid(13), "cited")
        .unwrap()
        .with_citations(vec![cite])
        .unwrap()
        .with_item_ref(ItemRef::new("msg_1").unwrap())
        .with_replay(signature("sig"))
        .with_observed_at(common::at(3));
    let blocks = vec![
        TextBlock::new(bid(11), "one").unwrap().into(),
        ThinkingBlock::new(bid(12), "secret plan").unwrap().into(),
        cited.into(),
        png(14).with_observed_at(common::at(4)).into(),
        StructuredBlock::new(bid(15), common::oj(json!({"hidden": true})))
            .unwrap()
            .with_observed_at(common::at(5))
            .into(),
        RuntimeToolCall::new(bid(16), tool("lookup"), common::oj(json!({})))
            .unwrap()
            .into(),
    ];
    let conversation = conversation(vec![turn_of(
        1,
        2,
        "Claude",
        vec![response(10, blocks, ResponseEnding::tool_use())],
    )]);

    let thread = Perspective::of(aid(1)).project(&conversation);

    let content = outputs(&thread.turns()[0])[0].content();
    let expected: Vec<PlainBlock> = vec![
        plain_text(11, "one"),
        plain_text(13, "cited"),
        png(14).into(),
        StructuredBlock::new(bid(15), common::oj(json!({"hidden": true})))
            .unwrap()
            .into(),
    ];
    assert_eq!(content, expected.as_slice());
    let rendered = format!("{thread:?}");
    assert!(!rendered.contains("secret plan") && !rendered.contains("lookup"));
}

#[test]
fn given_responses_without_text_when_projected_then_no_entry_and_no_input_turn_is_made() {
    let private = response(
        11,
        vec![
            ThinkingBlock::new(bid(12), "plan").unwrap().into(),
            RuntimeToolCall::new(bid(13), tool("lookup"), common::oj(json!({})))
                .unwrap()
                .into(),
            ProviderToolCall::new(bid(14)).into(),
        ],
        ResponseEnding::tool_use(),
    );
    let conversation = conversation(vec![
        turn_of(1, 2, "Claude", vec![private]),
        turn_of(2, 2, "Claude", vec![said(21, 22, "kept")]),
    ]);

    let thread = Perspective::of(aid(1)).project(&conversation);

    assert_eq!(thread.turns().len(), 1);
    assert_eq!(thread.turns()[0].id(), tid(2));
}

#[test]
fn given_incoming_entries_of_every_kind_in_an_own_turn_when_projected_then_they_survive() {
    let mut entries = incoming_batch(100);
    entries.push(said(200, 201, "hi"));
    let conversation = conversation(vec![turn_of(1, 2, "Claude", entries.clone())]);

    let thread = Perspective::of(aid(2)).project(&conversation);

    let own = thread.turns()[0].as_ai().expect("own turn");
    assert_eq!(own.entries(), entries.as_slice());
}

#[test]
fn given_a_projection_when_inspected_then_every_identity_was_already_recorded() {
    let original = rich();
    let thread = Perspective::of(aid(2)).project(&original);
    let recorded = original.to_value().unwrap().raw().to_string();

    for turn in thread.turns() {
        assert!(recorded.contains(&turn.id().to_string()));
        if let Turn::InputTurn(input) = turn {
            for entry in input.entries() {
                assert!(recorded.contains(&entry.id().to_string()));
            }
        }
    }
}

#[test]
fn given_overlapping_turns_of_two_agents_when_projected_then_order_is_selection_order() {
    let conversation = conversation(vec![
        turn_of(1, 2, "A", vec![said(21, 22, "earlier in the list")]),
        turn_of(2, 3, "B", vec![said(11, 12, "later")]),
        turn_of(3, 2, "A", vec![said(31, 32, "again")]),
    ]);

    let thread = Perspective::of(aid(1)).project(&conversation);

    let order: Vec<_> = thread.turns().iter().map(Turn::id).collect();
    assert_eq!(order, [tid(1), tid(2), tid(3)]);
    assert_eq!(outputs(&thread.turns()[1])[0].agent(), aid(3));
}

#[test]
fn given_a_conversation_when_projected_twice_then_it_is_unchanged_and_both_results_are_equal() {
    let original = rich();
    let before = original.clone();
    let perspective = Perspective::of(aid(2));

    let first = perspective.project(&original);
    let second = perspective
        .project_turns(original.id(), original.turns())
        .unwrap();

    assert_eq!(original, before);
    assert_eq!(first, second);
}
