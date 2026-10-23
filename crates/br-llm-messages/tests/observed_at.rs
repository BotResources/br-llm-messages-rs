mod common;

use br_llm_messages::{
    Conversation, ErrorCode, ModelResponse, ObservedAt, OpaqueJson, ResponseEnding,
    ResponseMetadata, Turn,
};
use common::turns::{finished, human, human_message, open_turn, pushed};
use common::{at, code_of, cvid, decode, eid, refused};

fn quoted(text: &str) -> String {
    serde_json::to_string(text).unwrap()
}

#[test]
fn given_utc_timestamps_with_uppercase_z_when_built_then_they_are_kept_exactly_as_given() {
    let accepted = [
        "2026-10-23T08:00:00Z",
        "2026-10-23T08:00:00.1Z",
        "2026-10-23T08:00:00.100Z",
        "2026-10-23T08:00:00.000000000Z",
        "2026-10-23T23:59:59.999999999Z",
        "2024-02-29T12:00:00Z",
        "2000-02-29T00:00:00Z",
        "1999-12-31T23:59:59Z",
        "0000-01-01T00:00:00Z",
        "9999-12-31T23:59:59Z",
        "2026-04-30T00:00:00Z",
    ];

    for text in accepted {
        let observed = ObservedAt::new(text).unwrap();

        assert_eq!(observed.as_str(), text);
        assert_eq!(observed.as_ref(), text);
    }
}

#[test]
fn given_text_that_is_not_an_rfc_3339_utc_timestamp_when_built_then_invalid_content() {
    let refusals = [
        ("", "empty"),
        ("2026-10-23T08:00:00z", "lowercase z"),
        ("2026-10-23t08:00:00Z", "lowercase t"),
        ("2026-10-23 08:00:00Z", "space separator"),
        ("2026-10-23T08:00:00+00:00", "numeric offset"),
        ("2026-10-23T08:00:00-00:00", "negative zero offset"),
        ("2026-10-23T09:00:00+01:00", "other offset"),
        ("2026-10-23T08:00:00", "no zone"),
        ("2026-10-23", "date only"),
        ("08:00:00Z", "time only"),
        ("2026-10-23T08:00Z", "no seconds"),
        ("2026-10-23T08:00:00.Z", "empty fraction"),
        ("2026-10-23T08:00:00.1234567890Z", "ten fraction digits"),
        ("2026-10-23T08:00:00,5Z", "comma fraction"),
        ("2026-10-23T08:00:00.5xZ", "letter in fraction"),
        ("2026-10-23T08:00:00ZZ", "double zone"),
        (" 2026-10-23T08:00:00Z", "leading space"),
        ("2026-10-23T08:00:00Z ", "trailing space"),
        ("2026-10-23T08:00:00Z\n", "trailing newline"),
        ("12026-10-23T08:00:00Z", "five year digits"),
        ("026-10-23T08:00:00Z", "three year digits"),
        ("2026-1-23T08:00:00Z", "one month digit"),
        ("2026-10-3T08:00:00Z", "one day digit"),
        ("2026/10/23T08:00:00Z", "slash separators"),
        ("2026-10-23T08-00-00Z", "dash time separators"),
        ("\u{ff12}026-10-23T08:00:00Z", "full width digit"),
        ("2026-10-23T08:00:00\u{3a3}", "non ascii zone"),
        ("+026-10-23T08:00:00Z", "signed year"),
    ];

    for (text, why) in refusals {
        let error = refused(ObservedAt::new(text));

        assert_eq!(error.code(), ErrorCode::InvalidContent, "{why}: {text:?}");
    }
}

#[test]
fn given_calendar_invalid_values_when_built_then_invalid_content() {
    let refusals = [
        ("2026-00-10T08:00:00Z", "month zero"),
        ("2026-13-10T08:00:00Z", "month thirteen"),
        ("2026-10-00T08:00:00Z", "day zero"),
        ("2026-10-32T08:00:00Z", "day thirty-two"),
        ("2026-04-31T08:00:00Z", "april has thirty days"),
        ("2026-06-31T08:00:00Z", "june has thirty days"),
        ("2026-09-31T08:00:00Z", "september has thirty days"),
        ("2026-11-31T08:00:00Z", "november has thirty days"),
        ("2026-02-29T08:00:00Z", "2026 is not a leap year"),
        ("1900-02-29T08:00:00Z", "1900 is not a leap year"),
        ("2100-02-29T08:00:00Z", "2100 is not a leap year"),
        ("2024-02-30T08:00:00Z", "february never has thirty days"),
        ("2026-10-23T24:00:00Z", "hour twenty-four"),
        ("2026-10-23T08:60:00Z", "minute sixty"),
        ("2026-10-23T08:00:60Z", "second sixty"),
        ("2026-10-23T99:99:99Z", "all out of range"),
    ];

    for (text, why) in refusals {
        let error = refused(ObservedAt::new(text));

        assert_eq!(error.code(), ErrorCode::InvalidContent, "{why}: {text:?}");
    }
}

