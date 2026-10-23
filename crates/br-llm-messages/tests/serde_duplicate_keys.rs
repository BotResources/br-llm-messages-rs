mod common;
mod snapshot_support;
mod stream_support;

use br_llm_messages::{
    AITurn, ContentBlock, Conversation, DraftSnapshot, ErrorCode, IncomingEntry, InputTurn,
    PlainBlock, RecordSet, ResponseDelta, StructuredBlock, ToolDefinition, Turn,
};
use common::conversations::rich;
use common::records::rich_set;
use common::{bid, code_of, tool};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use snapshot_support::{deltas, draft_after, script};
use stream_support::delta;

fn with_first_member_repeated(text: &str) -> String {
    let closing_quote = text[2..].find('"').expect("a first key") + 2;
    let colon_end = closing_quote + 2;
    let mut values = serde_json::Deserializer::from_str(&text[colon_end..]).into_iter::<Value>();
    values.next().expect("a first value").expect("valid json");
    let member = &text[1..colon_end + values.byte_offset()];
    format!("{{{member},{}", &text[1..])
}

fn assert_duplicates_refused<T: DeserializeOwned>(text: &str) {
    assert!(serde_json::from_str::<T>(text).is_ok(), "{text}");
    let duplicated = with_first_member_repeated(text);
    assert_eq!(
        code_of::<T>(&duplicated),
        ErrorCode::MalformedJson,
        "{duplicated}"
    );
}

#[test]
fn given_a_repeated_key_when_a_conversation_is_read_through_serde_then_malformed_json() {
    assert_duplicates_refused::<Conversation>(&rich().to_json().unwrap());
}

#[test]
fn given_a_repeated_key_when_a_turn_is_read_through_serde_then_malformed_json() {
    let conversation = rich();
    let input = conversation
        .turns()
        .iter()
        .find(|turn| matches!(turn, Turn::InputTurn(_)))
        .unwrap();
    let ai = conversation
        .turns()
        .iter()
        .find(|turn| matches!(turn, Turn::AITurn(_)))
        .unwrap();

    assert_duplicates_refused::<Turn>(&input.to_json().unwrap());
    assert_duplicates_refused::<Turn>(&ai.to_json().unwrap());
    let (Turn::InputTurn(input), Turn::AITurn(ai)) = (input, ai) else {
        unreachable!();
    };
    assert_duplicates_refused::<InputTurn>(&input.to_json().unwrap());
    assert_duplicates_refused::<AITurn>(&ai.to_json().unwrap());
}

#[test]
fn given_a_repeated_key_when_a_stream_document_is_read_through_serde_then_malformed_json() {
    let snapshot = draft_after(&deltas(&script()), 19)
        .to_snapshot()
        .unwrap()
        .to_json()
        .unwrap();
    let step = delta(br_llm_messages::DeltaOp::text_appended(bid(11), "x"))
        .to_json()
        .unwrap();

    assert_duplicates_refused::<DraftSnapshot>(&snapshot);
    assert_duplicates_refused::<ResponseDelta>(&step);
}

#[test]
fn given_a_repeated_key_when_a_record_set_is_read_through_serde_then_malformed_json() {
    assert_duplicates_refused::<RecordSet>(&rich_set().to_json().unwrap());
}

#[test]
fn given_a_repeated_key_when_incoming_entries_and_plain_blocks_are_read_through_serde_then_malformed_json()
 {
    let entry = IncomingEntry::try_from(common::entries::all_entries()[0].clone()).unwrap();

    assert_duplicates_refused::<IncomingEntry>(&entry.to_json().unwrap());
    assert_duplicates_refused::<PlainBlock>(&common::plain_text(1, "x").to_json().unwrap());
}

#[test]
fn given_a_repeated_key_inside_an_opaque_value_when_read_through_serde_then_malformed_json() {
    let block = ContentBlock::from(StructuredBlock::new(bid(1), json!({"k": 1})).unwrap());
    let block_text = block.to_json().unwrap();
    let definition = ToolDefinition::inline(tool("t"), "d", json!({"k": 1})).unwrap();
    let definition_text = serde_json::to_string(&definition).unwrap();

    let repeat = |text: &str| text.replacen("\"k\":1", "\"k\":1,\"k\":2", 1);
    let block_duplicated = repeat(&block_text);
    let definition_duplicated = repeat(&definition_text);
    assert_ne!(block_duplicated, block_text);
    assert_ne!(definition_duplicated, definition_text);
    assert_eq!(
        code_of::<ContentBlock>(&block_duplicated),
        ErrorCode::MalformedJson
    );
    assert_eq!(
        code_of::<ToolDefinition>(&definition_duplicated),
        ErrorCode::MalformedJson
    );
}
