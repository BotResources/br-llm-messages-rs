mod common;

use br_llm_messages::{ClosureReason, ErrorCode, TurnEntry};
use common::{eid, oj, refused};
use serde_json::json;

fn closure(reason: serde_json::Value) -> serde_json::Value {
    json!({"type": "turn_closure", "id": eid(1).to_string(), "reason": reason})
}

#[test]
fn given_every_reason_without_detail_when_serialized_then_only_the_type_is_written() {
    for (reason, name) in [
        (ClosureReason::completed(), "completed"),
        (ClosureReason::cancelled(), "cancelled"),
        (ClosureReason::interrupted(), "interrupted"),
        (ClosureReason::budget_exhausted(), "budget_exhausted"),
    ] {
        assert_eq!(
            serde_json::to_value(&reason).unwrap(),
            json!({"type": name})
        );
        assert_eq!(reason.detail(), None);
    }
}

#[test]
fn given_every_reason_with_a_detail_when_serialized_then_it_round_trips_unchanged() {
    let reasons = [
        ClosureReason::completed().with_detail("done").unwrap(),
        ClosureReason::cancelled().with_detail("user stop").unwrap(),
        ClosureReason::interrupted()
            .with_detail("shutdown")
            .unwrap(),
        ClosureReason::budget_exhausted()
            .with_detail("tokens")
            .unwrap(),
        ClosureReason::failed("provider down").unwrap(),
    ];
    for reason in reasons {
        let written = serde_json::to_value(&reason).unwrap();

        assert_eq!(written["detail"], json!(reason.detail().unwrap()));
        assert_eq!(
            oj(written).deserialize_into::<ClosureReason>().unwrap(),
            reason
        );
    }
}

#[test]
fn given_a_failed_reason_without_a_detail_when_loaded_then_invalid_content() {
    for reason in [
        json!({"type": "failed"}),
        json!({"type": "failed", "detail": null}),
    ] {
        let error = refused(TurnEntry::from_value(oj(closure(reason))));

        assert_eq!(error.code(), ErrorCode::InvalidContent);
        assert_eq!(error.location(), Some("detail"));
    }
}

#[test]
fn given_a_blank_detail_when_built_or_loaded_then_invalid_content_for_every_reason() {
    for blank in ["", " ", "\n\t "] {
        assert_eq!(
            refused(ClosureReason::failed(blank)).code(),
            ErrorCode::InvalidContent
        );
        assert_eq!(
            refused(ClosureReason::cancelled().with_detail(blank)).code(),
            ErrorCode::InvalidContent
        );
        for name in [
            "completed",
            "cancelled",
            "interrupted",
            "budget_exhausted",
            "failed",
        ] {
            let error = refused(TurnEntry::from_value(oj(closure(
                json!({"type": name, "detail": blank}),
            ))));

            assert_eq!(error.code(), ErrorCode::InvalidContent, "{name}");
        }
    }
}

#[test]
fn given_a_reason_with_a_field_other_than_detail_when_loaded_then_refused() {
    let budget = closure(json!({"type": "budget_exhausted", "budget": "tokens"}));
    let failure = closure(json!({"type": "failed", "detail": "x", "failure": {"code": "oom"}}));

    for document in [budget, failure] {
        assert_eq!(
            refused(TurnEntry::from_value(oj(document))).code(),
            ErrorCode::InvalidContent
        );
    }
}

#[test]
fn given_a_closure_with_an_unknown_reason_when_loaded_then_variant_mismatch() {
    let error = refused(TurnEntry::from_value(oj(closure(
        json!({"type": "abducted"}),
    ))));

    assert_eq!(error.code(), ErrorCode::VariantMismatch);
}

#[test]
fn given_a_detail_with_surrounding_spaces_when_kept_then_it_is_not_trimmed() {
    let reason = ClosureReason::failed("  spaced out  ").unwrap();

    assert_eq!(reason.detail(), Some("  spaced out  "));
}
