use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::error::{Error, ErrorCode};

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResponseDeltaKind {
    ResponseStarted,
    BlockStarted,
    TextAppended,
    ThinkingAppended,
    JsonAppended,
    ReplayFragmentAppended,
    ReplayMaterialSet,
    BlockMetadataUpdated,
    BlockInserted,
    BlockFinalized,
    BlockDiscarded,
    ResponseMetadataUpdated,
    ResponseCompleted,
    ResponseInterrupted,
}

impl ResponseDeltaKind {
    pub const ALL: &'static [ResponseDeltaKind] = &[
        ResponseDeltaKind::ResponseStarted,
        ResponseDeltaKind::BlockStarted,
        ResponseDeltaKind::TextAppended,
        ResponseDeltaKind::ThinkingAppended,
        ResponseDeltaKind::JsonAppended,
        ResponseDeltaKind::ReplayFragmentAppended,
        ResponseDeltaKind::ReplayMaterialSet,
        ResponseDeltaKind::BlockMetadataUpdated,
        ResponseDeltaKind::BlockInserted,
        ResponseDeltaKind::BlockFinalized,
        ResponseDeltaKind::BlockDiscarded,
        ResponseDeltaKind::ResponseMetadataUpdated,
        ResponseDeltaKind::ResponseCompleted,
        ResponseDeltaKind::ResponseInterrupted,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            ResponseDeltaKind::ResponseStarted => "response_started",
            ResponseDeltaKind::BlockStarted => "block_started",
            ResponseDeltaKind::TextAppended => "text_appended",
            ResponseDeltaKind::ThinkingAppended => "thinking_appended",
            ResponseDeltaKind::JsonAppended => "json_appended",
            ResponseDeltaKind::ReplayFragmentAppended => "replay_fragment_appended",
            ResponseDeltaKind::ReplayMaterialSet => "replay_material_set",
            ResponseDeltaKind::BlockMetadataUpdated => "block_metadata_updated",
            ResponseDeltaKind::BlockInserted => "block_inserted",
            ResponseDeltaKind::BlockFinalized => "block_finalized",
            ResponseDeltaKind::BlockDiscarded => "block_discarded",
            ResponseDeltaKind::ResponseMetadataUpdated => "response_metadata_updated",
            ResponseDeltaKind::ResponseCompleted => "response_completed",
            ResponseDeltaKind::ResponseInterrupted => "response_interrupted",
        }
    }

    pub fn target(self) -> DeltaTarget {
        match self {
            ResponseDeltaKind::ResponseStarted
            | ResponseDeltaKind::ResponseMetadataUpdated
            | ResponseDeltaKind::ResponseCompleted
            | ResponseDeltaKind::ResponseInterrupted => DeltaTarget::Response,
            _ => DeltaTarget::Block,
        }
    }
}

impl FromStr for ResponseDeltaKind {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self, Error> {
        ResponseDeltaKind::ALL
            .iter()
            .copied()
            .find(|kind| kind.as_str() == value)
            .ok_or_else(|| Error::new(ErrorCode::UnsupportedVariant))
    }
}

impl fmt::Display for ResponseDeltaKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeltaTarget {
    Response,
    Block,
}
