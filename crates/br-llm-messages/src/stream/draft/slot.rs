use uuid::Uuid;

use crate::block::{ContentBlock, ContentBlockKind, PlainBlock};
use crate::id::BlockId;
use crate::stream::ops::BlockStart;
use crate::value::{
    Citation, ExecutionStatus, ExternalId, ProviderOperation, ReplayMaterial, ToolName,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::stream) enum Slot {
    Open(OpenBlock),
    Final(ContentBlock),
    Discarded { id: BlockId, kind: ContentBlockKind },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::stream) struct OpenBlock {
    pub(in crate::stream) id: BlockId,
    pub(in crate::stream) body: OpenBody,
    pub(in crate::stream) replay_fragments: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::stream) enum OpenBody {
    Text {
        text: String,
        citations: Vec<Citation>,
        replay: Option<ReplayMaterial>,
    },
    Thinking {
        text: String,
        replay: Option<ReplayMaterial>,
    },
    Structured {
        raw: String,
    },
    RuntimeCall {
        name: Option<ToolName>,
        external_id: Option<ExternalId>,
        raw: String,
        replay: Option<ReplayMaterial>,
    },
    ProviderCall {
        operation: Option<ProviderOperation>,
        external_id: Option<ExternalId>,
        raw: String,
        replay: Option<ReplayMaterial>,
    },
    Record {
        operation: Option<ProviderOperation>,
        external_id: Option<ExternalId>,
        status: Option<ExecutionStatus>,
    },
}

impl Slot {
    pub(in crate::stream) fn id(&self) -> BlockId {
        match self {
            Slot::Open(open) => open.id,
            Slot::Final(block) => block.id(),
            Slot::Discarded { id, .. } => *id,
        }
    }

    pub(in crate::stream) fn kind(&self) -> ContentBlockKind {
        match self {
            Slot::Open(open) => open.body.kind(),
            Slot::Final(block) => block.kind(),
            Slot::Discarded { kind, .. } => *kind,
        }
    }
}

impl OpenBlock {
    pub(in crate::stream) fn started(id: BlockId, start: &BlockStart) -> Self {
        OpenBlock {
            id,
            body: OpenBody::from_start(start),
            replay_fragments: false,
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
            BlockStart::Text => OpenBody::Text {
                text: String::new(),
                citations: Vec::new(),
                replay: None,
            },
            BlockStart::Thinking => OpenBody::Thinking {
                text: String::new(),
                replay: None,
            },
            BlockStart::StructuredContent => OpenBody::Structured { raw: String::new() },
            BlockStart::RuntimeToolCall { name, external_id } => OpenBody::RuntimeCall {
                name: name.clone(),
                external_id: external_id.clone(),
                raw: String::new(),
                replay: None,
            },
            BlockStart::ProviderToolCall {
                operation,
                external_id,
            } => OpenBody::ProviderCall {
                operation: operation.clone(),
                external_id: external_id.clone(),
                raw: String::new(),
                replay: None,
            },
            BlockStart::ProviderToolExecutionRecord {
                operation,
                external_id,
                status,
            } => OpenBody::Record {
                operation: operation.clone(),
                external_id: external_id.clone(),
                status: *status,
            },
        }
    }

    pub(in crate::stream) fn kind(&self) -> ContentBlockKind {
        match self {
            OpenBody::Text { .. } => ContentBlockKind::Text,
            OpenBody::Thinking { .. } => ContentBlockKind::Thinking,
            OpenBody::Structured { .. } => ContentBlockKind::StructuredContent,
            OpenBody::RuntimeCall { .. } => ContentBlockKind::RuntimeToolCall,
            OpenBody::ProviderCall { .. } => ContentBlockKind::ProviderToolCall,
            OpenBody::Record { .. } => ContentBlockKind::ProviderToolExecutionRecord,
        }
    }

    pub(in crate::stream) fn raw_json_mut(&mut self) -> Option<&mut String> {
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
            | OpenBody::RuntimeCall { replay, .. }
            | OpenBody::ProviderCall { replay, .. } => replay.as_ref(),
            _ => None,
        }
    }

    pub(in crate::stream) fn replay_mut(&mut self) -> Option<&mut Option<ReplayMaterial>> {
        match self {
            OpenBody::Text { replay, .. }
            | OpenBody::Thinking { replay, .. }
            | OpenBody::RuntimeCall { replay, .. }
            | OpenBody::ProviderCall { replay, .. } => Some(replay),
            _ => None,
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
