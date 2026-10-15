//! Entry ids, input ids and turn ids together, are unique in the
//! conversation: an input and a turn never share an id, and an entry is
//! found by its id. The rule holds at construction and on load.

use super::*;
use crate::block::{AssistantBlock, ToolCall, UserBlock};
use crate::stop_reason::StopReason;
use crate::user_input::UserSource;
use crate::value::{InputId, Text, ToolCallId, ToolName};

fn input(id: &str) -> UserInput {
    UserInput::new(
        InputId::new(id).unwrap(),
        UserSource::Human,
        None,
        vec![UserBlock::text(Text::new("hi").unwrap())],
    )
    .unwrap()
}

fn turn(id: &str, call: &str) -> Turn {
    let step = Step::new(
        vec![AssistantBlock::ToolCall(ToolCall {
            id: ToolCallId::new(call).unwrap(),
            name: ToolName::new("search").unwrap(),
            arguments: serde_json::json!({}),
        })],
        StopReason::AwaitingToolResults,
        None,
        None,
    )
    .unwrap();
    Turn::new(TurnId::new(id).unwrap(), None, step)
}

fn end_step() -> Step {
    Step::finished(vec![AssistantBlock::Text {
        text: Text::new("done").unwrap(),
    }])
}

fn duplicate(id: &str) -> MessageError {
    MessageError::DuplicateEntryId { id: id.to_owned() }
}

fn json(conversation: &Conversation) -> String {
    serde_json::to_string(conversation).unwrap()
}

/// Input `e1`, then turn `e2` calling `c1`.
fn conversation() -> Conversation {
    let mut conversation = Conversation::new();
    conversation.push_input(input("e1")).unwrap();
    conversation.push_turn(turn("e2", "c1")).unwrap();
    conversation
}

#[test]
fn given_an_id_used_by_an_entry_when_pushing_an_input_or_a_turn_then_refused_unchanged() {
    for id in ["e1", "e2"] {
        let mut conversation = conversation();
        let before = conversation.clone();

        assert_eq!(conversation.push_input(input(id)), Err(duplicate(id)));
        assert_eq!(conversation.push_turn(turn(id, "c9")), Err(duplicate(id)));

        assert_eq!(conversation, before);
        assert_eq!(json(&conversation), json(&before));
    }
}

#[test]
fn given_a_turn_reusing_an_entry_id_and_a_call_id_when_pushed_then_the_entry_id_is_reported() {
    let mut conversation = conversation();
    assert_eq!(
        conversation.push_turn(turn("e1", "c1")),
        Err(duplicate("e1"))
    );
}

#[test]
fn given_a_refused_entry_when_a_fresh_id_follows_then_accepted() {
    let mut conversation = conversation();
    conversation.push_input(input("e2")).unwrap_err();
    conversation.push_input(input("e3")).unwrap();
    conversation.push_turn(turn("e4", "c2")).unwrap();
    assert_eq!(conversation.entries().len(), 4);
}

#[test]
fn given_ids_differing_by_whitespace_when_pushed_then_distinct_entry_ids() {
    let mut conversation = Conversation::new();
    conversation.push_input(input(" ")).unwrap();
    conversation.push_input(input("e1")).unwrap();
    conversation.push_turn(turn(" e1", "c1")).unwrap();
    conversation.push_turn(turn("e1 ", "c2")).unwrap();
    assert_eq!(conversation.push_turn(turn(" ", "c3")), Err(duplicate(" ")));
    assert_eq!(conversation.entries().len(), 4);
}

#[test]
fn given_entries_when_looked_up_by_id_then_the_input_or_the_turn_is_found() {
    let conversation = conversation();
    assert_eq!(
        conversation.entry("e1"),
        Some(&Entry::UserInput(input("e1")))
    );
    assert_eq!(
        conversation.entry("e2"),
        Some(&Entry::Turn(turn("e2", "c1")))
    );
    assert_eq!(conversation.entry("ghost"), None);
    assert!(
        conversation
            .entries()
            .iter()
            .all(|entry| conversation.entry(entry.id()) == Some(entry))
    );
}

#[test]
fn given_the_id_of_an_input_when_reaching_a_turn_by_it_then_not_found() {
    let mut conversation = conversation();
    let id = TurnId::new("e1").unwrap();
    assert_eq!(conversation.turn(&id), None);
    assert_eq!(
        conversation.push_step(&id, end_step()),
        Err(MessageError::TurnNotFound { id: id.clone() })
    );
    let result = ToolResult::new(
        ToolCallId::new("c1").unwrap(),
        ToolName::new("search").unwrap(),
        Vec::new(),
        false,
    );
    assert_eq!(
        conversation.push_result(&id, result),
        Err(MessageError::TurnNotFound { id })
    );
}

