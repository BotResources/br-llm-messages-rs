mod common;
use common::Raw as _;

use br_llm_messages::{
    AITurn, ClosureReason, ErrorCode, IncomingEntry, InputTurn, Turn, TurnEntry, TurnKind,
};
use common::entries::all_entries;
use common::turns::{close, finished, human, human_message, input_turn, open_turn, tid};
use common::{aid, author, refused};
use serde_json::{Value, json};

fn incoming_entries() -> Vec<IncomingEntry> {
    all_entries()
        .into_iter()
        .filter_map(|entry| IncomingEntry::try_from(entry).ok())
        .collect()
}

#[test]
fn given_every_incoming_kind_when_input_turn_built_then_it_holds_exactly_that_entry() {
    let incoming = incoming_entries();
    assert_eq!(incoming.len(), 8);

    for (index, entry) in incoming.into_iter().enumerate() {
        let turn = InputTurn::new(tid(index as u64 + 900), vec![entry.clone()]).unwrap();

        assert_eq!(turn.entries(), std::slice::from_ref(&entry));
        assert_eq!(Turn::from(turn).kind(), TurnKind::InputTurn);
    }
}

#[test]
fn given_a_non_incoming_entry_in_input_turn_json_when_loaded_then_disallowed_kind() {
    for entry in all_entries().into_iter().filter(|e| !e.is_incoming()) {
        let json = json!({
            "type": "input_turn",
            "id": tid(1).to_string(),
            "entries": [entry.to_value().unwrap().raw()],
        });

        let error = refused(Turn::from_value(common::oj(json)));

        assert_eq!(error.code(), ErrorCode::DisallowedKind);
        assert_eq!(error.location(), Some("entries[0]"));
    }
}

#[test]
fn given_an_author_key_on_an_input_turn_when_loaded_then_author_forbidden() {
    let mut json = input_turn(1, 2).to_value().unwrap().raw();
    json["author"] = json!("someone");

    let error = refused(Turn::from_value(common::oj(json)));

    assert_eq!(error.code(), ErrorCode::AuthorForbidden);
    assert_eq!(error.object_id(), Some(tid(1).as_uuid()));
}

#[test]
fn given_an_input_turn_when_serialized_then_it_has_no_author_and_a_list_of_entries() {
    let json = input_turn(1, 2).to_value().unwrap().raw();

    assert_eq!(json["type"], "input_turn");
    assert!(json.get("author").is_none());
    assert!(json["entries"][0].is_object());
    assert_eq!(json["entries"].as_array().map(Vec::len), Some(1));
    assert!(json.get("entry").is_none());
}

#[test]
fn given_an_input_turn_id_equal_to_its_entry_id_when_built_then_reused_identity() {
    let entry = human_message(7);

    let error = refused(InputTurn::new(tid(7), vec![entry.into()]));

    assert_eq!(error.code(), ErrorCode::ReusedIdentity);
    assert_eq!(error.location(), Some("entries[0]"));
}

#[test]
fn given_agent_and_author_when_ai_turn_opened_then_it_starts_empty_and_open() {
    let turn = AITurn::open(tid(1), aid(2), author("Codex GPT-6.1-Sol high"));

    assert!(turn.entries().is_empty());
    assert!(!turn.is_closed());
    assert_eq!(turn.agent(), aid(2));
    assert_eq!(turn.state().closure(), None);
}

#[test]
fn given_an_author_with_odd_spacing_when_ai_turn_opened_then_it_is_kept_verbatim() {
    let label = "  Codex  GPT-6.1-Sol \u{00e9}  ";
    let turn = AITurn::open(tid(1), aid(2), author(label));

    let reloaded = AITurn::from_json(&turn.to_json().unwrap()).unwrap();

    assert_eq!(turn.author().as_str(), label);
    assert_eq!(reloaded.author().as_str(), label);
}

