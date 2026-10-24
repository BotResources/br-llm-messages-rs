use super::super::draft::raw_json::RawJson;
use super::super::draft::slot::{OpenBlock, OpenBody};
use crate::block::ContentBlockKind;
use crate::error::{Error, ErrorCode};
use crate::id::EntryId;
use crate::record::{
    BlockRecord, ProviderCallPartial, ProviderResultPartial, RecordPartial, RedactedPartial,
    RepresentationState, RuntimeCallPartial, StructuredPartial, TextPartial, ThinkingPartial,
    decode_partial, encode_partial,
};

pub(super) fn open_row(
    entry: EntryId,
    state: RepresentationState,
    open: &OpenBlock,
) -> Result<BlockRecord, Error> {
    let payload = match &open.body {
        OpenBody::Text {
            text,
            citations,
            item_ref,
            replay,
        } => encode_partial(&TextPartial {
            text: text.clone(),
            citations: citations.clone(),
            item_ref: item_ref.clone(),
            replay: replay.clone(),
        }),
        OpenBody::Thinking { text, replay } => encode_partial(&ThinkingPartial {
            text: text.clone(),
            replay: replay.clone(),
        }),
        OpenBody::RedactedThinking { replay } => encode_partial(&RedactedPartial {
            replay: replay.clone(),
        }),
        OpenBody::Structured { raw } => encode_partial(&StructuredPartial {
            raw_json: raw.as_str().to_owned(),
        }),
        OpenBody::RuntimeCall {
            name,
            external_id,
            item_ref,
            provisional,
            raw,
            replay,
        } => encode_partial(&RuntimeCallPartial {
            name: name.clone(),
            external_id: external_id.clone(),
            item_ref: item_ref.clone(),
            provisional_arguments: provisional.clone(),
            raw_arguments: raw.as_str().to_owned(),
            replay: replay.clone(),
        }),
        OpenBody::ProviderCall {
            operation,
            external_id,
            item_ref,
            provisional,
            raw,
            replay,
        } => encode_partial(&ProviderCallPartial {
            operation: operation.clone(),
            external_id: external_id.clone(),
            item_ref: item_ref.clone(),
            provisional_arguments: provisional.clone(),
            raw_arguments: raw.as_str().to_owned(),
            replay: replay.clone(),
        }),
        OpenBody::ProviderResult {
            external_id,
            replay,
            ..
        } => encode_partial(&ProviderResultPartial {
            external_id: external_id.clone(),
            replay: replay.clone(),
        }),
        OpenBody::Record {
            operation,
            external_id,
            status,
            replay,
        } => encode_partial(&RecordPartial {
            operation: operation.clone(),
            external_id: external_id.clone(),
            status: status.clone(),
            replay: replay.clone(),
        }),
    }?;
    let mut row = BlockRecord::new(open.id, entry, open.body.kind(), state, payload);
    if let OpenBody::ProviderResult { call, .. } = &open.body {
        row = row.with_call(*call);
    }
    Ok(match &open.observed_at {
        Some(observed_at) => row.with_observed_at(observed_at.clone()),
        None => row,
    })
}

pub(super) fn open_from_row(row: &BlockRecord) -> Result<OpenBlock, Error> {
    let payload = row.payload();
    let body = match row.kind() {
        ContentBlockKind::Text => {
            let partial: TextPartial = decode_partial(payload)?;
            OpenBody::Text {
                text: partial.text,
                citations: partial.citations,
                item_ref: partial.item_ref,
                replay: partial.replay,
            }
        }
        ContentBlockKind::Thinking => {
            let partial: ThinkingPartial = decode_partial(payload)?;
            OpenBody::Thinking {
                text: partial.text,
                replay: partial.replay,
            }
        }
        ContentBlockKind::RedactedThinking => {
            let partial: RedactedPartial = decode_partial(payload)?;
            OpenBody::RedactedThinking {
                replay: partial.replay,
            }
        }
        ContentBlockKind::StructuredContent => {
            let partial: StructuredPartial = decode_partial(payload)?;
            OpenBody::Structured {
                raw: RawJson::from(partial.raw_json),
            }
        }
        ContentBlockKind::RuntimeToolCall => {
            let partial: RuntimeCallPartial = decode_partial(payload)?;
            OpenBody::RuntimeCall {
                name: partial.name,
                external_id: partial.external_id,
                item_ref: partial.item_ref,
                provisional: partial.provisional_arguments,
                raw: RawJson::from(partial.raw_arguments),
                replay: partial.replay,
            }
        }
        ContentBlockKind::ProviderToolCall => {
            let partial: ProviderCallPartial = decode_partial(payload)?;
            OpenBody::ProviderCall {
                operation: partial.operation,
                external_id: partial.external_id,
                item_ref: partial.item_ref,
                provisional: partial.provisional_arguments,
                raw: RawJson::from(partial.raw_arguments),
                replay: partial.replay,
            }
        }
        ContentBlockKind::ProviderToolResult => {
            let partial: ProviderResultPartial = decode_partial(payload)?;
            let call = row.call().ok_or_else(|| {
                Error::new(ErrorCode::InvalidContent)
                    .with_id(row.id().as_uuid())
                    .at("call")
            })?;
            OpenBody::ProviderResult {
                call,
                external_id: partial.external_id,
                replay: partial.replay,
            }
        }
        ContentBlockKind::ProviderToolExecutionRecord => {
            let partial: RecordPartial = decode_partial(payload)?;
            OpenBody::Record {
                operation: partial.operation,
                external_id: partial.external_id,
                status: partial.status,
                replay: partial.replay,
            }
        }
        _ => {
            return Err(Error::new(ErrorCode::DisallowedKind)
                .with_id(row.id().as_uuid())
                .at("kind"));
        }
    };
    Ok(OpenBlock {
        id: row.id(),
        body,
        observed_at: row.observed_at().cloned(),
    })
}
