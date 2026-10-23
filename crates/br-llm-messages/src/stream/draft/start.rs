use super::slot::{OpenBlock, OpenBody, Slot, block_ids};
use super::{ResponseDraft, target_error};
use crate::block::{ContentBlock, ContentBlockKind};
use crate::error::{Error, ErrorCode};
use crate::id::BlockId;
use crate::stream::ops::{BlockMetadataUpdate, BlockStart};

impl ResponseDraft {
    pub(super) fn start_block(
        &mut self,
        block: BlockId,
        position: u32,
        start: &BlockStart,
    ) -> Result<(), Error> {
        self.ensure_free(&[block.as_uuid()], position)?;
        self.ids.insert(block.as_uuid());
        self.positions.insert(block, position);
        self.slots
            .insert(position, Slot::Open(OpenBlock::started(block, start)));
        Ok(())
    }

    pub(super) fn insert_block(
        &mut self,
        position: u32,
        block: &ContentBlock,
    ) -> Result<(), Error> {
        if block.kind() == ContentBlockKind::RuntimeToolResult {
            return Err(Error::new(ErrorCode::DisallowedBlockKind));
        }
        let ids = block_ids(block);
        self.ensure_free(&ids, position)?;
        self.ids.extend(ids);
        self.positions.insert(block.id(), position);
        self.slots.insert(position, Slot::Final(block.clone()));
        Ok(())
    }

    pub(super) fn update_block(
        &mut self,
        block: BlockId,
        update: &BlockMetadataUpdate,
    ) -> Result<(), Error> {
        match update {
            BlockMetadataUpdate::CitationAdded { citation } => {
                let duplicate = self.ids.contains(&citation.id().as_uuid());
                match &mut self.open_mut(block)?.body {
                    OpenBody::Text { citations, .. } => {
                        if duplicate {
                            return Err(Error::new(ErrorCode::ReusedIdentity).at("citation"));
                        }
                        citations.push(citation.clone());
                    }
                    _ => return Err(target_error()),
                }
                self.ids.insert(citation.id().as_uuid());
                Ok(())
            }
            BlockMetadataUpdate::RuntimeToolCallIdentified { name, external_id } => {
                match &mut self.open_mut(block)?.body {
                    OpenBody::RuntimeCall {
                        name: current_name,
                        external_id: current_id,
                        ..
                    } => {
                        replace(current_name, name);
                        replace(current_id, external_id);
                        Ok(())
                    }
                    _ => Err(target_error()),
                }
            }
            BlockMetadataUpdate::ProviderReferencesUpdated {
                operation,
                external_id,
            } => match &mut self.open_mut(block)?.body {
                OpenBody::ProviderCall {
                    operation: current_operation,
                    external_id: current_id,
                    ..
                }
                | OpenBody::Record {
                    operation: current_operation,
                    external_id: current_id,
                    ..
                } => {
                    replace(current_operation, operation);
                    replace(current_id, external_id);
                    Ok(())
                }
                _ => Err(target_error()),
            },
            BlockMetadataUpdate::ExecutionStatusObserved { status } => {
                match &mut self.open_mut(block)?.body {
                    OpenBody::Record {
                        status: current, ..
                    } => {
                        *current = Some(status.clone());
                        Ok(())
                    }
                    _ => Err(target_error()),
                }
            }
        }
    }

    fn ensure_free(&self, ids: &[uuid::Uuid], position: u32) -> Result<(), Error> {
        if ids.iter().any(|id| self.ids.contains(id)) {
            return Err(Error::new(ErrorCode::ReusedIdentity));
        }
        if self.slots.contains_key(&position) {
            return Err(Error::new(ErrorCode::InvalidPosition).at("position"));
        }
        Ok(())
    }
}

fn replace<T: Clone>(current: &mut Option<T>, update: &Option<T>) {
    if let Some(value) = update {
        *current = Some(value.clone());
    }
}
