use super::slot::OpenBody;
use super::{ResponseDraft, target_error};
use crate::error::Error;
use crate::id::BlockId;

impl ResponseDraft {
    pub(super) fn append_text(&mut self, block: BlockId, fragment: &str) -> Result<(), Error> {
        match &mut self.open_mut(block)?.body {
            OpenBody::Text { text, .. } => {
                text.push_str(fragment);
                Ok(())
            }
            _ => Err(target_error()),
        }
    }

    pub(super) fn append_thinking(&mut self, block: BlockId, fragment: &str) -> Result<(), Error> {
        match &mut self.open_mut(block)?.body {
            OpenBody::Thinking { text, .. } => {
                text.push_str(fragment);
                Ok(())
            }
            _ => Err(target_error()),
        }
    }

    pub(super) fn append_json(&mut self, block: BlockId, fragment: &str) -> Result<(), Error> {
        match self.open_mut(block)?.body.raw_json_mut() {
            Some(raw) => {
                raw.push_str(fragment);
                Ok(())
            }
            None => Err(target_error()),
        }
    }
}
