use super::number_from_text;
use crate::error::ErrorCode;

#[test]
fn given_number_text_when_converted_then_integers_stay_exact_and_others_are_the_nearest_f64() {
    let cases = [
        ("18446744073709551615", "18446744073709551615"),
        ("-9223372036854775808", "-9223372036854775808"),
        ("1.50", "1.5"),
        ("1e2", "100.0"),
        ("-0", "-0.0"),
        ("985.6906946328695", "985.6906946328695"),
        ("18446744073709551616", "1.8446744073709552e+19"),
    ];

    for (text, expected) in cases {
        let number = number_from_text(text).unwrap();

        assert_eq!(serde_json::to_string(&number).unwrap(), expected, "{text}");
    }
}

#[test]
fn given_number_text_out_of_range_or_malformed_when_converted_then_it_is_refused() {
    for text in ["1e400", "-1e400", "abc", ""] {
        let error = number_from_text(text).unwrap_err();

        assert_eq!(error.code(), ErrorCode::MalformedJson, "{text}");
    }
}
