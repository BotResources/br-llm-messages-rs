mod common;

use br_llm_messages::{
    AgentOutput, Citation, CitationSource, Conversation, ErrorCode, IncomingEntry, InputTurn,
    PartialAgentOutput, PlainBlock, ProjectedThread, TextBlock, TextRange, TurnEntry,
};
use common::turns::tid;
use common::{aid, author, bid, cid, cvid, eid, plain_text, refused};

fn cited(block: u64, citation: u64) -> PlainBlock {
    let citation = Citation::new(cid(citation), CitationSource::new("doc").unwrap())
        .with_range(TextRange::new(0, 1).unwrap());
    PlainBlock::try_from(
        TextBlock::new(bid(block), "x")
            .unwrap()
            .with_citations(vec![citation])
            .unwrap(),
    )
    .unwrap()
}

#[test]
fn given_an_agent_an_author_and_blocks_when_building_outputs_then_they_keep_what_was_given() {
    let output =
        AgentOutput::new(eid(1), aid(2), author("Codex"), vec![plain_text(3, "done")]).unwrap();
    let partial =
        PartialAgentOutput::new(eid(4), aid(2), author("Codex"), vec![plain_text(5, "so")])
            .unwrap();

    assert_eq!(output.id(), eid(1));
    assert_eq!(output.agent(), aid(2));
    assert_eq!(output.author(), &author("Codex"));
    assert_eq!(output.content().len(), 1);
    assert_eq!(partial.id(), eid(4));
    assert_eq!(partial.content()[0].id(), bid(5));
}

#[test]
fn given_no_block_when_building_an_output_then_invalid_content() {
    let output = AgentOutput::new(eid(1), aid(2), author("A"), vec![]);
    let partial = PartialAgentOutput::new(eid(1), aid(2), author("A"), vec![]);

    assert_eq!(refused(output).code(), ErrorCode::InvalidContent);
    assert_eq!(refused(partial).code(), ErrorCode::InvalidContent);
}

#[test]
fn given_blocks_that_a_reload_refuses_when_building_an_output_then_the_same_refusal_is_given() {
    let reused = AgentOutput::new(
        eid(1),
        aid(2),
        author("A"),
        vec![plain_text(3, "a"), plain_text(3, "b")],
    );
    let entry_id_as_block = AgentOutput::new(eid(1), aid(2), author("A"), vec![plain_text(1, "a")]);
    let decorated = PartialAgentOutput::new(eid(1), aid(2), author("A"), vec![cited(3, 4)]);

    assert_eq!(refused(reused).code(), ErrorCode::ReusedIdentity);
    assert_eq!(refused(entry_id_as_block).code(), ErrorCode::ReusedIdentity);
    assert_eq!(refused(decorated).code(), ErrorCode::DisallowedKind);
}

#[test]
fn given_a_built_output_when_serialized_in_a_projected_thread_then_it_reloads_equal() {
    let output = AgentOutput::new(eid(1), aid(2), author("A"), vec![plain_text(3, "a")]).unwrap();
    let partial =
        PartialAgentOutput::new(eid(4), aid(2), author("A"), vec![plain_text(5, "b")]).unwrap();
    let turn = InputTurn::new_projected(tid(6), vec![output.into(), partial.into()]).unwrap();
    let thread = ProjectedThread::from_turns(cvid(9), aid(1), vec![turn.into()]).unwrap();

    let reloaded = ProjectedThread::from_json(&thread.to_json().unwrap()).unwrap();

    assert_eq!(reloaded, thread);
}

#[test]
fn given_a_projected_input_turn_when_built_then_it_follows_the_input_turn_rules() {
    let output = AgentOutput::new(eid(1), aid(2), author("A"), vec![plain_text(3, "a")]).unwrap();

    let empty = InputTurn::new_projected(tid(6), vec![]);
    let reused = InputTurn::new_projected(
        tid(6),
        vec![
            output.clone().into(),
            AgentOutput::new(eid(7), aid(2), author("A"), vec![plain_text(3, "b")])
                .unwrap()
                .into(),
        ],
    );

    assert_eq!(refused(empty).code(), ErrorCode::InvalidRecordStructure);
    assert_eq!(refused(reused).code(), ErrorCode::ReusedIdentity);
}

#[test]
fn given_outputs_when_given_to_stored_histories_then_they_are_refused() {
    let output = AgentOutput::new(eid(1), aid(2), author("A"), vec![plain_text(3, "a")]).unwrap();
    let partial =
        PartialAgentOutput::new(eid(4), aid(2), author("A"), vec![plain_text(5, "b")]).unwrap();
    let projected_turn = InputTurn::new_projected(tid(6), vec![output.clone().into()]).unwrap();

    let stored_turn = InputTurn::new(tid(7), vec![output.clone().into()]);
    let mut conversation = Conversation::new(cvid(9));
    let pushed = conversation.push_turn(projected_turn.into());
    let as_ai_entry = TurnEntry::try_from(IncomingEntry::from(partial));

    assert_eq!(refused(stored_turn).code(), ErrorCode::DisallowedKind);
    assert_eq!(refused(pushed).code(), ErrorCode::DisallowedKind);
    assert_eq!(refused(as_ai_entry).code(), ErrorCode::DisallowedKind);
    assert!(conversation.turns().is_empty());
}
