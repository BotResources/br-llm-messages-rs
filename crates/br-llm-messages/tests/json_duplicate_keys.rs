mod common;
mod snapshot_support;
mod stream_support;

use br_llm_messages::{
    ContentBlock, Conversation, DraftSnapshot, ErrorCode, RecordSet, ResponseDelta,
    StructuredBlock, TurnEntry,
};
use common::conversations::rich;
use common::records::rich_set;
use common::{bid, refused};
use snapshot_support::{deltas, draft_after, script};
use stream_support::{delta, finalize, json, ok, run, start_json};

fn repeat_first(text: &str, member: &str) -> String {
    assert!(text.contains(member), "{member} not in {text}");
    text.replacen(member, &format!("{member},{member}"), 1)
}

fn code_of<T>(result: Result<T, br_llm_messages::Error>) -> ErrorCode {
    refused(result).code()
}

#[test]
fn given_a_repeated_top_level_key_when_a_conversation_is_loaded_then_malformed_json() {
    let text = rich().to_json().unwrap();
    assert!(Conversation::from_json(&text).is_ok());
    let id = format!("\"id\":\"{}\"", common::cvid(1000).as_uuid().hyphenated());

    let duplicated = repeat_first(&text, &id);

    assert_eq!(
        code_of(Conversation::from_json(&duplicated)),
        ErrorCode::MalformedJson
    );
}

#[test]
fn given_a_repeated_key_nested_in_a_turn_when_a_conversation_is_loaded_then_malformed_json() {
    let text = rich().to_json().unwrap();
    assert!(Conversation::from_json(&text).is_ok());

    let duplicated = repeat_first(&text, "\"type\":\"human_message\"");

    assert_eq!(
        code_of(Conversation::from_json(&duplicated)),
        ErrorCode::MalformedJson
    );
}

#[test]
fn given_a_key_repeated_through_an_escape_when_loaded_then_malformed_json() {
    let text = rich().to_json().unwrap();
    assert!(Conversation::from_json(&text).is_ok());

    let duplicated = text.replacen("\"turns\":", "\"tu\\u0072ns\":[],\"turns\":", 1);

    assert_eq!(
        code_of(Conversation::from_json(&duplicated)),
        ErrorCode::MalformedJson
    );
}

#[test]
fn given_a_repeated_key_inside_an_opaque_value_when_loaded_then_malformed_json() {
    let block = StructuredBlock::new(bid(1), serde_json::json!({"k": 1})).unwrap();
    let text = ContentBlock::from(block).to_json().unwrap();
    assert!(ContentBlock::from_json(&text).is_ok());

    let duplicated = text.replacen("\"k\":1", "\"k\":1,\"k\":2", 1);

    assert_eq!(
        code_of(ContentBlock::from_json(&duplicated)),
        ErrorCode::MalformedJson
    );
}

#[test]
fn given_a_repeated_key_when_each_document_kind_is_loaded_then_all_refuse_it() {
    let set = rich_set().to_json().unwrap();
    let snapshot = draft_after(&deltas(&script()), 19)
        .to_snapshot()
        .unwrap()
        .to_json()
        .unwrap();
    let step = delta(br_llm_messages::DeltaOp::text_appended(bid(11), "x"))
        .to_json()
        .unwrap();
    let entry = common::entries::all_entries()[0].to_json().unwrap();

    assert!(RecordSet::from_json(&set).is_ok());
    assert!(DraftSnapshot::from_json(&snapshot).is_ok());
    assert!(ResponseDelta::from_json(&step).is_ok());
    assert!(TurnEntry::from_json(&entry).is_ok());

    let refusals = [
        code_of(RecordSet::from_json(&repeat_first(
            &set,
            "\"schema\":\"br-llm-messages-records/1\"",
        ))),
        code_of(DraftSnapshot::from_json(&repeat_first(
            &snapshot,
            "\"schema\":\"br-llm-messages-draft/1\"",
        ))),
        code_of(ResponseDelta::from_json(&repeat_first(
            &step,
            "\"schema\":\"br-llm-messages-delta/1\"",
        ))),
        code_of(TurnEntry::from_json(&repeat_first(
            &entry,
            "\"type\":\"human_message\"",
        ))),
    ];

    assert_eq!(refusals, [ErrorCode::MalformedJson; 4]);
}

#[test]
fn given_raw_json_with_a_repeated_key_when_a_draft_block_is_finalized_then_it_is_refused() {
    let mut draft = run(vec![start_json(20, 0), json(20, "{\"a\":1,\"a\":2}")]);
    let before = draft.clone();

    let error = refused(draft.apply(&delta(finalize(20))));

    assert_eq!(error.code(), ErrorCode::MalformedJson);
    assert_eq!(draft, before);
    ok(&mut draft, json(20, " "));
}
