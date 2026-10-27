mod common;
use common::Raw as _;

use br_llm_messages::{
    ContentBlockKind, Conversation, DELTA_SCHEMA, DRAFT_SCHEMA, DeltaStream, DraftSnapshot,
    ErrorCode, RECORD_SCHEMA, RecordSet, RepresentationState, ResponseDelta, ResponseDeltaKind,
    ResponseDraft, TurnEntryKind, TurnKind,
};
use serde_json::{Value, json};
use std::collections::{BTreeMap, HashSet};

const DELTA_STREAM: &str = include_str!("golden/delta_stream.json");
const DRAFT_OPEN: &str = include_str!("golden/draft_open.json");
const DRAFT_INTERRUPTED: &str = include_str!("golden/draft_interrupted.json");
const RECORD_SET: &str = include_str!("golden/record_set.json");
const RECORD_SET_RICH: &str = include_str!("golden/record_set_rich.json");
const RICH_CONVERSATION: &str = include_str!("golden/rich_conversation.json");

const FROZEN_FILES: [&str; 7] = [
    "delta_stream.json",
    "draft_interrupted.json",
    "draft_open.json",
    "projected_thread.json",
    "record_set.json",
    "record_set_rich.json",
    "rich_conversation.json",
];

fn pretty(value: &Value) -> String {
    serde_json::to_string_pretty(value).unwrap()
}

fn document(text: &str) -> Value {
    serde_json::from_str(text).unwrap()
}

fn stream() -> Vec<ResponseDelta> {
    DeltaStream::from_json(DELTA_STREAM).unwrap().into_deltas()
}

fn reserialized_stream(deltas: Vec<ResponseDelta>) -> String {
    pretty(&DeltaStream::new(deltas).to_value().unwrap().raw())
}

fn per_entry(deltas: Vec<ResponseDelta>) -> BTreeMap<String, Vec<ResponseDelta>> {
    let mut grouped: BTreeMap<String, Vec<ResponseDelta>> = BTreeMap::new();
    for delta in deltas {
        grouped
            .entry(delta.entry().to_string())
            .or_default()
            .push(delta);
    }
    grouped
}

#[test]
fn given_the_schema_ids_when_read_then_they_are_the_frozen_wire_names() {
    assert_eq!(DELTA_SCHEMA, "br-llm-messages-delta/1");
    assert_eq!(DRAFT_SCHEMA, "br-llm-messages-draft/1");
    assert_eq!(RECORD_SCHEMA, "br-llm-messages-records/1");
}

#[test]
fn given_the_frozen_documents_when_read_then_each_names_its_schema_constant() {
    let stream = document(DELTA_STREAM);
    assert_eq!(stream["schema"], DELTA_SCHEMA);
    for item in stream["deltas"].as_array().unwrap() {
        assert!(item.get("schema").is_none());
    }
    for text in [DRAFT_OPEN, DRAFT_INTERRUPTED] {
        assert_eq!(document(text)["schema"], DRAFT_SCHEMA);
    }
    assert_eq!(document(RECORD_SET)["schema"], RECORD_SCHEMA);
}

#[test]
fn given_the_frozen_delta_stream_when_loaded_then_it_reserializes_byte_identical() {
    let deltas = stream();

    assert_eq!(reserialized_stream(deltas.clone()), DELTA_STREAM);
    for delta in &deltas {
        assert_eq!(
            ResponseDelta::from_json(&delta.to_json().unwrap()).unwrap(),
            *delta
        );
    }
}

#[test]
fn given_the_frozen_delta_stream_when_inspected_then_every_operation_kind_is_present() {
    let present: HashSet<ResponseDeltaKind> = stream().iter().map(|delta| delta.kind()).collect();

    let missing: Vec<_> = ResponseDeltaKind::ALL
        .iter()
        .filter(|kind| !present.contains(kind))
        .collect();
    assert!(missing.is_empty(), "{missing:?}");
}

