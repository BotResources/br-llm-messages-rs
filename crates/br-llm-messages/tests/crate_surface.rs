mod common;

use std::fmt::Debug;
use std::fs;
use std::path::{Path, PathBuf};

use br_llm_messages::{
    ActorRef, BlockRecord, BlockStart, CONVERSATION_SCHEMA, ContentBlockKind, DELTA_SCHEMA,
    DRAFT_SCHEMA, DraftSnapshot, EntryRecord, Error, ErrorCode, OtherItem, OtherTurn, Perspective,
    ProjectedTurn, Projection, PublicResponse, RECORD_SCHEMA, RecordSet, RepresentationState,
    ResponseDelta, ResponseDeltaKind, ResponseDraft, TurnEntryKind, TurnKind, TurnRecord,
};

const MAX_LINES: usize = 300;
const GROWTH_FAMILIES: &[&str] = &[
    "TurnEntry",
    "IncomingEntry",
    "ContentBlock",
    "PlainBlock",
    "ResponseEnding",
    "ClosureReason",
    "ExecutionStatus",
    "ResultExposure",
    "ReplayPayload",
    "ImageSource",
    "DeltaOp",
    "BlockMetadataUpdate",
    "BlockStart",
    "Finalization",
    "InterruptionReason",
    "ErrorCode",
    "Error",
    "TurnKind",
    "TurnEntryKind",
    "ContentBlockKind",
    "ResponseDeltaKind",
    "RepresentationState",
    "ActorRef",
    "TurnRecord",
    "EntryRecord",
    "BlockRecord",
    "RecordSet",
    "DraftSnapshot",
    "Usage",
    "ResponseMetadata",
    "MediaType",
    "Turn",
    "ProjectedTurn",
    "OtherItem",
    "OtherTurn",
    "PublicResponse",
    "Projection",
    "Perspective",
];

fn assert_value_traits<T: Clone + PartialEq + Debug + Send + Sync>() {}

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn rust_files(dir: &Path, found: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            rust_files(&path, found);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            found.push(path);
        }
    }
}

fn all_sources() -> Vec<PathBuf> {
    let mut found = Vec::new();
    rust_files(&root().join("src"), &mut found);
    rust_files(&root().join("tests"), &mut found);
    found
}

fn comment_marker() -> String {
    "/".repeat(2)
}

fn block_marker() -> String {
    format!("/{}", "*")
}

fn has_comment(line: &str) -> bool {
    let marker = comment_marker();
    if line.trim_start().starts_with(&marker) || line.contains(&block_marker()) {
        return true;
    }
    match line.find(&format!(" {marker}")) {
        Some(index) => line[..index].matches('"').count() & 1 == 0,
        None => false,
    }
}

fn is_sanctioned(line: &str) -> bool {
    line.contains(&format!("{} SAFETY:", comment_marker())) || line.contains("[allow(")
}

#[test]
fn given_the_value_types_when_checked_statically_then_snapshots_records_and_projections_are_clone_eq_debug_send_sync()
 {
    fn assert_all<T: Clone + PartialEq + Debug + Send + Sync>() {}
    assert_all::<DraftSnapshot>();
    assert_all::<RecordSet>();
    assert_all::<TurnRecord>();
    assert_all::<EntryRecord>();
    assert_all::<BlockRecord>();
    assert_all::<ActorRef>();
    assert_all::<RepresentationState>();
    assert_all::<Projection>();
    assert_all::<ProjectedTurn>();
    assert_all::<OtherTurn>();
    assert_all::<OtherItem>();
    assert_all::<PublicResponse>();
    assert_all::<Perspective>();
    assert_all::<ResponseDraft>();
    assert_all::<ResponseDelta>();
    assert_all::<BlockStart>();
    assert_value_traits::<Error>();
}

#[test]
fn given_the_public_kind_enums_when_listed_then_each_is_reachable_from_the_crate_root() {
    assert_eq!(TurnKind::ALL.len(), 2);
    assert_eq!(TurnEntryKind::ALL.len(), 11);
    assert_eq!(ContentBlockKind::ALL.len(), 10);
    assert_eq!(ResponseDeltaKind::ALL.len(), 14);
    assert_eq!(RepresentationState::ALL.len(), 4);
    assert!(ErrorCode::ALL.contains(&ErrorCode::InvalidRecordStructure));
}

