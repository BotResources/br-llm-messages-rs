#![cfg(feature = "mint")]

mod common;

use br_llm_messages::{
    BlockId, CitationId, Conversation, ConversationId, EntryId, ErrorCode, HumanMessage, InputTurn,
    PlainBlock, TextBlock, TurnId, Uuid,
};
use common::{hid, refused};
use uuid::{Variant, Version};

macro_rules! for_each_minted {
    ($body:ident) => {
        $body!(ConversationId);
        $body!(TurnId);
        $body!(EntryId);
        $body!(BlockId);
        $body!(CitationId);
    };
}

#[test]
fn given_the_mint_option_when_an_id_is_minted_then_it_is_an_ordinary_lowercase_uuid_v7() {
    macro_rules! check {
        ($id:ident) => {
            let (first, second) = ($id::mint(), $id::mint());
            let uuid: Uuid = first.as_uuid();
            assert_eq!(uuid.get_version(), Some(Version::SortRand));
            assert_eq!(uuid.get_variant(), Variant::RFC4122);
            assert_ne!(first, second);
            assert_eq!($id::parse(&first.to_string()).unwrap(), first);
            assert_eq!($id::from_uuid(uuid).unwrap(), first);
            assert_eq!(first.to_string(), first.to_string().to_lowercase());
        };
    }
    for_each_minted!(check);
}

#[test]
fn given_minted_ids_when_a_conversation_is_built_then_every_rule_applies_to_them() {
    let conversation_id = ConversationId::mint();
    let turn = TurnId::mint();
    let entry = EntryId::mint();
    let block = PlainBlock::try_from(TextBlock::new(BlockId::mint(), "hello").unwrap()).unwrap();
    let message = HumanMessage::new(entry, hid(1), vec![block.clone()]).unwrap();
    let mut conversation = Conversation::new(conversation_id);
    conversation
        .push_turn(InputTurn::new(turn, vec![message.into()]).unwrap().into())
        .unwrap();

    let again = HumanMessage::new(entry, hid(1), vec![block]).unwrap();
    let reuse = conversation.push_incoming(again, TurnId::mint());

    assert_eq!(refused(reuse).code(), ErrorCode::ReusedIdentity);
    assert_eq!(conversation.check(), Ok(()));
    assert!(conversation.entry(&entry).is_ok());
}
