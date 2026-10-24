mod common;
use common::Raw as _;

use br_llm_messages::{
    ContentBlock, Conversation, DeltaStream, DraftSnapshot, Error, ErrorCode, RecordSet,
    ResponseDelta, Turn, TurnEntry,
};
use serde_json::Value;

const CONVERSATION: &str = include_str!("golden/rich_conversation.json");
const DELTA_STREAM: &str = include_str!("golden/delta_stream.json");
const DRAFT_OPEN: &str = include_str!("golden/draft_open.json");
const DRAFT_INTERRUPTED: &str = include_str!("golden/draft_interrupted.json");
const RECORD_SET: &str = include_str!("golden/record_set.json");

type Load = fn(Value) -> Result<Value, Error>;

fn document(text: &str) -> Value {
    serde_json::from_str(text).unwrap()
}

fn load_turn(value: Value) -> Result<Value, Error> {
    Turn::from_value(common::oj(value))?
        .to_value()
        .map(|v| v.raw())
}

fn load_entry(value: Value) -> Result<Value, Error> {
    TurnEntry::from_value(common::oj(value))?
        .to_value()
        .map(|v| v.raw())
}

fn load_block(value: Value) -> Result<Value, Error> {
    ContentBlock::from_value(common::oj(value))?
        .to_value()
        .map(|v| v.raw())
}

fn load_conversation(value: Value) -> Result<Value, Error> {
    Conversation::from_value(common::oj(value))?
        .to_value()
        .map(|v| v.raw())
}

fn load_delta(value: Value) -> Result<Value, Error> {
    ResponseDelta::from_value(common::oj(value))?
        .to_value()
        .map(|v| v.raw())
}

fn load_delta_stream(value: Value) -> Result<Value, Error> {
    DeltaStream::from_value(common::oj(value))?
        .to_value()
        .map(|v| v.raw())
}

fn load_checkpoint(value: Value) -> Result<Value, Error> {
    DraftSnapshot::from_value(common::oj(value))?
        .to_value()
        .map(|v| v.raw())
}

fn load_records(value: Value) -> Result<Value, Error> {
    RecordSet::from_value(common::oj(value))?
        .to_value()
        .map(|v| v.raw())
}

fn split(pointer: &str) -> (&str, &str) {
    pointer.rsplit_once('/').unwrap()
}

fn with_null(document: &Value, pointer: &str) -> Value {
    let mut edited = document.clone();
    *edited.pointer_mut(pointer).expect("the key exists") = Value::Null;
    edited
}

fn without(document: &Value, pointer: &str) -> Value {
    let mut edited = document.clone();
    let (parent, key) = split(pointer);
    edited
        .pointer_mut(parent)
        .and_then(Value::as_object_mut)
        .and_then(|object| object.remove(key))
        .expect("the key exists");
    edited
}

fn check_family(document: &Value, load: Load, pointers: &[&str]) {
    check_family_keeping(document, load, pointers, &[]);
}

fn check_family_keeping(document: &Value, load: Load, pointers: &[&str], needed: &[&str]) {
    assert_eq!(&load(document.clone()).unwrap(), document, "round trip");
    for pointer in pointers {
        let refused = load(with_null(document, pointer)).unwrap_err();
        assert_eq!(refused.code(), ErrorCode::InvalidContent, "{pointer} null");
        if needed.contains(pointer) {
            continue;
        }
        let loaded = load(without(document, pointer));
        assert!(loaded.is_ok(), "{pointer} absent: {:?}", loaded.err());
    }
}

fn subtree(document: &Value, pointer: &str) -> Value {
    document
        .pointer(pointer)
        .expect("the subtree exists")
        .clone()
}

fn conversation() -> Value {
    document(CONVERSATION)
}

#[test]
fn given_a_turn_with_an_optional_key_when_the_key_is_null_then_it_is_refused_and_when_absent_then_it_loads()
 {
    let turn = subtree(&conversation(), "/turns/1");

    check_family(&turn, load_turn, &["/observed_at"]);
}

#[test]
fn given_a_conversation_with_optional_keys_when_a_key_is_null_then_it_is_refused_and_when_absent_then_it_loads()
 {
    check_family_keeping(
        &conversation(),
        load_conversation,
        &[
            "/turns/0/observed_at",
            "/turns/1/entries/0/model",
            "/turns/1/entries/0/usage/input_tokens",
            "/turns/1/entries/0/content/0/replay",
            "/turns/1/entries/0/content/1/item_ref",
            "/turns/1/entries/1/results/0/content/0/citations/0/range",
            "/turns/1/entries/1/results/0/content/0/citations/0/source/title",
        ],
        &[
            "/turns/1/entries/0/usage/input_tokens",
            "/turns/1/entries/0/content/0/replay",
        ],
    );
}