fn document(schema: &str, entries: serde_json::Value) -> serde_json::Value {
    serde_json::json!({ "schema": schema, "entries": entries })
}

fn input_entry(id: &str) -> serde_json::Value {
    serde_json::json!({
        "type": "user_input",
        "id": id,
        "source": { "type": "human" },
        "content": [ { "type": "text", "text": "hi" } ]
    })
}

fn turn_entry(id: &str) -> serde_json::Value {
    serde_json::json!({
        "type": "turn",
        "id": id,
        "items": [
            {
                "type": "step",
                "content": [ { "type": "text", "text": "done" } ],
                "stop_reason": { "type": "end_turn" }
            }
        ]
    })
}

fn load_error(document: serde_json::Value) -> String {
    serde_json::from_value::<Conversation>(document)
        .unwrap_err()
        .to_string()
}

#[test]
fn given_two_entries_sharing_an_id_when_loaded_then_refused_with_the_rule() {
    let pairs = [
        [input_entry("e1"), input_entry("e1")],
        [input_entry("e1"), turn_entry("e1")],
        [turn_entry("e1"), input_entry("e1")],
        [turn_entry("e1"), turn_entry("e1")],
    ];
    for pair in pairs {
        let error = load_error(document(crate::SCHEMA_VERSION, serde_json::json!(pair)));
        assert!(error.contains(&duplicate("e1").to_string()), "{error}");
    }
}

#[test]
fn given_entries_with_distinct_ids_when_loaded_then_each_found_by_its_id() {
    let loaded: Conversation = serde_json::from_value(document(
        crate::SCHEMA_VERSION,
        serde_json::json!([input_entry("e1"), turn_entry("e2"), input_entry("e3")]),
    ))
    .unwrap();
    let ids: Vec<&str> = loaded.entries().iter().map(Entry::id).collect();
    assert_eq!(ids, ["e1", "e2", "e3"]);
    assert!(matches!(loaded.entry("e2"), Some(Entry::Turn(_))));
    assert!(matches!(loaded.entry("e3"), Some(Entry::UserInput(_))));
}

#[test]
fn given_a_wrong_schema_and_a_duplicate_entry_id_when_loaded_then_the_schema_wins() {
    let error = load_error(document(
        "br-llm-messages/2",
        serde_json::json!([input_entry("e1"), turn_entry("e1")]),
    ));
    let schema = MessageError::SchemaMismatch {
        found: "br-llm-messages/2".to_owned(),
    };
    assert!(error.contains(&schema.to_string()), "{error}");
    assert!(!error.contains(&duplicate("e1").to_string()), "{error}");
}

#[test]
fn given_a_wrong_schema_and_a_refused_entry_when_loaded_then_the_entry_wins() {
    let error = load_error(document(
        "br-llm-messages/2",
        serde_json::json!([input_entry("")]),
    ));
    let blank = MessageError::Blank { field: "input_id" };
    assert!(error.contains(&blank.to_string()), "{error}");
    assert!(!error.contains("unexpected schema"), "{error}");
}

/// On every state of the walks: each entry is found by its id, and an input
/// or a turn reusing the id of any entry, input or turn, is refused, leaving
/// the conversation unchanged, at construction and on load.
#[test]
fn given_every_state_of_random_walks_when_an_entry_id_is_reused_then_refused_both_ways() {
    let (mut inputs_reused, mut turns_reused) = (0, 0);
    for seed in 0..100 {
        let mut state = 0;
        crate::test_walk::walk(seed, 40, |_, conversation| {
            let mut probe = conversation.clone();
            for entry in conversation.entries() {
                let id = entry.id();
                assert_eq!(conversation.entry(id), Some(entry), "seed {seed}");
                assert_eq!(probe.push_input(input(id)), Err(duplicate(id)));
                assert_eq!(probe.push_turn(turn(id, "fresh")), Err(duplicate(id)));
                match entry {
                    Entry::UserInput(_) => inputs_reused += 1,
                    Entry::Turn(_) => turns_reused += 1,
                }
            }
            assert_eq!(&probe, conversation, "seed {seed}");

            let entries = conversation.entries();
            let reused = entries[state % entries.len()].id();
            let stored = serde_json::to_value(conversation).unwrap();
            for appended in [input_entry(reused), turn_entry(reused)] {
                let mut tampered = stored.clone();
                tampered["entries"].as_array_mut().unwrap().push(appended);
                let error = load_error(tampered);
                assert!(error.contains(&duplicate(reused).to_string()), "{error}");
            }
            state += 1;
        });
    }
    assert!(inputs_reused > 1_000 && turns_reused > 1_000);
}
