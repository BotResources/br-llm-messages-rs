use super::super::draft::slot::{OpenBlock, OpenBody};
use crate::block::ContentBlockKind;
use crate::error::{Error, ErrorCode};
use crate::id::EntryId;
use crate::record::{
    BlockRecord, ProviderCallPartial, RecordPartial, RepresentationState, RuntimeCallPartial,
    StructuredPartial, TextPartial, ThinkingPartial, decode_partial, encode_partial,
};
use crate::value::ReplayPayload;

pub(super) fn open_row(
    entry: EntryId,
    position: u32,
    state: RepresentationState,
    open: &OpenBlock,
) -> Result<BlockRecord, Error> {
    let payload = match &open.body {
        OpenBody::Text {
            text,
            citations,
            replay,
        } => encode_partial(&TextPartial {
            text: text.clone(),
            citations: citations.clone(),
            replay: replay.clone(),
            replay_fragments: open.replay_fragments,
        }),
        OpenBody::Thinking { text, replay } => encode_partial(&ThinkingPartial {
            text: text.clone(),
            replay: replay.clone(),
            replay_fragments: open.replay_fragments,
        }),
        OpenBody::Structured { raw } => encode_partial(&StructuredPartial {
            raw_json: raw.clone(),
        }),
        OpenBody::RuntimeCall {
            name,
            external_id,
            raw,
            replay,
        } => encode_partial(&RuntimeCallPartial {
            name: name.clone(),
            external_id: external_id.clone(),
            raw_arguments: raw.clone(),
            replay: replay.clone(),
            replay_fragments: open.replay_fragments,
        }),
        OpenBody::ProviderCall {
            operation,
            external_id,
            raw,
            replay,
        } => encode_partial(&ProviderCallPartial {
            operation: operation.clone(),
            external_id: external_id.clone(),
            raw_arguments: raw.clone(),
            replay: replay.clone(),
            replay_fragments: open.replay_fragments,
        }),
        OpenBody::Record {
            operation,
            external_id,
            status,
        } => encode_partial(&RecordPartial {
            operation: operation.clone(),
            external_id: external_id.clone(),
            status: *status,
        }),
    }?;
    Ok(BlockRecord::new(
        open.id,
        entry,
        open.body.kind(),
        position,
        state,
        payload,
    ))
}

pub(super) fn open_from_row(row: &BlockRecord) -> Result<OpenBlock, Error> {
    let payload = row.payload();
    let mut replay_fragments = false;
    let body = match row.kind() {
        ContentBlockKind::Text => {
            let partial: TextPartial = decode_partial(payload)?;
            replay_fragments = partial.replay_fragments;
            OpenBody::Text {
                text: partial.text,
                citations: partial.citations,
                replay: partial.replay,
            }
        }
        ContentBlockKind::Thinking => {
            let partial: ThinkingPartial = decode_partial(payload)?;
            replay_fragments = partial.replay_fragments;
            OpenBody::Thinking {
                text: partial.text,
                replay: partial.replay,
            }
        }
        ContentBlockKind::StructuredContent => {
            let partial: StructuredPartial = decode_partial(payload)?;
            OpenBody::Structured {
                raw: partial.raw_json,
            }
        }
        ContentBlockKind::RuntimeToolCall => {
            let partial: RuntimeCallPartial = decode_partial(payload)?;
            replay_fragments = partial.replay_fragments;
            OpenBody::RuntimeCall {
                name: partial.name,
                external_id: partial.external_id,
                raw: partial.raw_arguments,
                replay: partial.replay,
            }
        }
        ContentBlockKind::ProviderToolCall => {
            let partial: ProviderCallPartial = decode_partial(payload)?;
            replay_fragments = partial.replay_fragments;
            OpenBody::ProviderCall {
                operation: partial.operation,
                external_id: partial.external_id,
                raw: partial.raw_arguments,
                replay: partial.replay,
            }
        }
        ContentBlockKind::ProviderToolExecutionRecord => {
            let partial: RecordPartial = decode_partial(payload)?;
            OpenBody::Record {
                operation: partial.operation,
                external_id: partial.external_id,
                status: partial.status,
            }
        }
        _ => {
            return Err(Error::new(ErrorCode::InvalidRecord)
                .with_id(row.id().as_uuid())
                .at("kind"));
        }
    };
    let block = OpenBlock {
        id: row.id(),
        body,
        replay_fragments,
    };
    check_fragments(&block)?;
    Ok(block)
}

fn check_fragments(block: &OpenBlock) -> Result<(), Error> {
    if !block.replay_fragments {
        return Ok(());
    }
    let assembled = block
        .body
        .replay()
        .is_some_and(|material| matches!(material.payload(), ReplayPayload::Text { .. }));
    if assembled {
        Ok(())
    } else {
        Err(Error::new(ErrorCode::InvalidRecord)
            .with_id(block.id.as_uuid())
            .at("payload.replay_fragments"))
    }
}
