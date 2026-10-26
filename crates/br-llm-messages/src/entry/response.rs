use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use super::{locate, refuse_author};
use crate::block::{ContentBlock, IdSink, RuntimeToolCall};
use crate::error::{Error, ErrorCode};
use crate::id::{BlockId, EntryId};
use crate::value::{ExternalId, ModelName, ObservedAt, ResponseEnding, Usage};

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResponseMetadata {
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::json::optional"
    )]
    usage: Option<Usage>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::json::optional"
    )]
    model: Option<ModelName>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::json::optional"
    )]
    provider_response_id: Option<ExternalId>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::json::optional"
    )]
    observed_at: Option<ObservedAt>,
}

impl ResponseMetadata {
    #[allow(clippy::new_without_default)] // a Default derive would freeze the field set
    pub fn new() -> Self {
        ResponseMetadata {
            usage: None,
            model: None,
            provider_response_id: None,
            observed_at: None,
        }
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

    pub fn with_observed_at(mut self, observed_at: ObservedAt) -> Self {
        self.observed_at = Some(observed_at);
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

    pub fn observed_at(&self) -> Option<&ObservedAt> {
        self.observed_at.as_ref()
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    #[serde(skip_serializing_if = "Option::is_none")]
    observed_at: Option<ObservedAt>,
    #[serde(rename = "content")]
    blocks: Vec<ContentBlock>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ModelResponseWire {
    id: EntryId,
    ending: ResponseEnding,
    #[serde(default, deserialize_with = "crate::json::optional")]
    usage: Option<Usage>,
    #[serde(default, deserialize_with = "crate::json::optional")]
    model: Option<ModelName>,
    #[serde(default, deserialize_with = "crate::json::optional")]
    provider_response_id: Option<ExternalId>,
    content: Vec<ContentBlock>,
    #[serde(default, deserialize_with = "super::present")]
    author: bool,
    #[serde(default, deserialize_with = "crate::json::optional")]
    observed_at: Option<ObservedAt>,
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
            observed_at: metadata.observed_at,
            blocks,
        };
        response.validate().map_err(|error| locate(error, id))?;
        Ok(response)
    }

    pub fn with_observed_at(mut self, observed_at: ObservedAt) -> Self {
        self.observed_at = Some(observed_at);
        self
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

    pub fn observed_at(&self) -> Option<&ObservedAt> {
        self.observed_at.as_ref()
    }

    pub fn metadata(&self) -> ResponseMetadata {
        ResponseMetadata {
            usage: self.usage.clone(),
            model: self.model.clone(),
            provider_response_id: self.provider_response_id.clone(),
            observed_at: self.observed_at.clone(),
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
                return Err(Error::new(ErrorCode::DisallowedKind)
                    .with_id(block.id().as_uuid())
                    .at(format!("content[{index}]")));
            }
        }
        self.collect_ids(&mut IdSink::default())?;
        self.check_provider_links()?;
        if self.ending == ResponseEnding::tool_use() && self.runtime_calls().next().is_none() {
            return Err(Error::new(ErrorCode::InvalidContent).at("content"));
        }
        Ok(())
    }
}

impl ModelResponse {
    fn check_provider_links(&self) -> Result<(), Error> {
        let here: HashSet<BlockId> = self
            .blocks
            .iter()
            .filter_map(|block| match block {
                ContentBlock::ProviderToolCall(call) => Some(call.id()),
                _ => None,
            })
            .collect();
        let mut preceding = HashSet::new();
        let mut answered = HashSet::new();
        for (index, block) in self.blocks.iter().enumerate() {
            match block {
                ContentBlock::ProviderToolCall(call) => {
                    preceding.insert(call.id());
                }
                ContentBlock::ProviderToolResult(result) => {
                    let target = result.call();
                    let located = |code| {
                        Error::new(code)
                            .with_id(target.as_uuid())
                            .at(format!("content[{index}].call"))
                    };
                    if here.contains(&target) && !preceding.contains(&target) {
                        return Err(located(ErrorCode::UnknownReference));
                    }
                    if here.contains(&target) && !answered.insert(target) {
                        return Err(located(ErrorCode::DuplicateResult));
                    }
                }
                _ => {}
            }
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
                observed_at: wire.observed_at,
            },
        )
    }
}
