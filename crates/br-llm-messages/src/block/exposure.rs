use serde::{Deserialize, Serialize};

use super::ids::IdSink;
use super::plain::PlainBlock;
use crate::error::{Error, ErrorCode};
use crate::value::ReplayMaterial;

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    try_from = "ResultExposureWire",
    tag = "type",
    rename_all = "snake_case"
)]
pub enum ResultExposure {
    #[non_exhaustive]
    Readable {
        content: Vec<PlainBlock>,
    },
    #[non_exhaustive]
    Opaque {
        material: ReplayMaterial,
    },
    #[non_exhaustive]
    Mixed {
        content: Vec<PlainBlock>,
        material: ReplayMaterial,
    },
    NotExposed,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
enum ResultExposureWire {
    Readable {
        content: Vec<PlainBlock>,
    },
    Opaque {
        material: ReplayMaterial,
    },
    Mixed {
        content: Vec<PlainBlock>,
        material: ReplayMaterial,
    },
    NotExposed {},
}

fn require_content(content: &[PlainBlock]) -> Result<(), Error> {
    if content.is_empty() {
        return Err(Error::new(ErrorCode::MissingContent).at("content"));
    }
    PlainBlock::collect_all(content, &mut IdSink::default()).map_err(|error| error.at("content"))
}

impl ResultExposure {
    pub fn readable(content: Vec<PlainBlock>) -> Result<Self, Error> {
        require_content(&content)?;
        Ok(ResultExposure::Readable { content })
    }

    pub fn opaque(material: ReplayMaterial) -> Self {
        ResultExposure::Opaque { material }
    }

    pub fn mixed(content: Vec<PlainBlock>, material: ReplayMaterial) -> Result<Self, Error> {
        require_content(&content)?;
        Ok(ResultExposure::Mixed { content, material })
    }

    pub fn not_exposed() -> Self {
        ResultExposure::NotExposed
    }

    pub fn content(&self) -> &[PlainBlock] {
        match self {
            ResultExposure::Readable { content } | ResultExposure::Mixed { content, .. } => content,
            _ => &[],
        }
    }

    pub fn material(&self) -> Option<&ReplayMaterial> {
        match self {
            ResultExposure::Opaque { material } | ResultExposure::Mixed { material, .. } => {
                Some(material)
            }
            _ => None,
        }
    }

    pub(crate) fn collect_ids(&self, sink: &mut IdSink) -> Result<(), Error> {
        PlainBlock::collect_all(self.content(), sink).map_err(|error| error.at("content"))
    }
}

impl TryFrom<ResultExposureWire> for ResultExposure {
    type Error = Error;

    fn try_from(wire: ResultExposureWire) -> Result<Self, Error> {
        match wire {
            ResultExposureWire::Readable { content } => ResultExposure::readable(content),
            ResultExposureWire::Opaque { material } => Ok(ResultExposure::opaque(material)),
            ResultExposureWire::Mixed { content, material } => {
                ResultExposure::mixed(content, material)
            }
            ResultExposureWire::NotExposed {} => Ok(ResultExposure::not_exposed()),
        }
    }
}
