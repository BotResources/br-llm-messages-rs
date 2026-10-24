mod common;

use br_llm_messages::{
    BlockId, BlockRef, ContentBlockKind, Conversation, EntryId, EntryRef, ErrorCode,
    ResponseEnding, Turn, TurnEntryKind, TurnId,
};
use common::conversations::rich;
use common::turns::{hosted_call, open_turn, response, tid};
use common::{bid, cid, cvid, eid, refused, text_block};

fn empty() -> Conversation {
    Conversation::new(cvid(1))
}

#[test]
fn given_a_rich_conversation_when_looked_up_by_identity_then_each_kind_is_returned() {
    let conversation = rich();

    assert_eq!(conversation.turn(&tid(10)).unwrap().id(), tid(10));
    let input = conversation.entry(&eid(2)).unwrap();
    assert!(matches!(input, EntryRef::Incoming(_)));
    assert_eq!(input.kind(), TurnEntryKind::HumanMessage);
    let response = conversation.entry(&eid(12)).unwrap();
    assert!(matches!(response, EntryRef::Turn(_)));
    assert_eq!(response.kind(), TurnEntryKind::ModelResponse);
    assert_eq!(
        conversation.entry(&eid(30)).unwrap().kind(),
        TurnEntryKind::TurnClosure
    );
}

#[test]
fn given_nested_blocks_when_looked_up_then_the_right_block_kind_is_returned() {
    let conversation = rich();
    let expectations = [
        (3, ContentBlockKind::Text),
        (13, ContentBlockKind::Thinking),
        (15, ContentBlockKind::RuntimeToolCall),
        (17, ContentBlockKind::ProviderToolResult),
        (18, ContentBlockKind::Text),
        (20, ContentBlockKind::Text),
        (22, ContentBlockKind::RuntimeToolResult),
        (23, ContentBlockKind::Text),
        (24, ContentBlockKind::Image),
        (25, ContentBlockKind::StructuredContent),
        (27, ContentBlockKind::Text),
        (42, ContentBlockKind::Text),
    ];

    for (id, kind) in expectations {
        let block = conversation.block(&bid(id)).unwrap();
        assert_eq!(block.id(), bid(id));
        assert_eq!(block.kind(), kind, "block {id}");
    }
    assert!(matches!(
        conversation.block(&bid(22)).unwrap(),
        BlockRef::RuntimeResult(_)
    ));
    assert!(matches!(
        conversation.block(&bid(24)).unwrap(),
        BlockRef::Plain(_)
    ));
    assert!(matches!(
        conversation.block(&bid(13)).unwrap(),
        BlockRef::Content(_)
    ));
}

#[test]
fn given_a_runtime_call_id_when_looked_up_then_the_call_is_returned() {
    let conversation = rich();

    let call = conversation.runtime_call(&bid(15)).unwrap();

    assert_eq!(call.name().as_str(), "lookup");
    assert_eq!(call.external_id().unwrap().as_str(), "toolu_1");
}

#[test]
fn given_a_non_call_id_when_looked_up_as_a_call_then_unknown_reference() {
    let conversation = rich();

    for id in [14, 16, 22, 24] {
        let error = refused(conversation.runtime_call(&bid(id)));
        assert_eq!(error.code(), ErrorCode::UnknownReference, "block {id}");
        assert_eq!(error.object_id(), Some(bid(id).as_uuid()));
    }
    let entry_id = BlockId::from_uuid(eid(12).as_uuid()).unwrap();
    assert_eq!(
        refused(conversation.runtime_call(&entry_id)).code(),
        ErrorCode::UnknownReference
    );
    assert_eq!(
        refused(conversation.runtime_call(&bid(999))).code(),
        ErrorCode::UnknownReference
    );
}

