use serde_json::Value;
use std::collections::BTreeSet;

const RECORD_SET: &str = include_str!("golden/record_set.json");

fn rows(collection: &str) -> Vec<Value> {
    let document: Value = serde_json::from_str(RECORD_SET).unwrap();
    document[collection].as_array().unwrap().clone()
}

fn payload_kinds(kind: &str, pointer: &str) -> BTreeSet<String> {
    rows("entries")
        .iter()
        .filter(|row| row["kind"] == kind)
        .filter_map(|row| row["payload"].pointer(pointer))
        .map(|value| value.as_str().unwrap().to_owned())
        .collect()
}

fn names(wanted: &[&str]) -> BTreeSet<String> {
    wanted.iter().map(|name| (*name).to_owned()).collect()
}

#[test]
fn given_the_frozen_record_set_when_inspected_then_every_response_ending_is_present() {
    assert_eq!(
        payload_kinds("model_response", "/ending/type"),
        names(&[
            "filtering",
            "finished",
            "output_exhaustion",
            "provider_continuation",
            "refusal",
            "stop_sequence",
            "tool_use",
            "unclassified",
        ])
    );
}

#[test]
fn given_the_frozen_record_set_when_inspected_then_stop_sequence_appears_with_and_without_the_matched_text()
 {
    let stops: Vec<Value> = rows("entries")
        .iter()
        .filter_map(|row| row["payload"]["ending"].as_object().cloned())
        .filter(|ending| ending["type"] == "stop_sequence")
        .map(Value::Object)
        .collect();

    assert!(stops.iter().any(|ending| ending.get("sequence").is_some()));
    assert!(stops.iter().any(|ending| ending.get("sequence").is_none()));
}

#[test]
fn given_the_frozen_record_set_when_inspected_then_every_interruption_and_closure_reason_is_present()
 {
    assert_eq!(
        payload_kinds("model_response", "/interruption/kind"),
        names(&["cancelled", "failed", "interrupted", "truncated"])
    );
    assert_eq!(
        payload_kinds("turn_closure", "/reason/type"),
        names(&[
            "budget_exhausted",
            "cancelled",
            "completed",
            "failed",
            "interrupted",
        ])
    );
}

#[test]
fn given_the_frozen_record_set_when_inspected_then_results_carry_both_error_flag_values() {
    let flags: BTreeSet<bool> = rows("blocks")
        .iter()
        .filter(|row| row["kind"] == "runtime_tool_result")
        .map(|row| row["payload"]["is_error"].as_bool().unwrap())
        .collect();

    assert_eq!(flags, BTreeSet::from([false, true]));
}
