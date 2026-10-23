use super::{ResponseDraft, target_error};
use crate::error::{Error, ErrorCode};
use crate::id::BlockId;
use crate::value::{ReplayFormat, ReplayMaterial, ReplayPayload};

fn mismatch() -> Error {
    Error::new(ErrorCode::ReplayMismatch)
}

impl ResponseDraft {
    pub(super) fn append_replay(
        &mut self,
        block: BlockId,
        format: &ReplayFormat,
        fragment: &str,
    ) -> Result<(), Error> {
        let open = self.open_mut(block)?;
        let slot = open.body.replay_mut().ok_or_else(target_error)?;
        match slot {
            None => {
                *slot = Some(ReplayMaterial::new(
                    format.clone(),
                    ReplayPayload::text(fragment),
                ));
                open.replay_fragments = true;
                Ok(())
            }
            Some(material) => {
                if !open.replay_fragments {
                    return Err(mismatch().at("mode"));
                }
                if material.format() != format {
                    return Err(mismatch().at("format"));
                }
                match material.text_mut() {
                    Some(text) => {
                        text.push_str(fragment);
                        Ok(())
                    }
                    None => Err(mismatch().at("payload")),
                }
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
            return Err(mismatch().at("material"));
        }
        *slot = Some(material.clone());
        Ok(())
    }
}
