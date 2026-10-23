mod common;

use br_llm_messages::{
    Author, ErrorCode, ExternalId, FailureDetail, ModelName, PlainText, ProviderOperation, ToolName,
};
use common::{code_of, refused};

#[test]
fn given_256_multibyte_scalars_when_author_built_then_accepted_untrimmed() {
    let name = "é".repeat(256);

    assert_eq!(Author::new(name.clone()).unwrap().as_str(), name);
}

#[test]
fn given_257_scalars_when_author_built_then_invalid_author() {
    assert_eq!(
        refused(Author::new("é".repeat(257))).code(),
        ErrorCode::InvalidAuthor
    );
    assert_eq!(
        refused(Author::new("a".repeat(257))).code(),
        ErrorCode::InvalidAuthor
    );
}

#[test]
fn given_a_blank_author_when_built_then_invalid_author() {
    for blank in ["", " ", "\t\n", "\u{00a0}\u{2003}"] {
        assert_eq!(refused(Author::new(blank)).code(), ErrorCode::InvalidAuthor);
    }
}

#[test]
fn given_an_author_with_surrounding_space_when_built_then_kept_exactly() {
    let label = "  Codex GPT-6.1-Sol high \n";

    let author = Author::new(label).unwrap();

    assert_eq!(author.as_str(), label);
    assert_eq!(
        serde_json::to_string(&author).unwrap(),
        serde_json::to_string(label).unwrap()
    );
    assert_eq!(
        serde_json::from_str::<Author>(&serde_json::to_string(label).unwrap()).unwrap(),
        author
    );
}

#[test]
fn given_json_with_an_invalid_author_when_loaded_then_invalid_author() {
    assert_eq!(code_of::<Author>("\"   \""), ErrorCode::InvalidAuthor);
    assert_eq!(
        code_of::<Author>(&format!("\"{}\"", "x".repeat(257))),
        ErrorCode::InvalidAuthor
    );
}

#[test]
fn given_blank_text_when_plain_text_built_then_invalid_content() {
    for blank in ["", "   ", "\n\t", "\u{2003}"] {
        assert_eq!(
            refused(PlainText::new(blank)).code(),
            ErrorCode::InvalidContent
        );
    }
    assert_eq!(code_of::<PlainText>("\" \""), ErrorCode::InvalidContent);
}

#[test]
fn given_text_with_odd_spacing_and_composed_forms_when_built_then_kept_byte_for_byte() {
    let text = "  e\u{0301} \u{00e9}\r\n  tail  ";

    let plain = PlainText::new(text).unwrap();

    assert_eq!(plain.as_str(), text);
    assert_eq!(plain.scalar_count(), text.chars().count() as u64);
    let reloaded: PlainText =
        serde_json::from_str(&serde_json::to_string(&plain).unwrap()).unwrap();
    assert_eq!(reloaded.as_str(), text);
}

#[test]
fn given_the_tool_name_grammar_when_names_are_checked_then_only_valid_ones_pass() {
    for valid in ["a", "_", "_x", "Get-Weather_2", &"a".repeat(64)] {
        assert_eq!(ToolName::new(valid).unwrap().as_str(), valid);
    }
    let too_long = "a".repeat(65);
    for invalid in [
        "", &too_long, "1abc", "-a", "a b", "a.b", "a/b", "é", "tëst", "a\n",
    ] {
        assert_eq!(
            refused(ToolName::new(invalid)).code(),
            ErrorCode::InvalidContent,
            "{invalid:?}"
        );
    }
    assert_eq!(code_of::<ToolName>("\"1abc\""), ErrorCode::InvalidContent);
}

#[test]
fn given_external_values_when_built_then_non_empty_and_preserved_exactly() {
    let odd = "  resp_ABC é\n";

    assert_eq!(ExternalId::new(odd).unwrap().as_str(), odd);
    assert_eq!(ModelName::new(odd).unwrap().as_str(), odd);
    assert_eq!(
        refused(ExternalId::new("")).code(),
        ErrorCode::InvalidContent
    );
    assert_eq!(
        refused(ModelName::new("")).code(),
        ErrorCode::InvalidContent
    );
    assert_eq!(code_of::<ExternalId>("\"\""), ErrorCode::InvalidContent);
    let back: ExternalId =
        serde_json::from_str(&serde_json::to_string(&ExternalId::new(odd).unwrap()).unwrap())
            .unwrap();
    assert_eq!(back.as_str(), odd);
}

#[test]
fn given_provider_operation_names_when_built_then_any_non_empty_value_is_kept_as_given() {
    assert_eq!(
        ProviderOperation::new("web_search").unwrap().as_str(),
        "web_search"
    );
    let long = "a".repeat(4096);
    assert_eq!(ProviderOperation::new(long.clone()).unwrap().as_str(), long);
    assert_eq!(ProviderOperation::new("a\nb").unwrap().as_str(), "a\nb");
    assert!(ProviderOperation::new("").is_err());
}

#[test]
fn given_a_failure_detail_when_built_then_code_is_required_and_message_kept() {
    let detail = FailureDetail::new("rate_limited", Some("  later ".to_owned())).unwrap();

    assert_eq!(detail.code(), "rate_limited");
    assert_eq!(detail.message(), Some("  later "));
    assert_eq!(FailureDetail::new("x", None).unwrap().message(), None);
    assert!(FailureDetail::new("", None).is_err());
    let long = "x".repeat(4096);
    assert_eq!(FailureDetail::new(long.clone(), None).unwrap().code(), long);
    assert_eq!(
        code_of::<FailureDetail>("{\"code\":\"\"}"),
        ErrorCode::InvalidContent
    );
    assert_eq!(
        code_of::<FailureDetail>("{\"code\":\"a\",\"extra\":1}"),
        ErrorCode::UnsupportedField
    );
    assert_eq!(code_of::<FailureDetail>("{}"), ErrorCode::MissingContent);
    assert_eq!(
        serde_json::to_string(&FailureDetail::new("x", None).unwrap()).unwrap(),
        "{\"code\":\"x\"}"
    );
}
