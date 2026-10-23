mod common;

use br_llm_messages::{ErrorCode, OpaqueJson};
use common::refused;
use serde::Serialize;

#[derive(Serialize)]
struct Unordered {
    zebra: u8,
    apple: Inner,
}

#[derive(Serialize)]
struct Inner {
    yak: u8,
    ant: u8,
}

#[test]
fn given_decimals_that_round_wrongly_by_default_when_stored_then_they_equal_the_exact_f64() {
    for text in [
        "985.6906946328695",
        "2.2250738585072011e-308",
        "9007199254740993.0",
    ] {
        let expected: f64 = text.parse().unwrap();

        let stored = OpaqueJson::parse(text).unwrap();
        let through_serde = serde_json::from_str::<OpaqueJson>(text).unwrap();

        assert_eq!(
            stored.deserialize_into::<f64>().unwrap(),
            expected,
            "{text}"
        );
        assert_eq!(
            through_serde.deserialize_into::<f64>().unwrap(),
            expected,
            "{text}"
        );
    }
}

#[test]
fn given_a_decimal_above_two_to_the_fifty_three_when_stored_then_it_is_the_nearest_double() {
    let stored = OpaqueJson::parse("9007199254740993.0").unwrap();

    assert_eq!(stored.to_json_string().unwrap(), "9007199254740992.0");
}

#[test]
fn given_a_non_finite_float_when_stored_from_a_value_then_it_is_refused() {
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let error = refused(OpaqueJson::from_serialize(&value));

        assert_eq!(error.code(), ErrorCode::InvalidContent, "{value}");
    }
    assert_eq!(
        refused(OpaqueJson::from_serialize(&vec![1.0, f64::NAN])).code(),
        ErrorCode::InvalidContent
    );
    assert_eq!(
        refused(OpaqueJson::from_serialize(&f32::NAN)).code(),
        ErrorCode::InvalidContent
    );
}

#[test]
fn given_a_finite_float_when_stored_from_a_value_then_it_is_kept() {
    let stored = OpaqueJson::from_serialize(&vec![0.5_f64, -0.0, 1.5]).unwrap();

    assert_eq!(stored.to_json_string().unwrap(), "[0.5,-0.0,1.5]");
}

#[test]
fn given_fields_declared_out_of_order_when_stored_from_a_value_then_keys_are_sorted_at_every_level()
{
    let stored = OpaqueJson::from_serialize(&Unordered {
        zebra: 1,
        apple: Inner { yak: 2, ant: 3 },
    })
    .unwrap();

    assert_eq!(
        stored.to_json_string().unwrap(),
        "{\"apple\":{\"ant\":3,\"yak\":2},\"zebra\":1}"
    );
}

#[test]
fn given_keys_in_any_order_when_parsed_then_the_stored_text_is_sorted() {
    let stored = OpaqueJson::parse("{\"b\":{\"y\":1,\"x\":2},\"a\":[{\"d\":1,\"c\":2}]}").unwrap();

    assert_eq!(
        stored.to_json_string().unwrap(),
        "{\"a\":[{\"c\":2,\"d\":1}],\"b\":{\"x\":2,\"y\":1}}"
    );
}
