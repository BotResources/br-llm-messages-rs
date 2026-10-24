use serde_json::{Map, Value};

use super::block::BlockRecord;
use super::entry::EntryRecord;
use super::kinds::ActorRef;
use crate::block::{ContentBlock, ContentBlockKind};
use crate::entry::{TurnEntry, TurnEntryKind};
use crate::error::{Error, ErrorCode};
use crate::id::BlockId;
use crate::json::encode_value;
use crate::opaque_json::OpaqueJson;
use crate::value::{Author, ObservedAt};

fn into_object(value: Value) -> Result<Map<String, Value>, Error> {
    match value {
        Value::Object(object) => Ok(object),
        _ => Err(Error::new(ErrorCode::InvalidContent)),
    }
}

pub(crate) fn call_of(block: &ContentBlock) -> Option<BlockId> {
    match block {
        ContentBlock::RuntimeToolResult(result) => Some(result.call()),
        ContentBlock::ProviderToolResult(result) => Some(result.call()),
        _ => None,
    }
}

pub(crate) fn block_payload(block: &ContentBlock) -> Result<OpaqueJson, Error> {
    let mut object = into_object(encode_value(block)?)?;
    for key in ["type", "id", "call", "observed_at"] {
        object.remove(key);
    }
    match block {
        ContentBlock::RuntimeToolResult(_)
        | ContentBlock::ProviderToolResult(_)
        | ContentBlock::ProviderToolExecutionRecord(_) => {
            object.remove("content");
        }
        _ => {}
    }
    Ok(OpaqueJson::from_inner(Value::Object(object)))
}

pub(crate) fn block_value(row: &BlockRecord, children: Vec<Value>) -> Result<Value, Error> {
    let located = |error: Error| error.with_id(row.id().as_uuid());
    let mut object = row
        .payload()
        .inner()
        .as_object()
        .cloned()
        .ok_or_else(|| located(Error::new(ErrorCode::InvalidContent).at("payload")))?;
    object.insert("type".into(), Value::String(row.kind().as_str().into()));
    object.insert("id".into(), encode_value(&row.id())?);
    if let Some(call) = row.call() {
        object.insert("call".into(), encode_value(&call)?);
    }
    if let Some(observed_at) = row.observed_at() {
        object.insert("observed_at".into(), encode_value(observed_at)?);
    }
    match row.kind() {
        ContentBlockKind::RuntimeToolResult => {
            object.insert("content".into(), Value::Array(children));
        }
        ContentBlockKind::ProviderToolResult | ContentBlockKind::ProviderToolExecutionRecord => {
            if !children.is_empty() {
                object.insert("content".into(), Value::Array(children));
            }
        }
        _ => {
            if !children.is_empty() {
                return Err(located(Error::new(ErrorCode::DisallowedKind)));
            }
        }
    }
    Ok(Value::Object(object))
}

pub(crate) struct EntryParts {
    pub(crate) payload: OpaqueJson,
    pub(crate) sender: Option<ActorRef>,
    pub(crate) author: Option<Author>,
    pub(crate) observed_at: Option<ObservedAt>,
}

pub(crate) fn split_entry(entry: &TurnEntry) -> Result<EntryParts, Error> {
    let mut object = into_object(encode_value(entry)?)?;
    for key in [
        "type",
        "id",
        "content",
        "results",
        "human",
        "requester",
        "sender",
        "author",
        "observed_at",
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
        payload: OpaqueJson::from_inner(Value::Object(object)),
        sender,
        author,
        observed_at: entry.observed_at().cloned(),
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
        .inner()
        .as_object()
        .cloned()
        .ok_or_else(|| located(Error::new(ErrorCode::InvalidContent).at("payload")))?;
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
    if let Some(observed_at) = row.observed_at() {
        object.insert("observed_at".into(), encode_value(observed_at)?);
    }
    match block_key(row.kind()) {
        Some(key) => {
            object.insert(key.into(), Value::Array(blocks));
        }
        None if !blocks.is_empty() => {
            return Err(located(Error::new(ErrorCode::DisallowedKind)));
        }
        None => {}
    }
    Ok(Value::Object(object))
}
