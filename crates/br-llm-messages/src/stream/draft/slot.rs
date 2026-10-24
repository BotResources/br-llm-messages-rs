use uuid::Uuid;

use super::raw_json::RawJson;
use crate::block::{ContentBlock, ContentBlockKind, PlainBlock};
use crate::id::BlockId;
use crate::opaque_json::OpaqueJson;
use crate::stream::block_start::BlockStart;
use crate::value::{
    Citation, ExecutionStatus, ExternalId, ItemRef, ObservedAt, ProviderOperation, ReplayMaterial,
    ToolName,
};

#[derive(Debug, Clone, PartialEq)]
pub(in crate::stream) enum Slot {
    Open(OpenBlock),
    Final(ContentBlock),
}

#[derive(Debug, Clone, PartialEq)]
pub(in crate::stream) struct OpenBlock {
    pub(in crate::stream) id: BlockId,
    pub(in crate::stream) body: OpenBody,
    pub(in crate::stream) observed_at: Option<ObservedAt>,
}

#[derive(Debug, Clone, PartialEq)]
pub(in crate::stream) enum OpenBody {
    Text {
        text: String,
        citations: Vec<Citation>,
        item_ref: Option<ItemRef>,
        replay: Option<ReplayMaterial>,
    },
    Thinking {
        text: String,
        replay: Option<ReplayMaterial>,
    },
    RedactedThinking {
        replay: Option<ReplayMaterial>,
    },
    Structured {
        raw: RawJson,
    },
    RuntimeCall {
        name: Option<ToolName>,
        external_id: Option<ExternalId>,
        item_ref: Option<ItemRef>,
        provisional: Option<OpaqueJson>,
        raw: RawJson,
        replay: Option<ReplayMaterial>,
    },
    ProviderCall {
        operation: Option<ProviderOperation>,
        external_id: Option<ExternalId>,
        item_ref: Option<ItemRef>,
        provisional: Option<OpaqueJson>,
        raw: RawJson,
        replay: Option<ReplayMaterial>,
    },
    ProviderResult {
        call: BlockId,
        external_id: Option<ExternalId>,
        replay: Option<ReplayMaterial>,
    },
    Record {
        operation: Option<ProviderOperation>,
        external_id: Option<ExternalId>,
        status: Option<ExecutionStatus>,
        replay: Option<ReplayMaterial>,
    },
}

impl Slot {
    pub(in crate::stream) fn id(&self) -> BlockId {
        match self {
            Slot::Open(open) => open.id,
            Slot::Final(block) => block.id(),
        }
    }

    pub(in crate::stream) fn kind(&self) -> ContentBlockKind {
        match self {
            Slot::Open(open) => open.body.kind(),
            Slot::Final(block) => block.kind(),
        }
    }

    pub(in crate::stream) fn ids(&self) -> Vec<Uuid> {
        match self {
            Slot::Open(open) => open.ids(),
            Slot::Final(block) => block_ids(block),
        }
    }
}

impl OpenBlock {
    pub(in crate::stream) fn started(id: BlockId, start: &BlockStart) -> Self {
        OpenBlock {
            id,
            body: OpenBody::from_start(start),
            observed_at: start.observed_at().cloned(),
        }
    }

    pub(in crate::stream) fn ids(&self) -> Vec<Uuid> {
        let mut ids = vec![self.id.as_uuid()];
        if let OpenBody::Text { citations, .. } = &self.body {
            ids.extend(citations.iter().map(|citation| citation.id().as_uuid()));
        }
        ids
    }
}

