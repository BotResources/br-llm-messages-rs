mod common;
mod projected_thread_support;

use br_llm_messages::{Conversation, ErrorCode, InputTurn, Turn};
use common::turns::tid;
use common::{aid, bid, cvid, oj, refused};
use projected_thread_support::*;
use serde_json::{Value, json};

type Change = fn(&mut Value);

#[test]
fn given_reused_identities_in_a_thread_when_loaded_then_they_are_located_at_the_entry() {
    let cases: [(Change, &str); 4] = [
        (
            |d| d["turns"][3]["entries"][0] = d["turns"][2]["entries"][0].clone(),
            "turns[3].entries[0]",
        ),
        (
            |d| d["turns"][4]["entries"][2] = d["turns"][4]["entries"][0].clone(),
            "turns[4].entries[2]",
        ),
        (
            |d| d["turns"][3]["id"] = d["turns"][2]["id"].clone(),
            "turns[3]",
        ),
        (
            |d| d["turns"][3]["entries"][0]["id"] = d["turns"][3]["id"].clone(),
            "turns[3].entries[0]",
        ),
    ];
    for (change, location) in cases {
        let mut document = document();
        change(&mut document);

        let error = refused(load(document));

        assert_eq!(error.code(), ErrorCode::ReusedIdentity, "{location}");
        assert_eq!(error.location(), Some(location));
    }
}

#[test]
fn given_outputs_not_sharing_one_agent_and_author_when_loaded_then_invalid_record_structure_at_the_first_that_differs_from_the_first_output()
 {
    let cases: [(Change, &str); 3] = [
        (
            |d| d["turns"][4]["entries"][2]["author"] = json!("Other"),
            "turns[4].entries[2]",
        ),
        (
            |d| d["turns"][4]["entries"][2]["agent"] = json!(aid(7).to_string()),
            "turns[4].entries[2]",
        ),
        (
            |d| {
                let copy = d["turns"][4]["entries"][2].clone();
                d["turns"][4]["entries"]
                    .as_array_mut()
                    .unwrap()
                    .insert(1, copy);
                d["turns"][4]["entries"][1]["id"] = json!(common::eid(990).to_string());
                d["turns"][4]["entries"][1]["content"][0]["id"] = json!(bid(991).to_string());
                d["turns"][4]["entries"][1]["author"] = json!("Second");
                d["turns"][4]["entries"][3]["author"] = json!("Third");
            },
            "turns[4].entries[1]",
        ),
    ];
    for (change, location) in cases {
        let mut document = document();
        change(&mut document);

        let error = refused(load(document));

        assert_eq!(error.code(), ErrorCode::InvalidRecordStructure);
        assert_eq!(error.location(), Some(location));
    }
}

#[test]
fn given_an_empty_input_turn_in_any_document_when_loaded_then_invalid_record_structure() {
    let mut thread = document();
    thread["turns"][0]["entries"] = json!([]);
    let standalone = json!({"type": "input_turn", "id": tid(1).to_string(), "entries": []});
    let conversation = json!({"schema": "br-llm-messages/5", "id": cvid(900).to_string(),
        "turns": [standalone.clone()]});

    for error in [
        refused(load(thread)),
        refused(InputTurn::from_value(oj(standalone.clone()))),
        refused(Turn::from_value(oj(standalone))),
        refused(Conversation::from_value(oj(conversation))),
    ] {
        assert_eq!(error.code(), ErrorCode::InvalidRecordStructure);
    }
}

#[test]
fn given_reused_identities_in_stored_documents_when_loaded_then_the_location_follows_the_document()
{
    let entry = |n: u64| {
        json!({"type": "human_message", "id": common::eid(n).to_string(),
            "human": common::hid(1).to_string(),
            "content": [{"type": "text", "id": bid(n + 500).to_string(), "text": "hi"}]})
    };
    let same_in_turn = json!({"type": "input_turn", "id": tid(1).to_string(),
        "entries": [entry(2), entry(3), entry(3)]});
    let conversation = json!({"schema": "br-llm-messages/5", "id": cvid(900).to_string(),
    "turns": [
        {"type": "input_turn", "id": tid(1).to_string(), "entries": [entry(2)]},
        {"type": "input_turn", "id": tid(5).to_string(), "entries": [entry(6), entry(2)]},
    ]});

    let standalone = refused(InputTurn::from_value(oj(same_in_turn)));
    let nested = refused(Conversation::from_value(oj(conversation)));

    assert_eq!(standalone.code(), ErrorCode::ReusedIdentity);
    assert_eq!(standalone.location(), Some("entries[2]"));
    assert_eq!(nested.code(), ErrorCode::ReusedIdentity);
    assert_eq!(nested.location(), Some("turns[1].entries[1]"));
}
