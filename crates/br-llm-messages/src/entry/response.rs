use serde::{Deserialize, Serialize};

use super::{locate, refuse_author};
use crate::block::{ContentBlock, IdSink, RuntimeToolCall};
use crate::error::{Error, ErrorCode};
use crate::id::EntryId;
use crate::value::{ExternalId, ModelName, ResponseEnding, Usage};

#[non_exhaustive]
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResponseMetadata {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    usage: Option<Usage>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    model: Option<ModelName>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    provider_response_id: Option<ExternalId>,
}

impl ResponseMetadata {
    pub fn new() -> Self {
        ResponseMetadata::default()
    }

    pub fn with_usage(mut self, usage: Usage) -> Self {
        self.usage = Some(usage);
        self
    }

    pub fn with_model(mut self, model: ModelName) -> Self {
        self.model = Some(model);
        self
    }

    pub fn with_provider_response_id(mut self, id: ExternalId) -> Self {
        self.provider_response_id = Some(id);
        self
    }

    pub fn usage(&self) -> Option<&Usage> {
        self.usage.as_ref()
    }

    pub fn model(&self) -> Option<&ModelName> {
        self.model.as_ref()
    }

    pub fn provider_response_id(&self) -> Option<&ExternalId> {
        self.provider_response_id.as_ref()
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "ModelResponseWire")]
pub struct ModelResponse {
    id: EntryId,
    ending: ResponseEnding,
    #[serde(skip_serializing_if = "Option::is_none")]
    usage: Option<Usage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    model: Option<ModelName>,
    #[serde(skip_serializing_if = "Option::is_none")]
    provider_response_id: Option<ExternalId>,
    #[serde(rename = "content")]
    blocks: Vec<ContentBlock>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ModelResponseWire {
    id: EntryId,
    ending: ResponseEnding,
    usage: Option<Usage>,
    model: Option<ModelName>,
    provider_response_id: Option<ExternalId>,
    content: Vec<ContentBlock>,
    #[serde(default, deserialize_with = "super::present")]
    author: bool,
}

impl ModelResponse {
    pub fn new(
        id: EntryId,
        blocks: Vec<ContentBlock>,
        ending: ResponseEnding,
        metadata: ResponseMetadata,
    ) -> Result<Self, Error> {
        let response = ModelResponse {
            id,
            ending,
            usage: metadata.usage,
            model: metadata.model,
            provider_response_id: metadata.provider_response_id,
            blocks,
        };
        response.validate().map_err(|error| locate(error, id))?;
        Ok(response)
    }

    pub fn id(&self) -> EntryId {
        self.id
    }

    pub fn blocks(&self) -> &[ContentBlock] {
        &self.blocks
    }

    pub fn ending(&self) -> &ResponseEnding {
        &self.ending
    }

    pub fn usage(&self) -> Option<&Usage> {
        self.usage.as_ref()
    }

    pub fn model(&self) -> Option<&ModelName> {
        self.model.as_ref()
    }

    pub fn provider_response_id(&self) -> Option<&ExternalId> {
        self.provider_response_id.as_ref()
    }

    pub fn metadata(&self) -> ResponseMetadata {
        ResponseMetadata {
            usage: self.usage,
            model: self.model.clone(),
            provider_response_id: self.provider_response_id.clone(),
        }
    }

    pub fn runtime_calls(&self) -> impl Iterator<Item = &RuntimeToolCall> {
        self.blocks.iter().filter_map(|block| match block {
            ContentBlock::RuntimeToolCall(call) => Some(call),
            _ => None,
        })
    }

    pub(crate) fn collect_ids(&self, sink: &mut IdSink) -> Result<(), Error> {
        sink.insert(self.id.as_uuid())?;
        for (index, block) in self.blocks.iter().enumerate() {
            block
                .collect_ids(sink)
                .map_err(|error| error.at(format!("content[{index}]")))?;
        }
        Ok(())
    }

    fn validate(&self) -> Result<(), Error> {
        for (index, block) in self.blocks.iter().enumerate() {
            if matches!(block, ContentBlock::RuntimeToolResult(_)) {
                return Err(Error::new(ErrorCode::DisallowedBlockKind)
                    .with_id(block.id().as_uuid())
                    .at(format!("content[{index}]")));
            }
        }
        self.collect_ids(&mut IdSink::default())?;
        if self.ending == ResponseEnding::ToolResultsRequested
            && self.runtime_calls().next().is_none()
        {
            return Err(Error::new(ErrorCode::MissingContent).at("content"));
        }
        Ok(())
    }
}

impl TryFrom<ModelResponseWire> for ModelResponse {
    type Error = Error;

    fn try_from(wire: ModelResponseWire) -> Result<Self, Error> {
        refuse_author(wire.author, wire.id)?;
        ModelResponse::new(
            wire.id,
            wire.content,
            wire.ending,
            ResponseMetadata {
                usage: wire.usage,
                model: wire.model,
                provider_response_id: wire.provider_response_id,
            },
        )
    }
}
