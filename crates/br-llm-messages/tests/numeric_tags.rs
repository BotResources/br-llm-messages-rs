use br_llm_messages::{Conversation, DeltaStream, DraftSnapshot, Error, ErrorCode, RecordSet};
use serde_json::Value;

const DELTA_STREAM: &str = include_str!("golden/delta_stream.json");
const DRAFT_OPEN: &str = include_str!("golden/draft_open.json");
const DRAFT_INTERRUPTED: &str = include_str!("golden/draft_interrupted.json");
const RECORD_SET: &str = include_str!("golden/record_set.json");
const RECORD_SET_RICH: &str = include_str!("golden/record_set_rich.json");
const RICH_CONVERSATION: &str = include_str!("golden/rich_conversation.json");

fn tag_pointers(value: &Value, here: String, found: &mut Vec<String>) {
    match value {
        Value::Object(fields) => {
            if fields.get("type").is_some_and(Value::is_string) {
                found.push(format!("{here}/type"));
            }
            for (key, child) in fields {
                tag_pointers(child, format!("{here}/{key}"), found);
            }
        }
        Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                tag_pointers(child, format!("{here}/{index}"), found);
            }
        }
        _ => {}
    }
}

fn with_number_at(document: &str, pointer: &str, number: u64) -> String {
    let mut value: Value = serde_json::from_str(document).unwrap();
    *value.pointer_mut(pointer).unwrap() = Value::from(number);
    value.to_string()
}

fn loader_refuses_every_numeric_tag(document: &str, load: fn(&str) -> Result<(), Error>) {
    let value: Value = serde_json::from_str(document).unwrap();
    let mut pointers = Vec::new();
    tag_pointers(&value, String::new(), &mut pointers);
    assert!(!pointers.is_empty());
    for pointer in pointers {
        for number in 0..=9 {
            let mutant = with_number_at(document, &pointer, number);
            let error = load(&mutant).expect_err(&format!("{pointer} accepted number {number}"));
            assert_eq!(
                error.code(),
                ErrorCode::VariantMismatch,
                "{pointer} number {number}"
            );
        }
    }
}

#[test]
fn given_a_conversation_when_any_type_tag_is_a_number_then_loading_is_refused_as_variant_mismatch()
{
    loader_refuses_every_numeric_tag(RICH_CONVERSATION, |text| {
        Conversation::from_json(text).map(drop)
    });
}

#[test]
fn given_a_record_set_when_any_type_tag_is_a_number_then_loading_is_refused_as_variant_mismatch() {
    for document in [RECORD_SET, RECORD_SET_RICH] {
        loader_refuses_every_numeric_tag(document, |text| RecordSet::from_json(text).map(drop));
    }
}

#[test]
fn given_a_delta_stream_when_any_type_tag_is_a_number_then_loading_is_refused_as_variant_mismatch()
{
    loader_refuses_every_numeric_tag(DELTA_STREAM, |text| DeltaStream::from_json(text).map(drop));
}

#[test]
fn given_a_draft_snapshot_when_any_type_tag_is_a_number_then_loading_is_refused_as_variant_mismatch()
 {
    for document in [DRAFT_OPEN, DRAFT_INTERRUPTED] {
        loader_refuses_every_numeric_tag(document, |text| DraftSnapshot::from_json(text).map(drop));
    }
}
