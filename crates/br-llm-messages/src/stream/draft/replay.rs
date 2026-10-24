use super::{ResponseDraft, target_error};
use crate::error::Error;
use crate::id::BlockId;
use crate::value::{ReplayFormat, ReplayMaterial, ReplayPayload};

impl ResponseDraft {
    pub(super) fn append_replay(
        &mut self,
        block: BlockId,
        format: &ReplayFormat,
        fragment: &str,
    ) -> Result<(), Error> {
        let slot = self
            .open_mut(block)?
            .body
            .replay_mut()
            .ok_or_else(target_error)?;
        match slot {
            Some(material) => material.append(format, fragment),
            None => {
                if !fragment.is_empty() {
                    *slot = Some(ReplayMaterial::new(
                        format.clone(),
                        ReplayPayload::text(fragment),
                    ));
                }
                Ok(())
            }
        }
    }

    pub(super) fn set_replay(
        &mut self,
        block: BlockId,
        material: &ReplayMaterial,
    ) -> Result<(), Error> {
        let slot = self
            .open_mut(block)?
            .body
            .replay_mut()
            .ok_or_else(target_error)?;
        if slot.is_some() {
            return Err(target_error().at("material"));
        }
        *slot = Some(material.clone());
        Ok(())
    }
}
