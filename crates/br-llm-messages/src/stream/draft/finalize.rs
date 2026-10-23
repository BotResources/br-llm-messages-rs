use super::slot::{OpenBlock, OpenBody, Slot, block_ids};
use super::{ResponseDraft, phase_error, target_error};
use crate::block::{
    ContentBlock, ContentBlockKind, ProviderToolCall, RuntimeToolCall, StructuredBlock, TextBlock,
    ThinkingBlock,
};
use crate::error::{Error, ErrorCode};
use crate::id::BlockId;
use crate::opaque_json::OpaqueJson;
use crate::stream::ops::Finalization;

fn inconsistent(location: &str) -> Error {
    Error::new(ErrorCode::InconsistentFinalization).at(location)
}

fn is_blank(text: &str) -> bool {
    text.chars().all(char::is_whitespace)
}

fn parse(raw: &str) -> Result<OpaqueJson, Error> {
    OpaqueJson::parse(raw)
}

fn accumulated(open: &OpenBlock) -> Result<ContentBlock, Error> {
    let id = open.id;
    match &open.body {
        OpenBody::Text {
            text,
            citations,
            replay,
        } => {
            if is_blank(text) {
                return Err(inconsistent("text"));
            }
            let block = TextBlock::new(id, text.clone())
                .and_then(|block| block.with_citations(citations.clone()))
                .map_err(|error| error.recoded(ErrorCode::InconsistentFinalization))?;
            Ok(match replay {
                Some(replay) => block.with_replay(replay.clone()),
                None => block,
            }
            .into())
        }
        OpenBody::Thinking { text, replay } => {
            let block = ThinkingBlock::new(id, text.clone());
            Ok(match replay {
                Some(replay) => block.with_replay(replay.clone()),
                None => block,
            }
            .into())
        }
        OpenBody::Structured { raw } => Ok(StructuredBlock::new(id, parse(raw)?)?.into()),
        OpenBody::RuntimeCall {
            name,
            external_id,
            raw,
            replay,
        } => {
            let name = name.clone().ok_or_else(|| inconsistent("name"))?;
            let mut call = RuntimeToolCall::new(id, name, parse(raw)?)?;
            if let Some(external_id) = external_id {
                call = call.with_external_id(external_id.clone());
            }
            if let Some(replay) = replay {
                call = call.with_replay(replay.clone());
            }
            Ok(call.into())
        }
        OpenBody::ProviderCall {
            operation,
            external_id,
            raw,
            replay,
        } => {
            let mut call = ProviderToolCall::new(id);
            if let Some(operation) = operation {
                call = call.with_operation(operation.clone());
            }
            if let Some(external_id) = external_id {
                call = call.with_external_id(external_id.clone());
            }
            if !raw.is_empty() {
                call = call.with_arguments(parse(raw)?)?;
            }
            if let Some(replay) = replay {
                call = call.with_replay(replay.clone());
            }
            Ok(call.into())
        }
        OpenBody::Record { .. } => Err(inconsistent("finalization")),
    }
}

fn authoritative(open: &OpenBlock, block: &ContentBlock) -> Result<ContentBlock, Error> {
    if block.id() != open.id {
        return Err(inconsistent("block.id"));
    }
    if block.kind() != open.body.kind() {
        return Err(inconsistent("block.type"));
    }
    Ok(block.clone())
}

impl ResponseDraft {
    pub(super) fn finalize_block(
        &mut self,
        block: BlockId,
        finalization: &Finalization,
    ) -> Result<(), Error> {
        let (position, open) = self.open_ref(block)?;
        let built = match finalization {
            Finalization::Accumulated { .. } => accumulated(open)?,
            Finalization::Authoritative { block: value } => authoritative(open, value)?,
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
        self.slots.insert(position, Slot::Final(built));
        Ok(())
    }

    pub(super) fn discard_block(&mut self, block: BlockId) -> Result<u32, Error> {
        let (position, open) = self.open_ref(block)?;
        match &open.body {
            OpenBody::Text {
                text,
                citations,
                replay,
            } if is_blank(text) && citations.is_empty() && replay.is_none() => {}
            _ => return Err(inconsistent("block")),
        }
        self.slots.insert(
            position,
            Slot::Discarded {
                id: block,
                kind: ContentBlockKind::Text,
            },
        );
        Ok(position)
    }

    fn open_ref(&self, block: BlockId) -> Result<(u32, &OpenBlock), Error> {
        let position = *self.positions.get(&block).ok_or_else(target_error)?;
        match self.slots.get(&position) {
            Some(Slot::Open(open)) => Ok((position, open)),
            Some(_) => Err(phase_error()),
            None => Err(target_error()),
        }
    }
}