fn attributes_above(lines: &[&str], declaration: usize) -> String {
    let mut collected = Vec::new();
    for line in lines[..declaration].iter().rev() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.ends_with('}') || trimmed.ends_with(';') {
            break;
        }
        collected.push(trimmed);
    }
    collected.join(" ")
}

#[test]
fn given_the_growth_families_when_the_sources_are_scanned_then_each_is_non_exhaustive() {
    let mut missing = Vec::new();
    let mut seen = Vec::new();
    for path in all_sources()
        .iter()
        .filter(|p| p.starts_with(root().join("src")))
    {
        let text = fs::read_to_string(path).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        for (index, line) in lines.iter().enumerate() {
            for family in GROWTH_FAMILIES {
                let enum_decl = format!("pub enum {family} ");
                let struct_decl = format!("pub struct {family} ");
                let struct_open = format!("pub struct {family}(");
                let declared = line.starts_with(&enum_decl)
                    || line.starts_with(&struct_decl)
                    || line.starts_with(&struct_open);
                if declared {
                    seen.push(*family);
                    if !attributes_above(&lines, index).contains("non_exhaustive") {
                        missing.push(format!("{family} in {}", path.display()));
                    }
                }
            }
        }
    }
    let unseen: Vec<_> = GROWTH_FAMILIES
        .iter()
        .filter(|f| !seen.contains(f))
        .collect();
    assert!(unseen.is_empty(), "families not found: {unseen:?}");
    assert!(missing.is_empty(), "missing non_exhaustive: {missing:?}");
}

#[test]
fn given_the_sources_when_scanned_then_no_comment_exists_outside_the_sanctioned_forms() {
    let mut offenders = Vec::new();
    for path in all_sources() {
        let text = fs::read_to_string(&path).unwrap();
        for (index, line) in text.lines().enumerate() {
            if has_comment(line) && !is_sanctioned(line) {
                offenders.push(format!("{}:{}", path.display(), index + 1));
            }
        }
    }
    assert!(offenders.is_empty(), "comments found: {offenders:?}");
}

#[test]
fn given_the_sources_when_measured_then_no_file_exceeds_the_line_limit() {
    let oversized: Vec<_> = all_sources()
        .into_iter()
        .filter(|path| fs::read_to_string(path).unwrap().lines().count() > MAX_LINES)
        .collect();
    assert!(oversized.is_empty(), "oversized files: {oversized:?}");
}

#[test]
fn given_the_manifest_when_read_then_only_serde_serde_json_and_uuid_are_direct_dependencies() {
    let manifest = fs::read_to_string(root().join("Cargo.toml")).unwrap();
    let mut in_section = false;
    let mut names = Vec::new();
    for line in manifest.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_section = trimmed == "[dependencies]";
        } else if in_section && !trimmed.is_empty() {
            names.push(trimmed.split('=').next().unwrap().trim().to_owned());
        }
    }
    names.sort();
    assert_eq!(names, ["serde", "serde_json", "uuid"]);
}

#[test]
fn given_the_sources_when_scanned_then_no_thread_or_sql_type_exists() {
    for path in all_sources()
        .iter()
        .filter(|p| p.starts_with(root().join("src")))
    {
        let text = fs::read_to_string(path).unwrap();
        for forbidden in [
            "pub struct Thread",
            "pub enum ThreadKind",
            "sqlx",
            "CREATE TABLE",
        ] {
            assert!(
                !text.contains(forbidden),
                "{forbidden} in {}",
                path.display()
            );
        }
    }
}

#[test]
fn given_the_four_schema_constants_when_documents_are_serialized_then_each_names_its_constant() {
    let conversation = serde_json::to_value(common::conversations::rich()).unwrap();
    let records = serde_json::to_value(common::records::rich_set()).unwrap();

    assert_eq!(conversation["schema"], CONVERSATION_SCHEMA);
    assert_eq!(records["schema"], RECORD_SCHEMA);
    assert_eq!(DELTA_SCHEMA, "br-llm-messages-delta/1");
    assert_eq!(DRAFT_SCHEMA, "br-llm-messages-draft/1");
}