#[test]
fn given_an_entry_with_optional_keys_when_a_key_is_null_then_it_is_refused_and_when_absent_then_it_loads()
 {
    let entry = subtree(&conversation(), "/turns/1/entries/0");

    check_family_keeping(
        &entry,
        load_entry,
        &[
            "/model",
            "/provider_response_id",
            "/usage",
            "/usage/input_tokens",
            "/observed_at",
        ],
        &["/usage/input_tokens"],
    );
}

#[test]
fn given_a_block_with_optional_keys_when_a_key_is_null_then_it_is_refused_and_when_absent_then_it_loads()
 {
    let conversation = conversation();
    let replayed = subtree(&conversation, "/turns/1/entries/0/content/0");
    let referenced = subtree(&conversation, "/turns/1/entries/0/content/1");

    check_family_keeping(
        &replayed,
        load_block,
        &["/observed_at", "/replay"],
        &["/replay"],
    );
    check_family(&referenced, load_block, &["/observed_at", "/item_ref"]);
}

#[test]
fn given_a_delta_with_optional_keys_when_a_key_is_null_then_it_is_refused_and_when_absent_then_it_loads()
 {
    let stream = document(DELTA_STREAM);
    let started = subtree(&stream, "/deltas/0");
    let provider = subtree(&stream, "/deltas/6");
    let interrupted = subtree(&stream, "/deltas/40");

    check_family(
        &started,
        load_delta,
        &["/metadata/model", "/metadata/observed_at"],
    );
    check_family(&provider, load_delta, &["/start/operation"]);
    check_family(&interrupted, load_delta, &["/reason/detail"]);
}

#[test]
fn given_a_delta_stream_with_optional_keys_when_a_key_is_null_then_it_is_refused_and_when_absent_then_it_loads()
 {
    check_family_keeping(
        &document(DELTA_STREAM),
        load_delta_stream,
        &[
            "/deltas/0/metadata/model",
            "/deltas/1/start/observed_at",
            "/deltas/2/content/observed_at",
            "/deltas/3/start/arguments",
            "/deltas/6/start/operation",
            "/deltas/13/update/usage/input_tokens",
            "/deltas/32/finalization/block/operation",
            "/deltas/32/finalization/block/status",
            "/deltas/40/reason/detail",
        ],
        &["/deltas/13/update/usage/input_tokens"],
    );
}

#[test]
fn given_a_draft_checkpoint_with_optional_keys_when_a_key_is_null_then_it_is_refused_and_when_absent_then_it_loads()
 {
    check_family_keeping(
        &document(DRAFT_OPEN),
        load_checkpoint,
        &[
            "/entry/observed_at",
            "/entry/payload/model",
            "/entry/payload/usage",
            "/entry/payload/usage/input_tokens",
            "/blocks/0/observed_at",
            "/blocks/0/payload/replay",
            "/blocks/2/payload/external_id",
            "/blocks/2/payload/provisional_arguments",
            "/blocks/3/payload/item_ref",
            "/blocks/5/payload/operation",
        ],
        &[
            "/entry/payload/usage/input_tokens",
            "/blocks/0/payload/replay",
        ],
    );
    check_family(
        &document(DRAFT_INTERRUPTED),
        load_checkpoint,
        &["/entry/payload/interruption/detail"],
    );
}

#[test]
fn given_a_record_set_with_optional_keys_when_a_key_is_null_then_it_is_refused_and_when_absent_then_it_loads()
 {
    check_family_keeping(
        &document(RECORD_SET),
        load_records,
        &[
            "/turns/14/observed_at",
            "/entries/0/author",
            "/entries/17/observed_at",
            "/entries/17/payload/model",
            "/entries/17/payload/provider_response_id",
            "/entries/17/payload/usage",
            "/entries/3/payload/summary",
            "/entries/6/payload/conversation",
            "/entries/1/payload/in_reply_to",
            "/entries/8/payload/reason/detail",
            "/blocks/17/observed_at",
            "/blocks/3/payload/item_ref",
            "/blocks/7/payload/external_id",
            "/blocks/7/payload/replay",
            "/blocks/8/payload/arguments",
            "/blocks/8/payload/operation",
            "/blocks/10/payload/status",
            "/blocks/17/payload/citations/0/range",
            "/blocks/17/payload/citations/0/source/title",
        ],
        &["/entries/0/author"],
    );
}
