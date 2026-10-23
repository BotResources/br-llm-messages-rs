mod common;

use std::time::{Duration, Instant};

use br_llm_messages::{ErrorCode, RecordSet};
use common::conversations::rich;
use common::records::{rich_set, row};
use common::v7;
use serde_json::{Value, json};

fn chained(length: u64) -> Value {
    let mut document = rich_set().to_value().unwrap();
    let template = row(&mut document, "blocks", v7(14)).clone();
    let blocks = document["blocks"].as_array_mut().unwrap();
    for index in 0..length {
        let mut copy = template.clone();
        copy["id"] = json!(v7(1_000_000 + index).hyphenated().to_string());
        copy["position"] = json!(0);
        if index > 0 {
            copy["parent"] = json!(v7(1_000_000 + index - 1).hyphenated().to_string());
        }
        blocks.push(copy);
    }
    document
}

#[test]
fn given_a_long_parent_chain_when_loaded_then_it_is_refused_quickly_as_a_disallowed_kind() {
    let set = RecordSet::from_value(chained(50_000)).unwrap();
    let started = Instant::now();

    let error = set.into_conversation(rich().id()).unwrap_err();

    assert_eq!(error.code(), ErrorCode::DisallowedBlockKind);
    assert!(started.elapsed() < Duration::from_secs(20));
}
