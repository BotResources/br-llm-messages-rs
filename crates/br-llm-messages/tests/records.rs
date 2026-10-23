mod common;
use common::Raw as _;

use br_llm_messages::{
    ActorRef, BlockRecord, ContentBlockKind, EntryRecord, RECORD_SCHEMA, RecordSet,
    RepresentationState, TurnEntryKind, TurnKind, TurnRecord,
};
use common::conversations::rich;
use common::records::{edited, rich_set, shuffled};
use common::turns::tid;
use common::{author, bid, cvid, eid, hid};
use serde_json::json;

#[test]
fn given_the_record_kinds_when_listed_then_discriminants_are_stable_snake_case() {
    let states: Vec<&str> = RepresentationState::ALL
        .iter()
        .map(|s| s.as_str())
        .collect();
    assert_eq!(
        states,
        ["producing", "complete", "interrupted", "discarded"]
    );
    let turns: Vec<&str> = TurnKind::ALL.iter().map(|k| k.as_str()).collect();
    assert_eq!(turns, ["input_turn", "ai_turn"]);
    assert_eq!(TurnEntryKind::ALL[12].as_str(), "turn_closure");
    assert_eq!(
        ContentBlockKind::ProviderToolExecutionRecord.as_str(),
        "provider_tool_execution_record"
    );
    let document = rich_set().to_value().unwrap().raw();
    assert_eq!(document["turns"][0]["kind"], "input_turn");
    assert_eq!(document["entries"][0]["state"], "complete");
    assert_eq!(document["entries"][0]["kind"], "human_message");
}

#[test]
fn given_a_record_set_when_serialized_then_it_carries_its_schema() {
    assert_eq!(RecordSet::schema(), "br-llm-messages-records/1");
    assert_eq!(RECORD_SCHEMA, RecordSet::schema());
    let document = rich_set().to_value().unwrap().raw();
    assert_eq!(document["schema"], RECORD_SCHEMA);
}

#[test]
fn given_another_schema_when_the_record_set_is_loaded_then_it_is_refused() {
    for schema in [
        json!("br-llm-messages-records/2"),
        json!("br-llm-messages/4"),
        json!(7),
    ] {
        let mut document = rich_set().to_value().unwrap().raw();
        document["schema"] = schema;
        let error = RecordSet::from_value(common::oj(document)).unwrap_err();
        assert_eq!(error.code(), br_llm_messages::ErrorCode::SchemaMismatch);
    }
    let mut document = rich_set().to_value().unwrap().raw();
    document.as_object_mut().unwrap().remove("schema");
    assert_eq!(
        RecordSet::from_value(common::oj(document))
            .unwrap_err()
            .code(),
        br_llm_messages::ErrorCode::SchemaMismatch
    );
}

#[test]
fn given_a_rich_conversation_when_decomposed_then_reconstruction_equals_it() {
    let original = rich();

    let rebuilt = rich_set().into_conversation(original.id()).unwrap();

    assert_eq!(rebuilt, original);
}

#[test]
fn given_shuffled_rows_when_reconstructed_then_the_conversation_is_equal() {
    let original = rich();
    let scrambled = shuffled(&rich_set());
    assert_ne!(scrambled, rich_set());

    let rebuilt = scrambled.into_conversation(original.id()).unwrap();

    assert_eq!(rebuilt, original);
}

#[test]
fn given_rows_through_json_when_reconstructed_then_the_conversation_is_equal() {
    let original = rich();
    let json = shuffled(&rich_set()).to_json().unwrap();

    let rebuilt = RecordSet::from_json(&json)
        .unwrap()
        .into_conversation(original.id())
        .unwrap();

    assert_eq!(rebuilt, original);
}

#[test]
fn given_single_turn_rows_when_reconstructed_as_turns_then_each_turn_is_equal() {
    let original = rich();
    let turn = original.turns()[1].clone();

    let rebuilt = RecordSet::from_turn(&turn, 4)
        .unwrap()
        .into_turns()
        .unwrap();

    assert_eq!(rebuilt, vec![turn]);
}

#[test]
fn given_the_sender_columns_when_decomposed_then_each_kind_maps_to_its_actor() {
    let set = rich_set();
    for row in set.entries() {
        match row.kind() {
            TurnEntryKind::HumanMessage => {
                assert!(matches!(row.sender(), Some(ActorRef::Human { .. })));
            }
            TurnEntryKind::AgentRequest | TurnEntryKind::AgentMail => {
                assert!(matches!(row.sender(), Some(ActorRef::Agent { .. })));
                assert!(row.author().is_some());
            }
            _ => {
                assert!(row.sender().is_none());
                assert!(row.author().is_none());
            }
        }
    }
    let ai: Vec<_> = set
        .turns()
        .iter()
        .filter(|turn| turn.kind() == TurnKind::AITurn)
        .collect();
    assert!(
        ai.iter()
            .all(|turn| turn.agent().is_some() && turn.author().is_some())
    );
}

#[test]
fn given_rows_built_by_hand_when_reconstructed_then_no_database_assumption_is_needed() {
    let turn = TurnRecord::new(tid(1), TurnKind::InputTurn, 0);
    let entry = EntryRecord::new(
        eid(2),
        tid(1),
        TurnEntryKind::HumanMessage,
        0,
        RepresentationState::Complete,
        common::oj(json!({})),
    )
    .with_sender(ActorRef::human(hid(1)));
    let block = BlockRecord::new(
        bid(3),
        eid(2),
        ContentBlockKind::Text,
        0,
        RepresentationState::Complete,
        common::oj(json!({"text": "hello"})),
    );

    let conversation = RecordSet::new()
        .with_turns(vec![turn])
        .with_entries(vec![entry])
        .with_blocks(vec![block])
        .into_conversation(cvid(9))
        .unwrap();

    assert_eq!(conversation.turns().len(), 1);
    assert!(conversation.entry(&eid(2)).is_ok());
}

#[test]
fn given_a_changed_author_label_when_reconstructed_then_it_is_kept_verbatim() {
    let set = edited(&rich_set(), |document| {
        document["turns"][1]["author"] = json!("  Padded Label  ");
    });
    let turn_author = set.turns()[1].author().unwrap().as_str().to_owned();
    assert_eq!(turn_author, "  Padded Label  ");
    let rebuilt = set.into_conversation(rich().id()).unwrap();
    assert_eq!(
        rebuilt.turns()[1].as_ai().unwrap().author(),
        &author("  Padded Label  ")
    );
}
