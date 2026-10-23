use super::slot::{OpenBody, Slot};
use super::{DraftPhase, ResponseDraft};
use crate::block::{ContentBlock, ContentBlockKind};
use crate::entry::ResponseMetadata;
use crate::id::{BlockId, EntryId, TurnId};
use crate::value::{
    Citation, ExecutionStatus, ExternalId, ObservedAt, ProviderOperation, ReplayMaterial, ToolName,
};

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SlotLifecycle {
    Open,
    Final,
    Discarded,
}

#[derive(Debug, Clone, Copy)]
pub struct DraftView<'a> {
    draft: &'a ResponseDraft,
}

#[derive(Debug, Clone, Copy)]
pub struct SlotView<'a> {
    position: u32,
    slot: &'a Slot,
}

impl<'a> DraftView<'a> {
    pub(super) fn new(draft: &'a ResponseDraft) -> Self {
        DraftView { draft }
    }

    pub fn turn(&self) -> TurnId {
        self.draft.turn
    }

    pub fn entry(&self) -> EntryId {
        self.draft.entry
    }

    pub fn phase(&self) -> &'a DraftPhase {
        &self.draft.phase
    }

    pub fn metadata(&self) -> &'a ResponseMetadata {
        &self.draft.metadata
    }

    pub fn slots(&self) -> Vec<SlotView<'a>> {
        self.draft
            .slots
            .iter()
            .map(|(position, slot)| SlotView {
                position: *position,
                slot,
            })
            .collect()
    }

    pub fn slot(&self, block: BlockId) -> Option<SlotView<'a>> {
        let position = *self.draft.positions.get(&block)?;
        self.draft
            .slots
            .get(&position)
            .map(|slot| SlotView { position, slot })
    }
}

impl<'a> SlotView<'a> {
    pub fn position(&self) -> u32 {
        self.position
    }

    pub fn block(&self) -> BlockId {
        self.slot.id()
    }

    pub fn kind(&self) -> ContentBlockKind {
        self.slot.kind()
    }

    pub fn lifecycle(&self) -> SlotLifecycle {
        match self.slot {
            Slot::Open(_) => SlotLifecycle::Open,
            Slot::Final(_) => SlotLifecycle::Final,
            Slot::Discarded { .. } => SlotLifecycle::Discarded,
        }
    }

    pub fn observed_at(&self) -> Option<&'a ObservedAt> {
        match self.slot {
            Slot::Open(open) => open.observed_at.as_ref(),
            Slot::Final(block) => block.observed_at(),
            Slot::Discarded { .. } => None,
        }
    }

    pub fn text(&self) -> Option<&'a str> {
        match self.slot {
            Slot::Open(open) => match &open.body {
                OpenBody::Text { text, .. } | OpenBody::Thinking { text, .. } => Some(text),
                _ => None,
            },
            Slot::Final(ContentBlock::Text(block)) => Some(block.text().as_str()),
            Slot::Final(ContentBlock::Thinking(block)) => Some(block.text()),
            _ => None,
        }
    }

    pub fn raw_json(&self) -> Option<&'a str> {
        match self.slot {
            Slot::Open(open) => match &open.body {
                OpenBody::Structured { raw }
                | OpenBody::RuntimeCall { raw, .. }
                | OpenBody::ProviderCall { raw, .. } => Some(raw),
                _ => None,
            },
            _ => None,
        }
    }

    pub fn name(&self) -> Option<&'a ToolName> {
        match self.slot {
            Slot::Open(open) => match &open.body {
                OpenBody::RuntimeCall { name, .. } => name.as_ref(),
                _ => None,
            },
            Slot::Final(ContentBlock::RuntimeToolCall(call)) => Some(call.name()),
            _ => None,
        }
    }

    pub fn external_id(&self) -> Option<&'a ExternalId> {
        match self.slot {
            Slot::Open(open) => match &open.body {
                OpenBody::RuntimeCall { external_id, .. }
                | OpenBody::ProviderCall { external_id, .. }
                | OpenBody::Record { external_id, .. } => external_id.as_ref(),
                _ => None,
            },
            Slot::Final(ContentBlock::RuntimeToolCall(call)) => call.external_id(),
            Slot::Final(ContentBlock::ProviderToolCall(call)) => call.external_id(),
            Slot::Final(ContentBlock::ProviderToolResult(result)) => result.external_id(),
            Slot::Final(ContentBlock::ProviderToolExecutionRecord(record)) => record.external_id(),
            _ => None,
        }
    }

    pub fn operation(&self) -> Option<&'a ProviderOperation> {
        match self.slot {
            Slot::Open(open) => match &open.body {
                OpenBody::ProviderCall { operation, .. } | OpenBody::Record { operation, .. } => {
                    operation.as_ref()
                }
                _ => None,
            },
            Slot::Final(ContentBlock::ProviderToolCall(call)) => call.operation(),
            Slot::Final(ContentBlock::ProviderToolExecutionRecord(record)) => record.operation(),
            _ => None,
        }
    }

    pub fn status(&self) -> Option<ExecutionStatus> {
        match self.slot {
            Slot::Open(open) => match &open.body {
                OpenBody::Record { status, .. } => status.clone(),
                _ => None,
            },
            Slot::Final(ContentBlock::ProviderToolExecutionRecord(record)) => record.status(),
            _ => None,
        }
    }

    pub fn citations(&self) -> &'a [Citation] {
        match self.slot {
            Slot::Open(open) => match &open.body {
                OpenBody::Text { citations, .. } => citations,
                _ => &[],
            },
            Slot::Final(ContentBlock::Text(block)) => block.citations(),
            _ => &[],
        }
    }

    pub fn replay(&self) -> Option<&'a ReplayMaterial> {
        match self.slot {
            Slot::Open(open) => open.body.replay(),
            Slot::Final(block) => match block {
                ContentBlock::Text(block) => block.replay(),
                ContentBlock::Thinking(block) => block.replay(),
                ContentBlock::RedactedThinking(block) => Some(block.material()),
                ContentBlock::RuntimeToolCall(block) => block.replay(),
                ContentBlock::ProviderToolCall(block) => block.replay(),
                ContentBlock::ProviderToolResult(block) => block.replay(),
                _ => None,
            },
            Slot::Discarded { .. } => None,
        }
    }

    pub fn final_block(&self) -> Option<&'a ContentBlock> {
        match self.slot {
            Slot::Final(block) => Some(block),
            _ => None,
        }
    }
}
