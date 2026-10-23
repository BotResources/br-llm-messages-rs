mod common;

use br_llm_messages::{
    AITurn, AgentId, BlockId, ContentBlock, Conversation, EntryId, ErrorCode, HumanId,
    HumanMessage, PlainBlock, ResponseEnding, TurnId, Uuid,
};
use common::turns::{calling, delivery, human_message, mount, open_turn, response, tid};
use common::{author, bid, cid, cvid, eid, plain_text, refused, v7};

fn empty() -> Conversation {
    Conversation::new(cvid(1))
}

#[test]
fn given_ids_of_every_object_kind_when_reused_for_another_kind_then_reused_identity_names_it() {
    let mut conversation = empty();
    conversation.push_turn(open_turn(10).into()).unwrap();
    conversation
        .push_entry(
            &tid(10),
            response(
                11,
                vec![cited(12, 13), calling_block(14)],
                ResponseEnding::tool_results_requested(),
            ),
        )
        .unwrap();
    let taken: [Uuid; 5] = [
        cvid(1).as_uuid(),
        tid(10).as_uuid(),
        eid(11).as_uuid(),
        bid(12).as_uuid(),
        cid(13).as_uuid(),
    ];

    for (attempt, id) in taken.into_iter().enumerate() {
        let probe = HumanMessage::new(
            EntryId::from_uuid(id).unwrap(),
            HumanId::from_uuid(v7(1)).unwrap(),
            vec![plain_text(700 + attempt as u64, "x")],
        )
        .unwrap();
        let reuse_as_entry = refused(conversation.push_entry(&tid(10), probe.into()));
        let reuse_as_turn = refused(
            conversation.push_turn(
                br_llm_messages::InputTurn::new(
                    TurnId::from_uuid(id).unwrap(),
                    human_message(800 + attempt as u64),
                )
                .unwrap()
                .into(),
            ),
        );
        let reuse_as_block = refused(
            conversation.push_turn(
                br_llm_messages::InputTurn::new(
                    tid(900 + attempt as u64),
                    HumanMessage::new(
                        eid(950 + attempt as u64),
                        HumanId::from_uuid(v7(1)).unwrap(),
                        vec![
                            PlainBlock::try_from(
                                br_llm_messages::TextBlock::new(
                                    BlockId::from_uuid(id).unwrap(),
                                    "x",
                                )
                                .unwrap(),
                            )
                            .unwrap(),
                        ],
                    )
                    .unwrap(),
                )
                .unwrap()
                .into(),
            ),
        );

        for error in [reuse_as_entry, reuse_as_turn, reuse_as_block] {
            assert_eq!(error.code(), ErrorCode::ReusedIdentity);
            assert_eq!(error.object_id(), Some(id));
        }
    }
    assert_eq!(conversation.turns().len(), 1);
}

fn cited(block: u64, citation: u64) -> ContentBlock {
    use br_llm_messages::{Citation, CitationSource, TextBlock, TextRange};
    let cite = Citation::new(cid(citation), CitationSource::new("d").unwrap())
        .with_range(TextRange::new(0, 1).unwrap());
    TextBlock::new(bid(block), "text")
        .unwrap()
        .with_citations(vec![cite])
        .unwrap()
        .into()
}

fn calling_block(n: u64) -> ContentBlock {
    common::call(n, "t").into()
}

#[test]
fn given_references_to_existing_objects_when_pushed_then_they_are_not_second_declarations() {
    let mut conversation = empty();
    conversation.push_turn(open_turn(10).into()).unwrap();
    conversation
        .push_entry(&tid(10), calling(11, &[(12, "t")]))
        .unwrap();
    conversation
        .push_entry(&tid(10), mount(13, "alpha"))
        .unwrap();

    conversation
        .push_entry(&tid(10), delivery(20, &[(21, 12, "t")]))
        .unwrap();
    conversation
        .push_entry(&tid(10), common::turns::unmount(22, 13))
        .unwrap();

    assert_eq!(conversation.turns()[0].as_ai().unwrap().entries().len(), 4);
}

#[test]
fn given_human_and_agent_ids_when_repeated_or_equal_to_object_ids_then_accepted() {
    let mut conversation = empty();
    let shared_human = HumanId::from_uuid(eid(11).as_uuid()).unwrap();
    let shared_agent = AgentId::from_uuid(tid(10).as_uuid()).unwrap();
    let message = |entry: u64, block: u64| {
        HumanMessage::new(eid(entry), shared_human, vec![plain_text(block, "x")]).unwrap()
    };
    conversation
        .push_turn(
            br_llm_messages::InputTurn::new(tid(10), message(11, 12))
                .unwrap()
                .into(),
        )
        .unwrap();
    conversation
        .push_turn(
            br_llm_messages::InputTurn::new(tid(20), message(21, 22))
                .unwrap()
                .into(),
        )
        .unwrap();
    conversation
        .push_turn(AITurn::open(tid(30), shared_agent, author("a")).into())
        .unwrap();
    conversation
        .push_turn(AITurn::open(tid(40), shared_agent, author("b")).into())
        .unwrap();

    assert_eq!(conversation.turns().len(), 4);
}