impl OpenBody {
    fn from_start(start: &BlockStart) -> Self {
        match start {
            BlockStart::Text { item_ref, .. } => OpenBody::Text {
                text: String::new(),
                citations: Vec::new(),
                item_ref: item_ref.clone(),
                replay: None,
            },
            BlockStart::Thinking { .. } => OpenBody::Thinking {
                text: String::new(),
                replay: None,
            },
            BlockStart::RedactedThinking { .. } => OpenBody::RedactedThinking { replay: None },
            BlockStart::StructuredContent { .. } => OpenBody::Structured {
                raw: RawJson::default(),
            },
            BlockStart::RuntimeToolCall {
                name,
                external_id,
                item_ref,
                arguments,
                ..
            } => OpenBody::RuntimeCall {
                name: name.clone(),
                external_id: external_id.clone(),
                item_ref: item_ref.clone(),
                provisional: arguments.clone(),
                raw: RawJson::default(),
                replay: None,
            },
            BlockStart::ProviderToolCall {
                operation,
                external_id,
                item_ref,
                arguments,
                ..
            } => OpenBody::ProviderCall {
                operation: operation.clone(),
                external_id: external_id.clone(),
                item_ref: item_ref.clone(),
                provisional: arguments.clone(),
                raw: RawJson::default(),
                replay: None,
            },
            BlockStart::ProviderToolResult {
                call, external_id, ..
            } => OpenBody::ProviderResult {
                call: *call,
                external_id: external_id.clone(),
                replay: None,
            },
            BlockStart::ProviderToolExecutionRecord {
                operation,
                external_id,
                status,
                ..
            } => OpenBody::Record {
                operation: operation.clone(),
                external_id: external_id.clone(),
                status: status.clone(),
                replay: None,
            },
        }
    }

    pub(in crate::stream) fn kind(&self) -> ContentBlockKind {
        match self {
            OpenBody::Text { .. } => ContentBlockKind::Text,
            OpenBody::Thinking { .. } => ContentBlockKind::Thinking,
            OpenBody::RedactedThinking { .. } => ContentBlockKind::RedactedThinking,
            OpenBody::Structured { .. } => ContentBlockKind::StructuredContent,
            OpenBody::RuntimeCall { .. } => ContentBlockKind::RuntimeToolCall,
            OpenBody::ProviderCall { .. } => ContentBlockKind::ProviderToolCall,
            OpenBody::ProviderResult { .. } => ContentBlockKind::ProviderToolResult,
            OpenBody::Record { .. } => ContentBlockKind::ProviderToolExecutionRecord,
        }
    }

    pub(in crate::stream) fn raw_json_mut(&mut self) -> Option<&mut RawJson> {
        match self {
            OpenBody::Structured { raw }
            | OpenBody::RuntimeCall { raw, .. }
            | OpenBody::ProviderCall { raw, .. } => Some(raw),
            _ => None,
        }
    }

    pub(in crate::stream) fn replay(&self) -> Option<&ReplayMaterial> {
        match self {
            OpenBody::Text { replay, .. }
            | OpenBody::Thinking { replay, .. }
            | OpenBody::RedactedThinking { replay }
            | OpenBody::RuntimeCall { replay, .. }
            | OpenBody::ProviderCall { replay, .. }
            | OpenBody::ProviderResult { replay, .. }
            | OpenBody::Record { replay, .. } => replay.as_ref(),
            OpenBody::Structured { .. } => None,
        }
    }

    pub(in crate::stream) fn replay_mut(&mut self) -> Option<&mut Option<ReplayMaterial>> {
        match self {
            OpenBody::Text { replay, .. }
            | OpenBody::Thinking { replay, .. }
            | OpenBody::RedactedThinking { replay }
            | OpenBody::RuntimeCall { replay, .. }
            | OpenBody::ProviderCall { replay, .. }
            | OpenBody::ProviderResult { replay, .. }
            | OpenBody::Record { replay, .. } => Some(replay),
            OpenBody::Structured { .. } => None,
        }
    }
}

pub(in crate::stream) fn block_ids(block: &ContentBlock) -> Vec<Uuid> {
    let mut ids = vec![block.id().as_uuid()];
    if let ContentBlock::Text(text) = block {
        ids.extend(text.citations().iter().map(|c| c.id().as_uuid()));
    }
    for child in block.children() {
        ids.push(child.id().as_uuid());
        if let PlainBlock::Text(text) = child {
            ids.extend(text.citations().iter().map(|c| c.id().as_uuid()));
        }
    }
    ids
}
