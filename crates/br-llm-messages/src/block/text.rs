use serde::{Deserialize, Serialize};

use super::ids::IdSink;
use crate::error::{Error, ErrorCode};
use crate::id::BlockId;
use crate::value::{Citation, PlainText, ReplayMaterial};

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "TextBlockWire")]
pub struct TextBlock {
    id: BlockId,
    text: PlainText,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    citations: Vec<Citation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    replay: Option<ReplayMaterial>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TextBlockWire {
    id: BlockId,
    text: PlainText,
    #[serde(default)]
    citations: Vec<Citation>,
    replay: Option<ReplayMaterial>,
}

impl TextBlock {
    pub fn new(
        id: BlockId,
        text: impl Into<String>,
        citations: Vec<Citation>,
        replay: Option<ReplayMaterial>,
    ) -> Result<Self, Error> {
        let located = |error: Error| error.with_id(id.as_uuid());
        let text = PlainText::new(text).map_err(|error| located(error).at("text"))?;
        Citation::check_all(&text, &citations).map_err(located)?;
        let block = TextBlock {
            id,
            text,
            citations,
            replay,
        };
        block.collect_ids(&mut IdSink::default())?;
        Ok(block)
    }

    pub fn plain(id: BlockId, text: impl Into<String>) -> Result<Self, Error> {
        TextBlock::new(id, text, Vec::new(), None)
    }

    pub fn id(&self) -> BlockId {
        self.id
    }

    pub fn text(&self) -> &PlainText {
        &self.text
    }

    pub fn citations(&self) -> &[Citation] {
        &self.citations
    }

    pub fn replay(&self) -> Option<&ReplayMaterial> {
        self.replay.as_ref()
    }

    pub(crate) fn collect_ids(&self, sink: &mut IdSink) -> Result<(), Error> {
        sink.insert(self.id.as_uuid())?;
        for (index, citation) in self.citations.iter().enumerate() {
            sink.insert(citation.id().as_uuid())
                .map_err(|error| error.at(format!("citations[{index}]")))?;
        }
        Ok(())
    }

    pub(crate) fn without_replay(&self) -> TextBlock {
        TextBlock {
            id: self.id,
            text: self.text.clone(),
            citations: self.citations.clone(),
            replay: None,
        }
    }

    pub(crate) fn refuse_replay(&self) -> Result<(), Error> {
        match self.replay {
            Some(_) => Err(Error::new(ErrorCode::InvalidContent)
                .with_id(self.id.as_uuid())
                .at("replay")),
            None => Ok(()),
        }
    }
}

impl TryFrom<TextBlockWire> for TextBlock {
    type Error = Error;

    fn try_from(wire: TextBlockWire) -> Result<Self, Error> {
        TextBlock::new(
            wire.id,
            wire.text.into_string(),
            wire.citations,
            wire.replay,
        )
    }
}
