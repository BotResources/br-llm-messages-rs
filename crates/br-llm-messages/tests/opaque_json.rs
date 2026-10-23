mod common;

use br_llm_messages::{ErrorCode, OpaqueJson};
use common::refused;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Payload {
    name: String,
    ratio: f64,
    tags: Vec<String>,
}

fn payload() -> Payload {
    Payload {
        name: "lookup".into(),
        ratio: 0.5,
        tags: vec!["a".into(), "b".into()],
    }
}

#[test]
fn given_a_serializable_value_when_stored_then_it_comes_back_as_the_same_type() {
    let stored = OpaqueJson::from_serialize(&payload()).unwrap();

    let back: Payload = stored.deserialize_into().unwrap();

    assert_eq!(back, payload());
}

#[test]
fn given_json_text_when_parsed_then_it_is_written_back_as_compact_json() {
    let parsed = OpaqueJson::parse(" { \"b\" : [ 1 , 2 ] , \"a\" : null } ").unwrap();

    assert_eq!(parsed.to_json_string().unwrap(), "{\"a\":null,\"b\":[1,2]}");
}

#[test]
fn given_a_repeated_key_when_parsed_then_the_text_is_refused() {
    let error = refused(OpaqueJson::parse("{\"a\":1,\"b\":{\"c\":1,\"c\":2}}"));

    assert_eq!(error.code(), ErrorCode::MalformedJson);
}

#[test]
fn given_malformed_text_when_parsed_then_the_text_is_refused() {
    for text in ["", "{", "[1,]", "{\"a\":1} x", "1e400"] {
        assert_eq!(
            refused(OpaqueJson::parse(text)).code(),
            ErrorCode::MalformedJson,
            "{text}"
        );
    }
}

#[test]
fn given_a_repeated_key_when_read_through_serde_then_the_text_is_refused() {
    let failure = serde_json::from_str::<OpaqueJson>("{\"a\":1,\"a\":2}").unwrap_err();

    assert!(failure.to_string().starts_with("malformed_json"));
}

#[test]
fn given_a_stored_value_when_serialized_with_serde_then_it_is_the_json_itself() {
    let stored = OpaqueJson::parse("{\"n\":[1,2.5]}").unwrap();

    assert_eq!(serde_json::to_string(&stored).unwrap(), "{\"n\":[1,2.5]}");
    assert_eq!(
        serde_json::from_str::<OpaqueJson>("{\"n\":[1,2.5]}").unwrap(),
        stored
    );
}

#[test]
fn given_a_value_of_the_wrong_shape_when_deserialized_into_a_type_then_it_is_refused() {
    let stored = OpaqueJson::parse("{\"name\":1}").unwrap();

    assert!(stored.deserialize_into::<Payload>().is_err());
}

#[test]
fn given_a_stored_value_when_debug_printed_then_the_json_is_not_shown() {
    let stored = OpaqueJson::parse("{\"secret\":\"SENTINEL\"}").unwrap();

    let printed = format!("{stored:?}");

    assert!(!printed.contains("SENTINEL"), "{printed}");
    assert!(!printed.contains("secret"), "{printed}");
}

#[test]
fn given_text_nested_beyond_the_parser_recursion_limit_when_parsed_then_malformed_json() {
    let text = format!("{}1{}", "[".repeat(300), "]".repeat(300));

    assert_eq!(
        refused(OpaqueJson::parse(&text)).code(),
        ErrorCode::MalformedJson
    );
}
