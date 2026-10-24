mod common;

use br_llm_messages::{ContentBlock, Conversation, ErrorCode, ResultExposure, TurnEntry};
use common::conversations::rich;
use common::records::{edited, rich_set, row, try_edit};
use serde_json::json;

fn text(id: u64) -> String {
    common::v7(id).hyphenated().to_string()
}

fn uuid(id: u64) -> br_llm_messages::Uuid {
    common::v7(id)
}

fn exposure_of_the_record(conversation: &Conversation) -> ResultExposure {
    conversation
        .turns()
        .iter()
        .filter_map(|turn| turn.as_ai())
        .flat_map(|turn| turn.entries())
        .filter_map(|entry| match entry {
            TurnEntry::ModelResponse(response) => Some(response),
            _ => None,
        })
        .flat_map(|response| response.blocks())
        .find_map(|block| match block {
            ContentBlock::ProviderToolExecutionRecord(record) => Some(record.exposure()),
            _ => None,
        })
        .unwrap()
}

#[test]
fn given_a_record_row_when_its_child_rows_are_dropped_then_it_rebuilds_and_exposure_follows_the_content()
 {
    let without_child = edited(&rich_set(), |d| {
        d["blocks"]
            .as_array_mut()
            .unwrap()
            .retain(|row| row["id"] != text(20));
    });

    let rebuilt = without_child.into_conversation(rich().id()).unwrap();

    assert_eq!(exposure_of_the_record(&rich()), ResultExposure::Readable);
    assert_eq!(exposure_of_the_record(&rebuilt), ResultExposure::NotExposed);
}

#[test]
fn given_a_record_row_whose_payload_still_carries_an_exposure_when_loaded_then_invalid_content() {
    let error = try_edit(&rich_set(), |d| {
        row(d, "blocks", uuid(19))["payload"]["exposure"] = json!({"type": "not_exposed"});
    })
    .unwrap_err();

    assert_eq!(error.code(), ErrorCode::InvalidContent);
}
