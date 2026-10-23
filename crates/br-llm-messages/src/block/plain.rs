use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use uuid::Uuid;

use super::ContentBlock;
use super::ContentBlockKind;
use super::ids::IdSink;
use super::image::ImageBlock;
use super::structured::StructuredBlock;
use super::text::TextBlock;
use crate::error::{Error, ErrorCode};
use crate::id::BlockId;

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PlainBlock {
    Text(TextBlock),
    Image(ImageBlock),
    StructuredContent(StructuredBlock),
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum PlainBlockWire {
    Text(TextBlock),
    Image(ImageBlock),
    StructuredContent(StructuredBlock),
}

impl PlainBlock {
    pub fn kind(&self) -> ContentBlockKind {
        match self {
            PlainBlock::Text(_) => ContentBlockKind::Text,
            PlainBlock::Image(_) => ContentBlockKind::Image,
            PlainBlock::StructuredContent(_) => ContentBlockKind::StructuredContent,
        }
    }

    pub fn id(&self) -> BlockId {
        match self {
            PlainBlock::Text(block) => block.id(),
            PlainBlock::Image(block) => block.id(),
            PlainBlock::StructuredContent(block) => block.id(),
        }
    }

    pub fn ensure_distinct_ids(blocks: &[PlainBlock]) -> Result<(), Error> {
        PlainBlock::collect_all(blocks, &mut IdSink::default())
    }

    pub fn to_json(&self) -> Result<String, Error> {
        crate::json::encode(self)
    }

    pub fn from_json(text: &str) -> Result<Self, Error> {
        crate::json::decode(text)
    }

    pub fn to_value(&self) -> Result<Value, Error> {
        crate::json::encode_value(self)
    }

    pub fn from_value(value: Value) -> Result<Self, Error> {
        crate::json::decode_value(value)
    }

    pub(crate) fn collect_ids(&self, sink: &mut IdSink) -> Result<(), Error> {
        match self {
            PlainBlock::Text(block) => block.collect_ids(sink),
            PlainBlock::Image(block) => block.collect_ids(sink),
            PlainBlock::StructuredContent(block) => block.collect_ids(sink),
        }
    }

    pub(crate) fn collect_all(blocks: &[PlainBlock], sink: &mut IdSink) -> Result<(), Error> {
        for (index, block) in blocks.iter().enumerate() {
            block
                .collect_ids(sink)
                .map_err(|error| error.at(format!("[{index}]")))?;
        }
        Ok(())
    }
}

impl TryFrom<TextBlock> for PlainBlock {
    type Error = Error;

    fn try_from(block: TextBlock) -> Result<Self, Error> {
        block.refuse_replay()?;
        Ok(PlainBlock::Text(block))
    }
}

impl From<ImageBlock> for PlainBlock {
    fn from(block: ImageBlock) -> Self {
        PlainBlock::Image(block)
    }
}

impl From<StructuredBlock> for PlainBlock {
    fn from(block: StructuredBlock) -> Self {
        PlainBlock::StructuredContent(block)
    }
}

impl TryFrom<ContentBlock> for PlainBlock {
    type Error = Error;

    fn try_from(block: ContentBlock) -> Result<Self, Error> {
        match block {
            ContentBlock::Text(text) => PlainBlock::try_from(text),
            ContentBlock::Image(image) => Ok(PlainBlock::Image(image)),
            ContentBlock::StructuredContent(structured) => {
                Ok(PlainBlock::StructuredContent(structured))
            }
            other => Err(Error::new(ErrorCode::DisallowedBlockKind).with_id(other.id().as_uuid())),
        }
    }
}

impl TryFrom<PlainBlockWire> for PlainBlock {
    type Error = Error;

    fn try_from(wire: PlainBlockWire) -> Result<Self, Error> {
        match wire {
            PlainBlockWire::Text(text) => PlainBlock::try_from(text),
            PlainBlockWire::Image(image) => Ok(PlainBlock::Image(image)),
            PlainBlockWire::StructuredContent(structured) => {
                Ok(PlainBlock::StructuredContent(structured))
            }
        }
    }
}

fn refuse_non_plain(value: &Value) -> Result<(), Error> {
    let Some(kind) = value
        .get("type")
        .and_then(Value::as_str)
        .and_then(|name| name.parse::<ContentBlockKind>().ok())
    else {
        return Ok(());
    };
    if kind.is_plain() {
        return Ok(());
    }
    let mut error = Error::new(ErrorCode::DisallowedBlockKind);
    if let Some(id) = value
        .get("id")
        .and_then(Value::as_str)
        .and_then(|text| Uuid::try_parse(text).ok())
    {
        error = error.with_id(id);
    }
    Err(error)
}

impl<'de> Deserialize<'de> for PlainBlock {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = Value::deserialize(deserializer)?;
        refuse_non_plain(&value).map_err(serde::de::Error::custom)?;
        let wire = PlainBlockWire::deserialize(value)
            .map_err(|error| serde::de::Error::custom(Error::from_json_error(&error)))?;
        PlainBlock::try_from(wire).map_err(serde::de::Error::custom)
    }
}