#[test]
fn given_a_json_string_when_decoded_then_the_same_rules_apply_and_the_text_is_unchanged() {
    let kept = decode::<ObservedAt>(&quoted("2026-10-23T08:00:00.500Z")).unwrap();
    assert_eq!(kept.as_str(), "2026-10-23T08:00:00.500Z");

    for text in [
        "2026-10-23T08:00:00+00:00",
        "2026-10-23T08:00:00z",
        "2026-02-29T08:00:00Z",
    ] {
        assert_eq!(
            code_of::<ObservedAt>(&quoted(text)),
            ErrorCode::InvalidContent,
            "{text}"
        );
    }
    assert_eq!(
        code_of::<ObservedAt>("1788000000"),
        ErrorCode::InvalidContent
    );
    assert_eq!(code_of::<ObservedAt>("null"), ErrorCode::InvalidContent);
}

#[test]
fn given_an_observed_time_when_serialized_then_it_is_a_plain_string_that_loads_back_equal() {
    let observed = ObservedAt::new("2026-10-23T08:00:00.120Z").unwrap();

    let json = OpaqueJson::from_serialize(&observed).unwrap();
    let text = serde_json::to_string(&json.deserialize_into::<serde_json::Value>().unwrap());

    assert_eq!(text.unwrap(), "\"2026-10-23T08:00:00.120Z\"");
    assert_eq!(json.deserialize_into::<ObservedAt>().unwrap(), observed);
}

#[test]
fn given_entries_whose_times_run_backwards_when_pushed_then_history_keeps_its_recorded_order() {
    let mut stamped = open_turn(1);
    let mut plain = open_turn(1);
    let first = human_message(10).with_observed_at(at(50));
    let second = ModelResponse::new(
        eid(11),
        vec![],
        ResponseEnding::finished(),
        ResponseMetadata::new().with_observed_at(at(10)),
    )
    .unwrap();

    pushed(&mut stamped, vec![first.into(), second.into()]);
    pushed(&mut plain, vec![human(10), finished(11)]);

    let order: Vec<_> = stamped.entries().iter().map(|entry| entry.id()).collect();
    let expected: Vec<_> = plain.entries().iter().map(|entry| entry.id()).collect();
    assert_eq!(order, expected);
    assert_eq!(stamped.state(), plain.state());
    let times: Vec<_> = stamped
        .entries()
        .iter()
        .map(|entry| entry.observed_at().map(ObservedAt::as_str))
        .collect();
    assert_eq!(times, [Some(at(50).as_str()), Some(at(10).as_str())]);
}

#[test]
fn given_turns_whose_times_run_backwards_when_a_conversation_is_loaded_then_it_stays_valid_in_order()
 {
    let later = common::turns::input_turn(100, 10).with_observed_at(at(40));
    let earlier = common::turns::input_turn(200, 20).with_observed_at(at(5));
    let conversation =
        Conversation::from_parts(cvid(1), vec![later.into(), earlier.into()]).unwrap();

    let reloaded = Conversation::from_json(&conversation.to_json().unwrap()).unwrap();

    assert_eq!(reloaded, conversation);
    let ids: Vec<_> = reloaded.turns().iter().map(Turn::id).collect();
    assert_eq!(ids, [common::turns::tid(100), common::turns::tid(200)]);
}

#[test]
fn given_the_same_content_with_and_without_times_when_compared_then_they_are_not_equal() {
    let plain = common::turns::input_turn(1, 10);
    let stamped = common::turns::input_turn(1, 10).with_observed_at(at(1));

    assert_ne!(plain, stamped);
    assert_eq!(plain.observed_at(), None);
    assert_eq!(stamped.observed_at(), Some(&at(1)));
}
