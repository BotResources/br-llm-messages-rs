mod common;

use br_llm_messages::{ErrorCode, ExecutionStatus, ResponseEnding, ResultStatus, Usage};
use common::{code_of, refused};
use serde_json::json;

#[test]
fn given_no_observation_when_usage_serialized_then_every_count_stays_absent() {
    let usage = Usage::new();

    assert_eq!(serde_json::to_value(&usage).unwrap(), json!({}));
    assert_eq!(usage.input_tokens(), None);
    assert_eq!(usage.thinking_tokens(), None);
    assert_eq!(serde_json::from_str::<Usage>("{}").unwrap(), usage);
}

#[test]
fn given_a_zero_count_when_usage_round_trips_then_zero_stays_distinct_from_absent() {
    let usage = Usage::new().with_input_tokens(0);

    let value = serde_json::to_value(&usage).unwrap();

    assert_eq!(value, json!({"input_tokens": 0}));
    let back: Usage = serde_json::from_value(value).unwrap();
    assert_eq!(back.input_tokens(), Some(0));
    assert_eq!(back.output_tokens(), None);
    assert_ne!(back, Usage::new());
}

#[test]
fn given_u64_max_counts_when_usage_round_trips_then_nothing_is_lost() {
    let usage = Usage::new()
        .with_input_tokens(u64::MAX)
        .with_output_tokens(u64::MAX - 1)
        .with_cache_read_input_tokens(1)
        .with_cache_creation_input_tokens(2)
        .with_thinking_tokens(3);

    let wire = serde_json::to_string(&usage).unwrap();
    let back: Usage = serde_json::from_str(&wire).unwrap();

    assert_eq!(back, usage);
    assert_eq!(back.input_tokens(), Some(u64::MAX));
    assert_eq!(back.output_tokens(), Some(u64::MAX - 1));
    assert_eq!(back.cache_read_input_tokens(), Some(1));
    assert_eq!(back.cache_creation_input_tokens(), Some(2));
    assert_eq!(back.thinking_tokens(), Some(3));
}

#[test]
fn given_unknown_or_malformed_usage_fields_when_loaded_then_refused() {
    assert_eq!(
        code_of::<Usage>(r#"{"input_tokens":1,"total_tokens":3}"#),
        ErrorCode::UnsupportedField
    );
    assert_eq!(
        code_of::<Usage>(r#"{"input_tokens":-1}"#),
        ErrorCode::InvalidContent
    );
    assert_eq!(
        code_of::<Usage>(r#"{"input_tokens":1.5}"#),
        ErrorCode::InvalidContent
    );
    assert_eq!(
        code_of::<Usage>(r#"{"input_tokens":18446744073709551616}"#),
        ErrorCode::InvalidContent
    );
    assert_eq!(
        serde_json::from_str::<Usage>(r#"{"input_tokens":null}"#).unwrap(),
        Usage::new()
    );
}

#[test]
fn given_every_ending_when_round_tripped_then_golden_json_is_stable() {
    let cases = [
        (ResponseEnding::finished(), json!({"type":"finished"})),
        (
            ResponseEnding::tool_results_requested(),
            json!({"type":"tool_results_requested"}),
        ),
        (
            ResponseEnding::provider_continuation(),
            json!({"type":"provider_continuation"}),
        ),
        (
            ResponseEnding::output_exhausted(),
            json!({"type":"output_exhausted"}),
        ),
        (
            ResponseEnding::stop_sequence(),
            json!({"type":"stop_sequence"}),
        ),
        (ResponseEnding::refusal(), json!({"type":"refusal"})),
        (ResponseEnding::filtered(), json!({"type":"filtered"})),
        (
            ResponseEnding::unclassified("pause_turn").unwrap(),
            json!({"type":"unclassified","reason":"pause_turn"}),
        ),
    ];
    for (ending, wire) in cases {
        assert_eq!(serde_json::to_value(&ending).unwrap(), wire);
        assert_eq!(
            serde_json::from_value::<ResponseEnding>(wire).unwrap(),
            ending
        );
    }
}

#[test]
fn given_an_unclassified_reason_when_round_tripped_then_kept_verbatim() {
    let reason = "  Provider/Reason \u{00e9}\n";

    let ending = ResponseEnding::unclassified(reason).unwrap();
    let back: ResponseEnding =
        serde_json::from_str(&serde_json::to_string(&ending).unwrap()).unwrap();

    match back {
        ResponseEnding::Unclassified { reason: kept, .. } => assert_eq!(kept, reason),
        other => panic!("unexpected {other:?}"),
    }
    assert_eq!(
        refused(ResponseEnding::unclassified("")).code(),
        ErrorCode::InvalidContent
    );
}

#[test]
fn given_malformed_endings_when_loaded_then_refused_with_the_precise_code() {
    assert_eq!(
        code_of::<ResponseEnding>(r#"{"type":"finished","x":1}"#),
        ErrorCode::UnsupportedField
    );
    assert_eq!(
        code_of::<ResponseEnding>(r#"{"type":"finished","reason":"r"}"#),
        ErrorCode::UnsupportedField
    );
    assert_eq!(
        code_of::<ResponseEnding>(r#"{"type":"unclassified"}"#),
        ErrorCode::MissingContent
    );
    assert_eq!(
        code_of::<ResponseEnding>(r#"{"type":"unclassified","reason":""}"#),
        ErrorCode::InvalidContent
    );
    assert_eq!(
        code_of::<ResponseEnding>(r#"{"type":"stopped"}"#),
        ErrorCode::UnsupportedVariant
    );
    assert_eq!(
        code_of::<ResponseEnding>(r#"{}"#),
        ErrorCode::MissingContent
    );
}

#[test]
fn given_execution_and_result_statuses_when_round_tripped_then_snake_case() {
    for (status, text) in [
        (ExecutionStatus::Pending, "pending"),
        (ExecutionStatus::Running, "running"),
        (ExecutionStatus::Completed, "completed"),
        (ExecutionStatus::Failed, "failed"),
        (ExecutionStatus::Incomplete, "incomplete"),
    ] {
        assert_eq!(serde_json::to_value(&status).unwrap(), json!(text));
        assert_eq!(
            serde_json::from_value::<ExecutionStatus>(json!(text)).unwrap(),
            status
        );
    }
    assert_eq!(
        serde_json::to_value(ResultStatus::Success).unwrap(),
        json!("success")
    );
    assert_eq!(
        serde_json::to_value(ResultStatus::Error).unwrap(),
        json!("error")
    );
    assert_eq!(
        code_of::<ExecutionStatus>("\"cancelled\""),
        ErrorCode::UnsupportedVariant
    );
}
