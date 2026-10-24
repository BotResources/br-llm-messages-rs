#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorCode {
    InvalidIdentity,
    ReusedIdentity,
    InvalidAuthor,
    AuthorForbidden,
    InvalidContent,
    DisallowedKind,
    UnknownReference,
    DuplicateResult,
    MismatchedResult,
    EntryOutOfOrder,
    ClosedTurn,
    UnresolvedExecution,
    InvalidRecordStructure,
    MalformedJson,
    SchemaMismatch,
    VariantMismatch,
    StreamTargetError,
    StreamPhaseError,
    InconsistentFinalization,
}

impl ErrorCode {
    pub const ALL: &'static [ErrorCode] = &[
        ErrorCode::InvalidIdentity,
        ErrorCode::ReusedIdentity,
        ErrorCode::InvalidAuthor,
        ErrorCode::AuthorForbidden,
        ErrorCode::InvalidContent,
        ErrorCode::DisallowedKind,
        ErrorCode::UnknownReference,
        ErrorCode::DuplicateResult,
        ErrorCode::MismatchedResult,
        ErrorCode::EntryOutOfOrder,
        ErrorCode::ClosedTurn,
        ErrorCode::UnresolvedExecution,
        ErrorCode::InvalidRecordStructure,
        ErrorCode::MalformedJson,
        ErrorCode::SchemaMismatch,
        ErrorCode::VariantMismatch,
        ErrorCode::StreamTargetError,
        ErrorCode::StreamPhaseError,
        ErrorCode::InconsistentFinalization,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            ErrorCode::InvalidIdentity => "invalid_identity",
            ErrorCode::ReusedIdentity => "reused_identity",
            ErrorCode::InvalidAuthor => "invalid_author",
            ErrorCode::AuthorForbidden => "author_forbidden",
            ErrorCode::InvalidContent => "invalid_content",
            ErrorCode::DisallowedKind => "disallowed_kind",
            ErrorCode::UnknownReference => "unknown_reference",
            ErrorCode::DuplicateResult => "duplicate_result",
            ErrorCode::MismatchedResult => "mismatched_result",
            ErrorCode::EntryOutOfOrder => "entry_out_of_order",
            ErrorCode::ClosedTurn => "closed_turn",
            ErrorCode::UnresolvedExecution => "unresolved_execution",
            ErrorCode::InvalidRecordStructure => "invalid_record_structure",
            ErrorCode::MalformedJson => "malformed_json",
            ErrorCode::SchemaMismatch => "schema_mismatch",
            ErrorCode::VariantMismatch => "variant_mismatch",
            ErrorCode::StreamTargetError => "stream_target_error",
            ErrorCode::StreamPhaseError => "stream_phase_error",
            ErrorCode::InconsistentFinalization => "inconsistent_finalization",
        }
    }

    pub fn parse(value: &str) -> Option<ErrorCode> {
        ErrorCode::ALL
            .iter()
            .copied()
            .find(|code| code.as_str() == value)
    }
}

impl std::fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
#[path = "code_tests.rs"]
mod tests;
