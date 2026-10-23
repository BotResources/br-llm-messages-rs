use super::ErrorCode;

fn position(code: ErrorCode) -> usize {
    match code {
        ErrorCode::InvalidIdentity => 0,
        ErrorCode::ReusedIdentity => 1,
        ErrorCode::InvalidAuthor => 2,
        ErrorCode::AuthorForbidden => 3,
        ErrorCode::MissingContent => 4,
        ErrorCode::InvalidContent => 5,
        ErrorCode::DisallowedEntryKind => 6,
        ErrorCode::DisallowedBlockKind => 7,
        ErrorCode::UnknownReference => 8,
        ErrorCode::WrongKindReference => 9,
        ErrorCode::DuplicateToolResult => 10,
        ErrorCode::ToolResultMismatch => 11,
        ErrorCode::TurnClosed => 12,
        ErrorCode::UnresolvedExecution => 13,
        ErrorCode::ContinuationPending => 14,
        ErrorCode::MalformedJson => 15,
        ErrorCode::SchemaMismatch => 16,
        ErrorCode::UnsupportedVariant => 17,
        ErrorCode::UnsupportedField => 18,
        ErrorCode::InvalidPosition => 19,
        ErrorCode::InvalidRecord => 20,
        ErrorCode::ReplayMismatch => 21,
        ErrorCode::StreamTarget => 22,
        ErrorCode::StreamPhase => 23,
        ErrorCode::InconsistentFinalization => 24,
    }
}

#[test]
fn given_every_error_code_when_listed_then_all_matches_the_exhaustive_variant_order() {
    for (index, code) in ErrorCode::ALL.iter().enumerate() {
        assert_eq!(position(*code), index);
    }
    assert_eq!(ErrorCode::ALL.len(), 25);
}
