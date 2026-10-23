mod common;

use br_llm_messages::{
    Author, Base64Data, Citation, CitationId, CitationSource, Error, ErrorCode, ExternalId,
    PlainText, ReplayMaterial, ToolName,
};
use common::{code_of, refused, v7};

const GOLDEN: [(ErrorCode, &str); 25] = [
    (ErrorCode::InvalidIdentity, "invalid_identity"),
    (ErrorCode::ReusedIdentity, "reused_identity"),
    (ErrorCode::InvalidAuthor, "invalid_author"),
    (ErrorCode::AuthorForbidden, "author_forbidden"),
    (ErrorCode::MissingContent, "missing_content"),
    (ErrorCode::InvalidContent, "invalid_content"),
    (ErrorCode::DisallowedEntryKind, "disallowed_entry_kind"),
    (ErrorCode::DisallowedBlockKind, "disallowed_block_kind"),
    (ErrorCode::UnknownReference, "unknown_reference"),
    (ErrorCode::WrongKindReference, "wrong_kind_reference"),
    (ErrorCode::DuplicateToolResult, "duplicate_tool_result"),
    (ErrorCode::ToolResultMismatch, "tool_result_mismatch"),
    (ErrorCode::TurnClosed, "turn_closed"),
    (ErrorCode::UnresolvedExecution, "unresolved_execution"),
    (ErrorCode::ContinuationPending, "continuation_pending"),
    (ErrorCode::MalformedJson, "malformed_json"),
    (ErrorCode::SchemaMismatch, "schema_mismatch"),
    (ErrorCode::UnsupportedVariant, "unsupported_variant"),
    (ErrorCode::UnsupportedField, "unsupported_field"),
    (ErrorCode::InvalidPosition, "invalid_position"),
    (ErrorCode::InvalidRecord, "invalid_record"),
    (ErrorCode::ReplayMismatch, "replay_mismatch"),
    (ErrorCode::StreamTarget, "stream_target"),
    (ErrorCode::StreamPhase, "stream_phase"),
    (
        ErrorCode::InconsistentFinalization,
        "inconsistent_finalization",
    ),
];

#[test]
fn given_every_error_code_when_named_then_the_snake_case_string_is_frozen() {
    assert_eq!(GOLDEN.len(), ErrorCode::ALL.len());
    for (code, text) in GOLDEN {
        assert_eq!(code.as_str(), text);
        assert_eq!(code.to_string(), text);
    }
}

#[test]
fn given_every_error_code_when_parsed_back_then_identity_holds() {
    for code in ErrorCode::ALL {
        assert_eq!(ErrorCode::parse(code.as_str()), Some(*code));
    }
    for text in [
        "",
        "Invalid_Identity",
        "invalid-identity",
        "invalid_identity ",
        "nope",
    ] {
        assert_eq!(ErrorCode::parse(text), None);
    }
}

const SENTINEL: &str = "SENTINEL-prompt-secret-9f3a";

fn assert_clean(error: &Error) {
    for rendered in [
        error.to_string(),
        format!("{error:?}"),
        format!("{error:#?}"),
    ] {
        assert!(!rendered.contains(SENTINEL), "{rendered}");
        assert!(!rendered.contains("9f3a"), "{rendered}");
    }
}

#[test]
fn given_refused_values_holding_a_sentinel_when_errors_are_rendered_then_no_payload_leaks() {
    assert_clean(&refused(ToolName::new(format!("{SENTINEL}.bad"))));
    assert_clean(&refused(Author::new(format!("   {}", " ".repeat(300)))));
    assert_clean(&refused(Author::new(SENTINEL.repeat(20))));
    assert_clean(&refused(Base64Data::new(format!("{SENTINEL}!"))));
    assert_clean(&refused(ExternalId::new("")));
    assert_clean(&refused(ToolName::new(SENTINEL.to_lowercase() + "?")));
    assert_clean(&refused(PlainText::new("  ")));
}

