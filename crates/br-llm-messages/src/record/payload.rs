use serde_json::{Map, Value};

use super::block::BlockRecord;
use super::entry::EntryRecord;
use super::kinds::ActorRef;
use crate::block::{ContentBlock, ContentBlockKind};
use crate::entry::{TurnEntry, TurnEntryKind};
use crate::error::{Error, ErrorCode};
use crate::id::BlockId;
use crate::json::encode_value;
use crate::value::Author;

fn into_object(value: Value) -> Result<Map<String, Value>, Error> {
    match value {
        Value::Object(object) => Ok(object),
        _ => Err(Error::new(ErrorCode::InvalidRecord)),
    }
}

pub(crate) fn call_of(block: &ContentBlock) -> Option<BlockId> {
    match block {
        ContentBlock::RuntimeToolResult(result) => Some(result.call()),
        ContentBlock::ProviderToolResult(result) => Some(result.call()),
        _ => None,
    }
}

pub(crate) fn block_payload(block: &ContentBlock) -> Result<Value, Error> {
    let mut object = into_object(block.to_value()?)?;
    for key in ["type", "id", "call"] {
        object.remove(key);
    }
    match block {
        ContentBlock::RuntimeToolResult(_) | ContentBlock::ProviderToolResult(_) => {
            object.remove("content");
        }
        ContentBlock::ProviderToolExecutionRecord(_) => {
            if let Some(Value::Object(exposure)) = object.get_mut("exposure") {
                exposure.remove("content");
            }
        }
        _ => {}
    }
    Ok(Value::Object(object))
}

pub(crate) fn block_value(row: &BlockRecord, children: Vec<Value>) -> Result<Value, Error> {
    let located = |error: Error| error.with_id(row.id().as_uuid());
    let mut object = row
        .payload()
        .as_object()
        .cloned()
        .ok_or_else(|| located(Error::new(ErrorCode::InvalidRecord).at("payload")))?;
    object.insert("type".into(), Value::String(row.kind().as_str().into()));
    object.insert("id".into(), encode_value(&row.id())?);
    if let Some(call) = row.call() {
        object.insert("call".into(), encode_value(&call)?);
    }
    match row.kind() {
        ContentBlockKind::RuntimeToolResult => {
            object.insert("content".into(), Value::Array(children));
        }
        ContentBlockKind::ProviderToolResult => {
            if !children.is_empty() {
                object.insert("content".into(), Value::Array(children));
            }
        }
        ContentBlockKind::ProviderToolExecutionRecord => {
            attach_exposure_content(&mut object, children).map_err(located)?;
        }
        _ => {
            if !children.is_empty() {
                return Err(located(Error::new(ErrorCode::DisallowedBlockKind)));
            }
        }
    }
    Ok(Value::Object(object))
}

fn attach_exposure_content(
    object: &mut Map<String, Value>,
    children: Vec<Value>,
) -> Result<(), Error> {
    let Some(Value::Object(exposure)) = object.get_mut("exposure") else {
        return Err(Error::new(ErrorCode::InvalidRecord).at("payload.exposure"));
    };
    let carries_content = matches!(
        exposure.get("type").and_then(Value::as_str),
        Some("readable" | "mixed")
    );
    match (carries_content, children.is_empty()) {
        (true, _) => {
            exposure.insert("content".into(), Value::Array(children));
            Ok(())
        }
        (false, true) => Ok(()),
        (false, false) => Err(Error::new(ErrorCode::InvalidRecord).at("payload.exposure")),
    }
}

pub(crate) struct EntryParts {
    pub(crate) payload: Value,
    pub(crate) sender: Option<ActorRef>,
    pub(crate) author: Option<Author>,
}

pub(crate) fn split_entry(entry: &TurnEntry) -> Result<EntryParts, Error> {
    let mut object = into_object(entry.to_value()?)?;
    for key in [
        "type",
        "id",
        "content",
        "results",
        "human",
        "requester",
        "sender",
        "author",
    ] {
        object.remove(key);
    }
    let (sender, author) = match entry {
        TurnEntry::HumanMessage(message) => (Some(ActorRef::human(message.human())), None),
        TurnEntry::AgentRequest(request) => (
            Some(ActorRef::agent(request.requester())),
            Some(request.author().clone()),
        ),
        TurnEntry::AgentMail(mail) => (
            Some(ActorRef::agent(mail.sender())),
            Some(mail.author().clone()),
        ),
        _ => (None, None),
    };
    Ok(EntryParts {
        payload: Value::Object(object),
        sender,
        author,
    })
}

fn sender_key(kind: TurnEntryKind) -> Option<&'static str> {
    match kind {
        TurnEntryKind::HumanMessage => Some("human"),
        TurnEntryKind::AgentRequest => Some("requester"),
        TurnEntryKind::AgentMail => Some("sender"),
        _ => None,
    }
}

fn block_key(kind: TurnEntryKind) -> Option<&'static str> {
    match kind {
        TurnEntryKind::HumanMessage
        | TurnEntryKind::AgentRequest
        | TurnEntryKind::AgentMail
        | TurnEntryKind::ModelResponse => Some("content"),
        TurnEntryKind::RuntimeToolResults => Some("results"),
        _ => None,
    }
}

pub(crate) fn entry_value(row: &EntryRecord, blocks: Vec<Value>) -> Result<Value, Error> {
    let located = |error: Error| error.with_id(row.id().as_uuid());
    let mut object = row
        .payload()
        .as_object()
        .cloned()
        .ok_or_else(|| located(Error::new(ErrorCode::InvalidRecord).at("payload")))?;
    object.insert("type".into(), Value::String(row.kind().as_str().into()));
    object.insert("id".into(), encode_value(&row.id())?);
    if let (Some(key), Some(sender)) = (sender_key(row.kind()), row.sender()) {
        let id = match sender {
            ActorRef::Human { id } => encode_value(id)?,
            ActorRef::Agent { id } => encode_value(id)?,
        };
        object.insert(key.into(), id);
    }
    if let Some(author) = row.author() {
        object.insert("author".into(), Value::String(author.as_str().into()));
    }
    match block_key(row.kind()) {
        Some(key) => {
            object.insert(key.into(), Value::Array(blocks));
        }
        None if !blocks.is_empty() => {
            return Err(located(Error::new(ErrorCode::DisallowedBlockKind)));
        }
        None => {}
    }
    Ok(Value::Object(object))
}
