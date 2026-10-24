use serde::{Deserialize, Serialize};

use super::ids::IdSink;
use crate::error::{Error, ErrorCode};
use crate::id::BlockId;
use crate::value::{Citation, ItemRef, ObservedAt, PlainText, ReplayMaterial};

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "TextBlockWire")]
pub struct TextBlock {
    id: BlockId,
    text: PlainText,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    citations: Vec<Citation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    item_ref: Option<ItemRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    replay: Option<ReplayMaterial>,
    #[serde(skip_serializing_if = "Option::is_none")]
    observed_at: Option<ObservedAt>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TextBlockWire {
    id: BlockId,
    text: PlainText,
    #[serde(default)]
    citations: Vec<Citation>,
    #[serde(default, deserialize_with = "crate::json::optional")]
    item_ref: Option<ItemRef>,
    #[serde(default, deserialize_with = "crate::json::optional")]
    replay: Option<ReplayMaterial>,
    #[serde(default, deserialize_with = "crate::json::optional")]
    observed_at: Option<ObservedAt>,
}

impl TextBlock {
    pub fn new(id: BlockId, text: impl Into<String>) -> Result<Self, Error> {
        let text = PlainText::new(text).map_err(|error| error.with_id(id.as_uuid()).at("text"))?;
        Ok(TextBlock {
            id,
            text,
            citations: Vec::new(),
            item_ref: None,
            replay: None,
            observed_at: None,
        })
    }

    pub fn with_citations(mut self, citations: Vec<Citation>) -> Result<Self, Error> {
        Citation::check_all(&self.text, &citations)
            .map_err(|error| error.with_id(self.id.as_uuid()))?;
        self.citations = citations;
        self.collect_ids(&mut IdSink::default())?;
        Ok(self)
    }

    pub fn with_item_ref(mut self, item_ref: ItemRef) -> Self {
        self.item_ref = Some(item_ref);
        self
    }

    pub fn with_replay(mut self, replay: ReplayMaterial) -> Self {
        self.replay = Some(replay);
        self
    }

    pub fn with_observed_at(mut self, observed_at: ObservedAt) -> Self {
        self.observed_at = Some(observed_at);
        self
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

    pub fn item_ref(&self) -> Option<&ItemRef> {
        self.item_ref.as_ref()
    }

    pub fn replay(&self) -> Option<&ReplayMaterial> {
        self.replay.as_ref()
    }

    pub fn observed_at(&self) -> Option<&ObservedAt> {
        self.observed_at.as_ref()
    }

    pub(crate) fn collect_ids(&self, sink: &mut IdSink) -> Result<(), Error> {
        sink.insert(self.id.as_uuid())?;
        for (index, citation) in self.citations.iter().enumerate() {
            sink.insert(citation.id().as_uuid())
                .map_err(|error| error.at(format!("citations[{index}]")))?;
        }
        Ok(())
    }

    pub(crate) fn text_only(&self) -> TextBlock {
        TextBlock {
            id: self.id,
            text: self.text.clone(),
            citations: Vec::new(),
            item_ref: None,
            replay: None,
            observed_at: None,
        }
    }

    pub(crate) fn refuse_model_fields(&self) -> Result<(), Error> {
        let refuse = |location: &str| {
            Err(Error::new(ErrorCode::DisallowedKind)
                .with_id(self.id.as_uuid())
                .at(location))
        };
        if self.replay.is_some() {
            return refuse("replay");
        }
        if self.item_ref.is_some() {
            return refuse("item_ref");
        }
        Ok(())
    }
}

impl TryFrom<TextBlockWire> for TextBlock {
    type Error = Error;

    fn try_from(wire: TextBlockWire) -> Result<Self, Error> {
        let mut block =
            TextBlock::new(wire.id, wire.text.into_string())?.with_citations(wire.citations)?;
        block.item_ref = wire.item_ref;
        block.replay = wire.replay;
        block.observed_at = wire.observed_at;
        Ok(block)
    }
}