#[test]
fn given_the_frozen_delta_stream_when_replayed_then_the_draft_accepts_every_delta() {
    let entries = per_entry(stream());

    assert_eq!(entries.len(), 2);
    let mut interrupted = 0;
    for deltas in entries.values() {
        let mut draft = ResponseDraft::new(deltas[0].turn(), deltas[0].entry());
        for delta in deltas {
            draft.apply(delta).unwrap();
        }
        if let Ok(snapshot) = draft.to_snapshot() {
            assert_eq!(snapshot.entry().state(), RepresentationState::Interrupted);
            interrupted += 1;
        }
    }
    assert_eq!(interrupted, 1);
}

#[test]
fn given_the_frozen_open_checkpoint_when_loaded_then_it_reserializes_byte_identical() {
    let snapshot = DraftSnapshot::from_json(DRAFT_OPEN).unwrap();

    assert_eq!(pretty(&snapshot.to_value().unwrap().raw()), DRAFT_OPEN);
    assert_eq!(
        DraftSnapshot::from_json(&snapshot.to_json().unwrap()).unwrap(),
        snapshot
    );
    assert_eq!(snapshot.entry().state(), RepresentationState::Producing);
    let states: HashSet<_> = snapshot.blocks().iter().map(|row| row.state()).collect();
    assert_eq!(
        states,
        HashSet::from([
            RepresentationState::Producing,
            RepresentationState::Complete,
        ])
    );
    assert!(ResponseDraft::from_snapshot(&snapshot).is_ok());
}

#[test]
fn given_the_frozen_interrupted_checkpoint_when_loaded_then_it_reserializes_byte_identical() {
    let snapshot = DraftSnapshot::from_json(DRAFT_INTERRUPTED).unwrap();

    assert_eq!(
        pretty(&snapshot.to_value().unwrap().raw()),
        DRAFT_INTERRUPTED
    );
    assert_eq!(snapshot.entry().state(), RepresentationState::Interrupted);
    let states: HashSet<_> = snapshot.blocks().iter().map(|row| row.state()).collect();
    assert!(states.contains(&RepresentationState::Interrupted));
    assert!(!states.contains(&RepresentationState::Producing));
    assert!(ResponseDraft::from_snapshot(&snapshot).is_ok());
}

#[test]
fn given_the_frozen_record_set_when_loaded_then_it_reserializes_byte_identical() {
    let set = RecordSet::from_json(RECORD_SET).unwrap();

    assert_eq!(pretty(&set.to_value().unwrap().raw()), RECORD_SET);
    assert_eq!(RecordSet::from_json(&set.to_json().unwrap()).unwrap(), set);
    assert!(set.check_local().is_ok());
}

#[test]
fn given_the_frozen_rich_record_set_when_rebuilt_then_it_is_byte_identical_and_equals_the_rich_conversation()
 {
    let conversation = Conversation::from_json(RICH_CONVERSATION).unwrap();
    let set = RecordSet::from_json(RECORD_SET_RICH).unwrap();

    assert_eq!(pretty(&set.to_value().unwrap().raw()), RECORD_SET_RICH);
    assert_eq!(
        set.into_conversation(conversation.id()).unwrap(),
        conversation
    );
}

