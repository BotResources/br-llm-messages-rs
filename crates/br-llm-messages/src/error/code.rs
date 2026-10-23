#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorCode {
    InvalidIdentity,
    ReusedIdentity,
    InvalidAuthor,
    AuthorForbidden,
    MissingContent,
    InvalidContent,
    DisallowedEntryKind,
    DisallowedBlockKind,
    UnknownReference,
    WrongKindReference,
    DuplicateToolResult,
    ToolResultMismatch,
    TurnClosed,
    UnresolvedExecution,
    ContinuationPending,
    MalformedJson,
    SchemaMismatch,
    UnsupportedVariant,
    UnsupportedField,
    InvalidPosition,
    InvalidRecord,
    ReplayMismatch,
    StreamTarget,
    StreamPhase,
    InconsistentFinalization,
}

impl ErrorCode {
    pub const ALL: &'static [ErrorCode] = &[
        ErrorCode::InvalidIdentity,
        ErrorCode::ReusedIdentity,
        ErrorCode::InvalidAuthor,
        ErrorCode::AuthorForbidden,
        ErrorCode::MissingContent,
        ErrorCode::InvalidContent,
        ErrorCode::DisallowedEntryKind,
        ErrorCode::DisallowedBlockKind,
        ErrorCode::UnknownReference,
        ErrorCode::WrongKindReference,
        ErrorCode::DuplicateToolResult,
        ErrorCode::ToolResultMismatch,
        ErrorCode::TurnClosed,
        ErrorCode::UnresolvedExecution,
        ErrorCode::ContinuationPending,
        ErrorCode::MalformedJson,
        ErrorCode::SchemaMismatch,
        ErrorCode::UnsupportedVariant,
        ErrorCode::UnsupportedField,
        ErrorCode::InvalidPosition,
        ErrorCode::InvalidRecord,
        ErrorCode::ReplayMismatch,
        ErrorCode::StreamTarget,
        ErrorCode::StreamPhase,
        ErrorCode::InconsistentFinalization,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            ErrorCode::InvalidIdentity => "invalid_identity",
            ErrorCode::ReusedIdentity => "reused_identity",
            ErrorCode::InvalidAuthor => "invalid_author",
            ErrorCode::AuthorForbidden => "author_forbidden",
            ErrorCode::MissingContent => "missing_content",
            ErrorCode::InvalidContent => "invalid_content",
            ErrorCode::DisallowedEntryKind => "disallowed_entry_kind",
            ErrorCode::DisallowedBlockKind => "disallowed_block_kind",
            ErrorCode::UnknownReference => "unknown_reference",
            ErrorCode::WrongKindReference => "wrong_kind_reference",
            ErrorCode::DuplicateToolResult => "duplicate_tool_result",
            ErrorCode::ToolResultMismatch => "tool_result_mismatch",
            ErrorCode::TurnClosed => "turn_closed",
            ErrorCode::UnresolvedExecution => "unresolved_execution",
            ErrorCode::ContinuationPending => "continuation_pending",
            ErrorCode::MalformedJson => "malformed_json",
            ErrorCode::SchemaMismatch => "schema_mismatch",
            ErrorCode::UnsupportedVariant => "unsupported_variant",
            ErrorCode::UnsupportedField => "unsupported_field",
            ErrorCode::InvalidPosition => "invalid_position",
            ErrorCode::InvalidRecord => "invalid_record",
            ErrorCode::ReplayMismatch => "replay_mismatch",
            ErrorCode::StreamTarget => "stream_target",
            ErrorCode::StreamPhase => "stream_phase",
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