#[test]
fn given_the_same_label_on_two_agents_when_turns_opened_then_the_agents_stay_distinct() {
    let first = AITurn::open(tid(1), aid(2), author("same"));
    let second = AITurn::open(tid(3), aid(4), author("same"));

    assert_eq!(first.author(), second.author());
    assert_ne!(first.agent(), second.agent());
}

#[test]
fn given_ai_turn_json_without_author_when_loaded_then_invalid_content() {
    let mut json = open_turn(1).to_value().unwrap().raw();
    json.as_object_mut().unwrap().remove("author");

    assert_eq!(
        refused(Turn::from_value(common::oj(json))).code(),
        ErrorCode::InvalidContent
    );
}

#[test]
fn given_ai_turn_json_with_a_blank_author_or_no_agent_when_loaded_then_refused() {
    let mut blank = open_turn(1).to_value().unwrap().raw();
    blank["author"] = json!("   ");
    let mut no_agent = open_turn(1).to_value().unwrap().raw();
    no_agent.as_object_mut().unwrap().remove("agent");

    assert_eq!(
        refused(Turn::from_value(common::oj(blank))).code(),
        ErrorCode::InvalidAuthor
    );
    assert_eq!(
        refused(Turn::from_value(common::oj(no_agent))).code(),
        ErrorCode::InvalidContent
    );
}

#[test]
fn given_a_closed_turn_when_any_entry_kind_is_pushed_then_turn_closed() {
    for entry in all_entries() {
        let mut turn = open_turn(1);
        turn.push_entry(close(90, ClosureReason::completed()))
            .unwrap();

        let error = refused(turn.push_entry(entry.clone()));

        assert_eq!(error.code(), ErrorCode::ClosedTurn, "{}", entry.kind());
        assert_eq!(error.object_id(), Some(entry.id().as_uuid()));
        assert_eq!(turn.entries().len(), 1);
    }
}

#[test]
fn given_a_closure_when_a_second_one_is_pushed_then_turn_closed() {
    let mut turn = open_turn(1);
    turn.push_entry(close(10, ClosureReason::completed()))
        .unwrap();

    let error = refused(turn.push_entry(close(11, ClosureReason::cancelled())));

    assert_eq!(error.code(), ErrorCode::ClosedTurn);
}

#[test]
fn given_a_closure_in_the_middle_of_entries_json_when_loaded_then_turn_closed_at_the_next_entry() {
    let mut json = open_turn(1).to_value().unwrap().raw();
    json["entries"] = json!([
        close(10, ClosureReason::completed())
            .to_value()
            .unwrap()
            .raw(),
        human(11).to_value().unwrap().raw(),
    ]);

    let error = refused(AITurn::from_value(common::oj(json)));

    assert_eq!(error.code(), ErrorCode::ClosedTurn);
    assert_eq!(error.location(), Some("entries[1]"));
}

#[test]
fn given_a_turn_closed_after_entries_when_state_read_then_it_reports_the_reason() {
    let mut turn = open_turn(1);
    turn.push_entry(human(10)).unwrap();
    turn.push_entry(finished(11)).unwrap();
    turn.push_entry(close(12, ClosureReason::completed()))
        .unwrap();

    assert!(turn.is_closed());
    assert_eq!(turn.state().closure(), Some(&ClosureReason::completed()));
    assert!(matches!(
        turn.entries().last(),
        Some(TurnEntry::TurnClosure(_))
    ));
}

#[test]
fn given_the_turn_kind_when_named_then_discriminants_are_input_turn_and_ai_turn() {
    assert_eq!(TurnKind::InputTurn.as_str(), "input_turn");
    assert_eq!(TurnKind::AITurn.as_str(), "ai_turn");
    assert_eq!(
        serde_json::to_value(TurnKind::AITurn).unwrap(),
        Value::String("ai_turn".into())
    );
    assert_eq!("ai_turn".parse::<TurnKind>().unwrap(), TurnKind::AITurn);
    assert_eq!(
        refused("a_i_turn".parse::<TurnKind>()).code(),
        ErrorCode::VariantMismatch
    );
}
