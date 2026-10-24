use super::raw_json::RawJson;
use super::slot::{OpenBlock, OpenBody};
use crate::block::{
    ContentBlock, ProviderToolCall, ProviderToolExecutionRecord, ProviderToolResult,
    RedactedThinkingBlock, RuntimeToolCall, StructuredBlock, TextBlock, ThinkingBlock,
};
use crate::error::{Error, ErrorCode};
use crate::opaque_json::OpaqueJson;
use crate::value::ObservedAt;

fn inconsistent(location: &str) -> Error {
    Error::new(ErrorCode::InconsistentFinalization).at(location)
}

fn is_blank(text: &str) -> bool {
    text.chars().all(char::is_whitespace)
}

fn arguments(raw: &RawJson, provisional: &Option<OpaqueJson>) -> Result<Option<OpaqueJson>, Error> {
    if !is_blank(raw.as_str()) {
        return OpaqueJson::parse(raw.as_str()).map(Some);
    }
    Ok(provisional.clone())
}

pub(super) fn accumulated(open: &OpenBlock) -> Result<ContentBlock, Error> {
    let block = body_block(open)?;
    Ok(match &open.observed_at {
        Some(observed_at) => with_observed_at(block, observed_at.clone()),
        None => block,
    })
}

fn with_observed_at(block: ContentBlock, observed_at: ObservedAt) -> ContentBlock {
    match block {
        ContentBlock::Text(block) => block.with_observed_at(observed_at).into(),
        ContentBlock::Thinking(block) => block.with_observed_at(observed_at).into(),
        ContentBlock::RedactedThinking(block) => block.with_observed_at(observed_at).into(),
        ContentBlock::StructuredContent(block) => block.with_observed_at(observed_at).into(),
        ContentBlock::RuntimeToolCall(block) => block.with_observed_at(observed_at).into(),
        ContentBlock::ProviderToolCall(block) => block.with_observed_at(observed_at).into(),
        ContentBlock::ProviderToolResult(block) => block.with_observed_at(observed_at).into(),
        ContentBlock::ProviderToolExecutionRecord(block) => {
            block.with_observed_at(observed_at).into()
        }
        other => other,
    }
}

fn body_block(open: &OpenBlock) -> Result<ContentBlock, Error> {
    let id = open.id;
    match &open.body {
        OpenBody::Text {
            text,
            citations,
            item_ref,
            replay,
        } => {
            if is_blank(text) {
                return Err(inconsistent("text"));
            }
            let mut block = TextBlock::new(id, text.clone())
                .and_then(|block| block.with_citations(citations.clone()))
                .map_err(|error| error.recoded(ErrorCode::InconsistentFinalization))?;
            if let Some(item_ref) = item_ref {
                block = block.with_item_ref(item_ref.clone());
            }
            if let Some(replay) = replay {
                block = block.with_replay(replay.clone());
            }
            Ok(block.into())
        }
        OpenBody::Thinking { text, replay } => {
            let block = match replay {
                Some(replay) if text.is_empty() => ThinkingBlock::replay_only(id, replay.clone()),
                Some(replay) => ThinkingBlock::new(id, text.clone())
                    .map_err(|error| error.recoded(ErrorCode::InconsistentFinalization))?
                    .with_replay(replay.clone()),
                None => ThinkingBlock::new(id, text.clone())
                    .map_err(|error| error.recoded(ErrorCode::InconsistentFinalization))?,
            };
            Ok(block.into())
        }
        OpenBody::RedactedThinking { replay } => {
            let replay = replay.clone().ok_or_else(|| inconsistent("replay"))?;
            Ok(RedactedThinkingBlock::new(id, replay).into())
        }
        OpenBody::Structured { raw } => {
            if is_blank(raw.as_str()) {
                return Err(inconsistent("value"));
            }
            Ok(StructuredBlock::new(id, OpaqueJson::parse(raw.as_str())?)?.into())
        }
        OpenBody::RuntimeCall {
            name,
            external_id,
            item_ref,
            provisional,
            raw,
            replay,
        } => {
            let name = name.clone().ok_or_else(|| inconsistent("name"))?;
            let value = match arguments(raw, provisional)? {
                Some(value) => value,
                None => OpaqueJson::empty_object(),
            };
            let mut call = RuntimeToolCall::new(id, name, value)?;
            if let Some(external_id) = external_id {
                call = call.with_external_id(external_id.clone());
            }
            if let Some(item_ref) = item_ref {
                call = call.with_item_ref(item_ref.clone());
            }
            if let Some(replay) = replay {
                call = call.with_replay(replay.clone());
            }
            Ok(call.into())
        }
        OpenBody::ProviderCall {
            operation,
            external_id,
            item_ref,
            provisional,
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
            if let Some(item_ref) = item_ref {
                call = call.with_item_ref(item_ref.clone());
            }
            if let Some(value) = arguments(raw, provisional)? {
                call = call.with_arguments(value)?;
            }
            if let Some(replay) = replay {
                call = call.with_replay(replay.clone());
            }
            Ok(call.into())
        }
        OpenBody::ProviderResult {
            call,
            external_id,
            replay,
        } => {
            let replay = replay.clone().ok_or_else(|| inconsistent("replay"))?;
            let mut result = ProviderToolResult::replay_only(id, *call, replay)?;
            if let Some(external_id) = external_id {
                result = result.with_external_id(external_id.clone());
            }
            Ok(result.into())
        }
        OpenBody::Record {
            operation,
            external_id,
            status,
            replay,
        } => {
            let mut record = ProviderToolExecutionRecord::new(id);
            if let Some(operation) = operation {
                record = record.with_operation(operation.clone());
            }
            if let Some(external_id) = external_id {
                record = record.with_external_id(external_id.clone());
            }
            if let Some(status) = status {
                record = record.with_status(status.clone());
            }
            if let Some(replay) = replay {
                record = record.with_replay(replay.clone());
            }
            Ok(record.into())
        }
    }
}
