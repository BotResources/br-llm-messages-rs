use super::slot::{OpenBlock, OpenBody, Slot, block_ids};
use super::{ResponseDraft, target_error};
use crate::block::{ContentBlock, ContentBlockKind};
use crate::error::{Error, ErrorCode};
use crate::id::BlockId;
use crate::stream::block_start::BlockStart;
use crate::stream::ops::BlockMetadataUpdate;

impl ResponseDraft {
    pub(super) fn start_block(&mut self, block: BlockId, start: &BlockStart) -> Result<(), Error> {
        self.ensure_free(&[block.as_uuid()])?;
        self.ids.insert(block.as_uuid());
        self.index.insert(block, self.slots.len());
        self.slots
            .push(Slot::Open(OpenBlock::started(block, start)));
        Ok(())
    }

    pub(super) fn insert_block(&mut self, block: &ContentBlock) -> Result<(), Error> {
        if block.kind() == ContentBlockKind::RuntimeToolResult {
            return Err(Error::new(ErrorCode::DisallowedKind));
        }
        let ids = block_ids(block);
        self.ensure_free(&ids)?;
        self.ids.extend(ids);
        self.index.insert(block.id(), self.slots.len());
        self.slots.push(Slot::Final(block.clone()));
        Ok(())
    }

    pub(super) fn update_block(
        &mut self,
        block: BlockId,
        update: &BlockMetadataUpdate,
    ) -> Result<(), Error> {
        let duplicate = match update {
            BlockMetadataUpdate::CitationAdded { citation } => {
                self.ids.contains(&citation.id().as_uuid())
            }
            _ => false,
        };
        let body = &mut self.open_mut(block)?.body;
        match (update, body) {
            (BlockMetadataUpdate::CitationAdded { citation }, OpenBody::Text { citations, .. }) => {
                if duplicate {
                    return Err(Error::new(ErrorCode::ReusedIdentity).at("citation"));
                }
                citations.push(citation.clone());
                self.ids.insert(citation.id().as_uuid());
            }
            (
                BlockMetadataUpdate::ToolNameSet { name: value },
                OpenBody::RuntimeCall { name, .. },
            ) => {
                *name = Some(value.clone());
            }
            (
                BlockMetadataUpdate::ExternalIdSet { external_id: value },
                OpenBody::RuntimeCall { external_id, .. }
                | OpenBody::ProviderCall { external_id, .. }
                | OpenBody::ProviderResult { external_id, .. }
                | OpenBody::Record { external_id, .. },
            ) => {
                *external_id = Some(value.clone());
            }
            (
                BlockMetadataUpdate::ItemRefSet { item_ref: value },
                OpenBody::Text { item_ref, .. }
                | OpenBody::RuntimeCall { item_ref, .. }
                | OpenBody::ProviderCall { item_ref, .. },
            ) => {
                *item_ref = Some(value.clone());
            }
            (
                BlockMetadataUpdate::OperationSet { operation: value },
                OpenBody::ProviderCall { operation, .. } | OpenBody::Record { operation, .. },
            ) => {
                *operation = Some(value.clone());
            }
            (
                BlockMetadataUpdate::ExecutionStatusSet { status: value },
                OpenBody::Record { status, .. },
            ) => {
                *status = Some(value.clone());
            }
            _ => return Err(target_error()),
        }
        Ok(())
    }

    fn ensure_free(&self, ids: &[uuid::Uuid]) -> Result<(), Error> {
        if ids.iter().any(|id| self.ids.contains(id)) {
            return Err(Error::new(ErrorCode::ReusedIdentity));
        }
        Ok(())
    }
}
