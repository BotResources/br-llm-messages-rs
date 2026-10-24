use serde::{Deserialize, Serialize};

use super::kinds::RepresentationState;
use super::partial;
use super::schema::{as_object, block_fields, check_object};
use crate::block::ContentBlockKind;
use crate::error::{Error, ErrorCode};
use crate::id::{BlockId, EntryId};
use crate::opaque_json::OpaqueJson;
use crate::value::ObservedAt;

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "BlockRecordWire")]
pub struct BlockRecord {
    id: BlockId,
    entry: EntryId,
    #[serde(skip_serializing_if = "Option::is_none")]
    parent: Option<BlockId>,
    kind: ContentBlockKind,
    state: RepresentationState,
    #[serde(skip_serializing_if = "Option::is_none")]
    call: Option<BlockId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    observed_at: Option<ObservedAt>,
    payload: OpaqueJson,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BlockRecordWire {
    id: BlockId,
    entry: EntryId,
    parent: Option<BlockId>,
    kind: ContentBlockKind,
    state: RepresentationState,
    call: Option<BlockId>,
    observed_at: Option<ObservedAt>,
    payload: OpaqueJson,
}

impl BlockRecord {
    pub fn new(
        id: BlockId,
        entry: EntryId,
        kind: ContentBlockKind,
        state: RepresentationState,
        payload: OpaqueJson,
    ) -> Self {
        BlockRecord {
            id,
            entry,
            parent: None,
            kind,
            state,
            call: None,
            observed_at: None,
            payload,
        }
    }

    pub fn with_parent(mut self, parent: BlockId) -> Self {
        self.parent = Some(parent);
        self
    }

    pub fn with_call(mut self, call: BlockId) -> Self {
        self.call = Some(call);
        self
    }

    pub fn with_observed_at(mut self, observed_at: ObservedAt) -> Self {
        self.observed_at = Some(observed_at);
        self
    }

    pub fn id(&self) -> BlockId {
        self.id
    }

    pub fn entry(&self) -> EntryId {
        self.entry
    }

    pub fn parent(&self) -> Option<BlockId> {
        self.parent
    }

    pub fn kind(&self) -> ContentBlockKind {
        self.kind
    }

    pub fn state(&self) -> RepresentationState {
        self.state
    }

    pub fn call(&self) -> Option<BlockId> {
        self.call
    }

    pub fn observed_at(&self) -> Option<&ObservedAt> {
        self.observed_at.as_ref()
    }

    pub fn payload(&self) -> &OpaqueJson {
        &self.payload
    }

    pub fn check_local(&self) -> Result<(), Error> {
        self.check_inner().map_err(|error| match error.object_id() {
            Some(_) => error,
            None => error.with_id(self.id.as_uuid()),
        })
    }

    fn check_inner(&self) -> Result<(), Error> {
        self.check_call()?;
        self.check_parent()?;
        match self.state {
            RepresentationState::Complete => {
                let object = as_object(self.payload.inner())?;
                check_object(object, &block_fields(self.kind))
                    .and_then(|()| check_combination(self.kind, object))
                    .map_err(|error| error.at("payload"))
            }
            RepresentationState::Producing | RepresentationState::Interrupted => {
                partial::check(self.kind, &self.payload)
            }
        }
    }

    fn check_call(&self) -> Result<(), Error> {
        let needs_call = matches!(
            self.kind,
            ContentBlockKind::RuntimeToolResult | ContentBlockKind::ProviderToolResult
        );
        match (needs_call, self.call) {
            (true, None) => Err(Error::new(ErrorCode::InvalidContent).at("call")),
            (false, Some(_)) => Err(Error::new(ErrorCode::DisallowedKind).at("call")),
            _ => Ok(()),
        }
    }

    fn check_parent(&self) -> Result<(), Error> {
        let Some(parent) = self.parent else {
            return Ok(());
        };
        if parent == self.id {
            return Err(Error::new(ErrorCode::InvalidRecordStructure).at("parent"));
        }
        if !self.kind.is_plain() {
            return Err(Error::new(ErrorCode::InvalidRecordStructure).at("parent"));
        }
        if self.state != RepresentationState::Complete {
            return Err(Error::new(ErrorCode::InvalidRecordStructure).at("state"));
        }
        Ok(())
    }
}

fn check_combination(
    kind: ContentBlockKind,
    object: &serde_json::Map<String, serde_json::Value>,
) -> Result<(), Error> {
    let empty_text = object
        .get("text")
        .and_then(serde_json::Value::as_str)
        .is_some_and(str::is_empty);
    if kind == ContentBlockKind::Thinking && empty_text && !object.contains_key("replay") {
        return Err(Error::new(ErrorCode::InvalidContent).at("text"));
    }
    Ok(())
}

impl TryFrom<BlockRecordWire> for BlockRecord {
    type Error = Error;

    fn try_from(wire: BlockRecordWire) -> Result<Self, Error> {
        let record = BlockRecord {
            id: wire.id,
            entry: wire.entry,
            parent: wire.parent,
            kind: wire.kind,
            state: wire.state,
            call: wire.call,
            observed_at: wire.observed_at,
            payload: wire.payload,
        };
        record.check_local()?;
        Ok(record)
    }
}
