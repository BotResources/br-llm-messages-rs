use super::slot::Slot;
use super::{DraftPhase, ResponseDraft};
use crate::block::ContentBlock;
use crate::entry::ModelResponse;
use crate::error::{Error, ErrorCode};
use crate::value::ResponseEnding;

impl ResponseDraft {
    pub(super) fn complete(&mut self, ending: &ResponseEnding) -> Result<(), Error> {
        if let Some((position, _)) = self
            .slots
            .iter()
            .find(|(_, slot)| matches!(slot, Slot::Open(_)))
        {
            return Err(
                Error::new(ErrorCode::InconsistentFinalization).at(format!("content[{position}]"))
            );
        }
        let mut blocks: Vec<ContentBlock> = Vec::with_capacity(self.slots.len());
        for (expected, (position, slot)) in self.slots.iter().enumerate() {
            if u32::try_from(expected) != Ok(*position) {
                return Err(
                    Error::new(ErrorCode::InvalidPosition).at(format!("content[{expected}]"))
                );
            }
            if let Slot::Final(block) = slot {
                blocks.push(block.clone());
            }
        }
        let response =
            ModelResponse::new(self.entry, blocks, ending.clone(), self.metadata.clone())?;
        self.response = Some(response);
        self.phase = DraftPhase::Completed {};
        Ok(())
    }
}