#[test]
fn given_json_with_sentinel_payloads_when_mapped_then_the_error_names_no_value() {
    let documents = [
        format!("{{\"format\":\"{SENTINEL}\",\"payload\":{{\"type\":\"text\",\"value\":\"s\"}}}}"),
        format!(
            "{{\"format\":\"a\",\"payload\":{{\"type\":\"json\",\"value\":\"{SENTINEL}\"}},\"x\":\"{SENTINEL}\"}}"
        ),
        format!(
            "{{\"format\":\"a\",\"payload\":{{\"type\":\"text\",\"value\":[\"{SENTINEL}\"]}}}}"
        ),
        format!("{{\"format\":\"a\",\"payload\":{{\"type\":\"{SENTINEL}\",\"value\":1}}}}"),
    ];
    for document in documents {
        let failure = serde_json::from_str::<ReplayMaterial>(&document).unwrap_err();
        assert_clean(&Error::from_json_error(&failure));
    }
}

#[test]
fn given_a_reused_citation_id_when_rendered_then_the_id_and_location_show_without_text() {
    let id = CitationId::from_uuid(v7(1)).unwrap();
    let source = CitationSource::new("doc").unwrap();
    let list = [Citation::new(id, source.clone()), Citation::new(id, source)];
    let text = PlainText::new(SENTINEL).unwrap();

    let error = refused(Citation::check_all(&text, &list));

    assert_eq!(
        error.to_string(),
        format!("reused_identity id={} at citations[1]", v7(1).hyphenated())
    );
    assert!(!format!("{error:?}").contains(SENTINEL));
}

#[test]
fn given_syntax_errors_when_mapped_then_malformed_json() {
    assert_eq!(code_of::<ToolName>("\"abc"), ErrorCode::MalformedJson);
    assert_eq!(
        code_of::<br_llm_messages::Usage>("{\"input_tokens\":"),
        ErrorCode::MalformedJson
    );
    assert_eq!(
        code_of::<br_llm_messages::Usage>("{,}"),
        ErrorCode::MalformedJson
    );
    assert_eq!(code_of::<ToolName>(""), ErrorCode::MalformedJson);
    assert_eq!(code_of::<ToolName>("nul"), ErrorCode::MalformedJson);
    assert_eq!(
        code_of::<ToolName>("\"abc\" trailing"),
        ErrorCode::MalformedJson
    );
}

#[test]
fn given_serde_data_errors_when_mapped_then_variant_field_and_missing_codes_are_recovered() {
    assert_eq!(
        code_of::<br_llm_messages::ExecutionStatus>("\"x\""),
        ErrorCode::UnsupportedVariant
    );
    assert_eq!(
        code_of::<br_llm_messages::Usage>(r#"{"nope":1}"#),
        ErrorCode::UnsupportedField
    );
    assert_eq!(
        code_of::<br_llm_messages::TextRange>(r#"{"start":1}"#),
        ErrorCode::MissingContent
    );
    assert_eq!(
        code_of::<br_llm_messages::TextRange>(r#"{"start":"a","end":2}"#),
        ErrorCode::InvalidContent
    );
    assert_eq!(code_of::<ToolName>("5"), ErrorCode::InvalidContent);
}

#[test]
fn given_embedded_library_errors_when_mapped_then_the_original_code_survives() {
    assert_eq!(code_of::<Author>("\"  \""), ErrorCode::InvalidAuthor);
    assert_eq!(
        code_of::<br_llm_messages::TurnId>("\"nil\""),
        ErrorCode::InvalidIdentity
    );
    assert_eq!(code_of::<Base64Data>("\"A\""), ErrorCode::InvalidContent);
    assert_eq!(
        code_of::<br_llm_messages::MediaType>("\"image/x\""),
        ErrorCode::UnsupportedVariant
    );
}

#[test]
fn given_a_syntax_error_when_mapped_then_the_location_is_a_position_and_not_the_input() {
    let failure = serde_json::from_str::<ToolName>(&format!("\"{SENTINEL}")).unwrap_err();

    let mapped = Error::from_json_error(&failure);

    assert_eq!(mapped.code(), ErrorCode::MalformedJson);
    assert_eq!(
        mapped.location(),
        Some(format!("line 1 column {}", SENTINEL.len() + 1).as_str())
    );
    assert_clean(&mapped);
}

#[test]
fn given_a_library_error_when_used_as_std_error_then_it_displays_its_code() {
    let error: Box<dyn std::error::Error + Send + Sync> = Box::new(refused(Author::new(" ")));

    assert_eq!(error.to_string(), "invalid_author");
}