#[test]
fn given_the_frozen_record_set_when_inspected_then_every_row_kind_and_state_is_present() {
    let set = RecordSet::from_json(RECORD_SET).unwrap();

    let turn_kinds: HashSet<_> = set.turns().iter().map(|row| row.kind()).collect();
    let entry_kinds: HashSet<_> = set.entries().iter().map(|row| row.kind()).collect();
    let block_kinds: HashSet<_> = set.blocks().iter().map(|row| row.kind()).collect();
    let block_states: HashSet<_> = set.blocks().iter().map(|row| row.state()).collect();
    let entry_states: HashSet<_> = set.entries().iter().map(|row| row.state()).collect();
    assert!(TurnKind::ALL.iter().all(|kind| turn_kinds.contains(kind)));
    assert!(
        TurnEntryKind::ALL
            .iter()
            .filter(|kind| **kind != TurnEntryKind::RuntimeNotice)
            .all(|kind| entry_kinds.contains(kind))
    );
    assert!(!entry_kinds.contains(&TurnEntryKind::RuntimeNotice));
    assert!(
        ContentBlockKind::ALL
            .iter()
            .all(|kind| block_kinds.contains(kind))
    );
    assert!(
        RepresentationState::ALL
            .iter()
            .all(|state| block_states.contains(state))
    );
    assert_eq!(
        entry_states,
        HashSet::from([
            RepresentationState::Producing,
            RepresentationState::Complete,
            RepresentationState::Interrupted,
        ])
    );
}

#[test]
fn given_the_frozen_rich_conversation_when_loaded_then_it_reserializes_byte_identical() {
    let loaded = Conversation::from_json(RICH_CONVERSATION).unwrap();

    assert_eq!(pretty(&loaded.to_value().unwrap().raw()), RICH_CONVERSATION);
}

#[test]
fn given_the_frozen_documents_when_inspected_then_observed_at_reaches_every_wire_shape() {
    let deltas = stream();
    let started = |wanted: &str| {
        deltas
            .iter()
            .filter_map(|delta| delta.to_value().ok().map(|value| value.raw()))
            .filter(|value| value["type"] == wanted)
            .any(|value| value.to_string().contains("observed_at"))
    };
    assert!(started("response_started"));
    assert!(started("block_started"));
    assert!(started("block_inserted"));
    assert!(started("block_finalized"));
    let set = document(RECORD_SET);
    for collection in ["turns", "entries", "blocks"] {
        let rows = set[collection].as_array().unwrap();
        assert!(rows.iter().any(|row| row.get("observed_at").is_some()));
        assert!(rows.iter().any(|row| row.get("observed_at").is_none()));
    }
    for text in [DRAFT_OPEN, DRAFT_INTERRUPTED] {
        let snapshot = document(text);
        assert!(snapshot["entry"].get("observed_at").is_some());
        let blocks = snapshot["blocks"].as_array().unwrap();
        assert!(blocks.iter().any(|row| row.get("observed_at").is_some()));
    }
    assert!(RICH_CONVERSATION.matches("\"observed_at\"").count() >= 20);
}

#[test]
fn given_the_golden_directory_when_listed_then_every_file_is_frozen_by_a_loader_assertion() {
    let directory = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/golden");

    let mut names: Vec<String> = std::fs::read_dir(directory)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    names.sort();

    assert_eq!(names, FROZEN_FILES);
}

#[test]
fn given_a_frozen_document_edited_beyond_the_loader_when_loaded_then_it_is_refused() {
    let mut delta = document(DELTA_STREAM)["deltas"][0].clone();
    delta["extra"] = json!(1);
    let mut documented = document(DELTA_STREAM);
    documented["cursor"] = json!(1);
    let mut draft = document(DRAFT_OPEN);
    draft["schema"] = json!("br-llm-messages-draft/2");
    let mut records = document(RECORD_SET);
    records["cursor"] = json!(1);

    let refused = [
        ResponseDelta::from_value(common::oj(delta))
            .err()
            .map(|e| e.code()),
        DeltaStream::from_value(common::oj(documented))
            .err()
            .map(|e| e.code()),
        DraftSnapshot::from_value(common::oj(draft))
            .err()
            .map(|e| e.code()),
        RecordSet::from_value(common::oj(records))
            .err()
            .map(|e| e.code()),
    ];

    assert_eq!(
        refused,
        [
            Some(ErrorCode::InvalidContent),
            Some(ErrorCode::InvalidContent),
            Some(ErrorCode::SchemaMismatch),
            Some(ErrorCode::InvalidContent),
        ]
    );
}
