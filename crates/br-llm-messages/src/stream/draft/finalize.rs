use super::build::accumulated;
use super::slot::{OpenBlock, Slot, block_ids};
use super::{ResponseDraft, phase_error, target_error};
use crate::block::ContentBlock;
use crate::error::{Error, ErrorCode};
use crate::id::BlockId;
use crate::stream::ops::Finalization;

fn authoritative(open: &OpenBlock, block: &ContentBlock) -> Result<ContentBlock, Error> {
    if block.id() != open.id {
        return Err(Error::new(ErrorCode::InconsistentFinalization).at("block.id"));
    }
    if block.kind() != open.body.kind() {
        return Err(Error::new(ErrorCode::InconsistentFinalization).at("block.type"));
    }
    Ok(block.clone())
}

impl ResponseDraft {
    pub(super) fn finalize_block(
        &mut self,
        block: BlockId,
        finalization: &Finalization,
    ) -> Result<bool, Error> {
        let (at, open) = self.open_ref(block)?;
        let (built, replaced) = match finalization {
            Finalization::Accumulated { .. } => (accumulated(open)?, false),
            Finalization::Authoritative { block: value } => (authoritative(open, value)?, true),
        };
        let previous = open.ids();
        let next = block_ids(&built);
        if next
            .iter()
            .any(|id| !previous.contains(id) && self.ids.contains(id))
        {
            return Err(Error::new(ErrorCode::ReusedIdentity).at("block"));
        }
        for id in &previous {
            self.ids.remove(id);
        }
        self.ids.extend(next);
        let slot = self.slots.get_mut(at).ok_or_else(target_error)?;
        *slot = Slot::Final(built);
        Ok(replaced)
    }

    fn open_ref(&self, block: BlockId) -> Result<(usize, &OpenBlock), Error> {
        let at = *self.index.get(&block).ok_or_else(target_error)?;
        match self.slots.get(at) {
            Some(Slot::Open(open)) => Ok((at, open)),
            Some(Slot::Final(_)) => Err(phase_error()),
            None => Err(target_error()),
        }
    }
}
