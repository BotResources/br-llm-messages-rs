mod common;

use br_llm_messages::{Citation, CitationId, CitationSource, ErrorCode, PlainText, TextRange};
use common::{code_of, refused, v7};
use serde_json::json;

fn citation(n: u64, start: u64, end: u64) -> Citation {
    Citation::new(
        CitationId::from_uuid(v7(n)).unwrap(),
        CitationSource::new("doc1", Some("Title".to_owned())).unwrap(),
        Some(TextRange::new(start, end).unwrap()),
    )
}

#[test]
fn given_empty_or_reversed_ranges_when_built_then_invalid_content() {
    assert_eq!(
        refused(TextRange::new(3, 3)).code(),
        ErrorCode::InvalidContent
    );
    assert_eq!(
        refused(TextRange::new(4, 3)).code(),
        ErrorCode::InvalidContent
    );
    assert_eq!(
        code_of::<TextRange>(r#"{"start":2,"end":2}"#),
        ErrorCode::InvalidContent
    );
    assert_eq!(TextRange::new(0, 1).unwrap().end(), 1);
}

#[test]
fn given_a_range_ending_at_the_text_end_when_validated_then_accepted_and_one_past_refused() {
    let text = PlainText::new("0123456789").unwrap();

    assert!(citation(1, 0, 10).validate_against(&text).is_ok());
    let error = refused(citation(2, 0, 11).validate_against(&text));
    assert_eq!(error.code(), ErrorCode::InvalidContent);
    assert_eq!(error.object_id(), Some(v7(2)));
}

#[test]
fn given_multibyte_text_when_ranges_are_validated_then_offsets_count_scalars_not_bytes() {
    let text = PlainText::new("é😀é").unwrap();

    assert_eq!(text.scalar_count(), 3);
    assert!(citation(1, 0, 3).validate_against(&text).is_ok());
    assert!(citation(2, 0, 4).validate_against(&text).is_err());
}

#[test]
fn given_a_citation_without_range_when_validated_then_accepted() {
    let whole = Citation::new(
        CitationId::from_uuid(v7(1)).unwrap(),
        CitationSource::new("doc", None).unwrap(),
        None,
    );

    assert!(
        whole
            .validate_against(&PlainText::new("x").unwrap())
            .is_ok()
    );
    assert_eq!(whole.range(), None);
}

#[test]
fn given_a_citation_list_with_a_repeated_id_when_checked_then_reused_identity_names_it() {
    let text = PlainText::new("0123456789").unwrap();
    let list = [citation(1, 0, 2), citation(2, 2, 4), citation(1, 5, 6)];

    let error = refused(Citation::check_all(&text, &list));

    assert_eq!(error.code(), ErrorCode::ReusedIdentity);
    assert_eq!(error.object_id(), Some(v7(1)));
    assert_eq!(error.location(), Some("citations[2]"));
    assert!(Citation::check_all(&text, &list[..2]).is_ok());
}

#[test]
fn given_a_citation_list_with_a_bad_range_when_checked_then_the_error_locates_it() {
    let text = PlainText::new("abc").unwrap();
    let list = [citation(1, 0, 3), citation(2, 1, 9)];

    let error = refused(Citation::check_all(&text, &list));

    assert_eq!(error.code(), ErrorCode::InvalidContent);
    assert_eq!(error.location(), Some("citations[1].range"));
}

#[test]
fn given_a_citation_when_round_tripped_then_golden_json_matches() {
    let id = v7(1);
    let wire = json!({
        "id": id.hyphenated().to_string(),
        "source": {"reference": "doc1", "title": "t"},
        "range": {"start": 0, "end": 4}
    });

    let loaded: Citation = serde_json::from_value(wire.clone()).unwrap();

    assert_eq!(serde_json::to_value(&loaded).unwrap(), wire);
    assert_eq!(loaded.source().reference(), "doc1");
    assert_eq!(loaded.source().title(), Some("t"));
    assert_eq!(
        code_of::<Citation>(&wire.to_string().replace("doc1", "")),
        ErrorCode::InvalidContent
    );
}

#[test]
fn given_an_incomplete_citation_document_when_loaded_then_refused() {
    let id = v7(1).hyphenated().to_string();
    let without_source = format!(r#"{{"id":"{id}"}}"#);
    let with_extra = format!(r#"{{"id":"{id}","source":{{"reference":"d"}},"extra":1}}"#);

    assert_eq!(
        code_of::<Citation>(&without_source),
        ErrorCode::MissingContent
    );
    assert_eq!(
        code_of::<Citation>(&with_extra),
        ErrorCode::UnsupportedField
    );
    assert_eq!(
        code_of::<CitationSource>(r#"{"reference":"","title":"t"}"#),
        ErrorCode::InvalidContent
    );
}