#[test]
fn given_ids_of_another_kind_when_looked_up_then_unknown_reference() {
    let conversation = rich();
    let block_as_entry = EntryId::from_uuid(bid(14).as_uuid()).unwrap();
    let entry_as_turn = TurnId::from_uuid(eid(12).as_uuid()).unwrap();
    let citation_as_block = BlockId::from_uuid(cid(4).as_uuid()).unwrap();

    assert_eq!(
        refused(conversation.entry(&block_as_entry)).code(),
        ErrorCode::UnknownReference
    );
    assert_eq!(
        refused(conversation.turn(&entry_as_turn)).code(),
        ErrorCode::UnknownReference
    );
    assert_eq!(
        refused(conversation.block(&citation_as_block)).code(),
        ErrorCode::UnknownReference
    );
    assert_eq!(
        refused(conversation.turn(&tid(999))).code(),
        ErrorCode::UnknownReference
    );
}

#[test]
fn given_cross_conversation_references_when_pushed_then_they_are_kept_and_never_looked_up() {
    let conversation = rich();

    let TurnEntryKind::AgentRequest = conversation.entry(&eid(41)).unwrap().kind() else {
        panic!("request expected");
    };
    let Turn::InputTurn(input) = conversation.turn(&tid(40)).unwrap() else {
        panic!("input turn expected");
    };
    let br_llm_messages::IncomingEntry::AgentRequest(request) = input.entry() else {
        panic!("request expected");
    };
    assert_eq!(request.source_conversation(), cvid(2));
    assert_eq!(
        request.origin(),
        &br_llm_messages::RequestOrigin::tool_call(bid(900))
    );
    assert_eq!(
        refused(conversation.block(&bid(900))).code(),
        ErrorCode::UnknownReference
    );
}

#[test]
fn given_a_task_and_launched_conversation_when_reloaded_then_they_are_kept_verbatim() {
    let conversation = rich();
    let started = conversation.entry(&eid(52)).unwrap();

    let br_llm_messages::TurnEntry::TaskStarted(started) = (match started {
        EntryRef::Turn(entry) => entry.clone(),
        _ => panic!("task started lives in an ai turn"),
    }) else {
        panic!("task started expected");
    };

    assert_eq!(started.conversation(), Some(cvid(3)));
    assert_eq!(started.task(), common::kid(1));
    let reloaded = Conversation::from_json(&conversation.to_json().unwrap()).unwrap();
    assert_eq!(reloaded, conversation);
}

#[test]
fn given_a_provider_call_when_looked_up_then_its_kind_is_returned() {
    let mut conversation = empty();
    conversation.push_turn(open_turn(10).into()).unwrap();

    conversation
        .push_entry(
            &tid(10),
            response(
                11,
                vec![hosted_call(12), text_block(13, "t").into()],
                ResponseEnding::finished(),
            ),
        )
        .unwrap();

    assert_eq!(
        conversation.block(&bid(12)).unwrap().kind(),
        ContentBlockKind::ProviderToolCall
    );
}

#[test]
fn given_references_equal_to_own_ids_when_pushed_then_they_are_accepted_and_not_indexed() {
    let mut conversation = rich();
    let own_conversation =
        br_llm_messages::ConversationId::from_uuid(cvid(1000).as_uuid()).unwrap();
    let request = br_llm_messages::AgentRequest::new(
        eid(70),
        common::aid(2),
        common::author("Claude"),
        own_conversation,
        br_llm_messages::RequestOrigin::tool_call(bid(15)),
        vec![common::plain_text(71, "again")],
    )
    .unwrap();
    let turn = br_llm_messages::InputTurn::new(tid(72), request).unwrap();
    let before = conversation.block(&bid(15)).unwrap().id();

    conversation.push_turn(turn.into()).unwrap();

    assert_eq!(conversation.block(&bid(15)).unwrap().id(), before);
    assert_eq!(
        conversation.entry(&eid(70)).unwrap().kind(),
        TurnEntryKind::AgentRequest
    );
    assert_eq!(conversation.turns().len(), 6);
}
