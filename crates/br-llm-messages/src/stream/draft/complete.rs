use super::slot::Slot;
use super::{DraftPhase, ResponseDraft};
use crate::block::ContentBlock;
use crate::entry::ModelResponse;
use crate::error::{Error, ErrorCode};
use crate::value::ResponseEnding;

impl ResponseDraft {
    pub(super) fn complete(&mut self, ending: &ResponseEnding) -> Result<(), Error> {
        let mut blocks: Vec<ContentBlock> = Vec::with_capacity(self.slots.len());
        for (index, slot) in self.slots.iter().enumerate() {
            match slot {
                Slot::Open(_) => {
                    return Err(Error::new(ErrorCode::InconsistentFinalization)
                        .at(format!("content[{index}]")));
                }
                Slot::Final(block) => blocks.push(block.clone()),
            }
        }
        let response =
            ModelResponse::new(self.entry, blocks, ending.clone(), self.metadata.clone())?;
        self.response = Some(response);
        self.phase = DraftPhase::Completed {};
        Ok(())
    }
}
