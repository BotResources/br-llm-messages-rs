use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::kinds::RepresentationState;
use super::partial;
use super::schema::{as_object, block_fields, check_object};
use crate::block::ContentBlockKind;
use crate::error::{Error, ErrorCode};
use crate::id::{BlockId, EntryId};

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "BlockRecordWire")]
pub struct BlockRecord {
    id: BlockId,
    entry: EntryId,
    #[serde(skip_serializing_if = "Option::is_none")]
    parent: Option<BlockId>,
    kind: ContentBlockKind,
    position: u32,
    state: RepresentationState,
    #[serde(skip_serializing_if = "Option::is_none")]
    call: Option<BlockId>,
    payload: Value,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BlockRecordWire {
    id: BlockId,
    entry: EntryId,
    parent: Option<BlockId>,
    kind: ContentBlockKind,
    position: u32,
    state: RepresentationState,
    call: Option<BlockId>,
    #[serde(deserialize_with = "crate::json::unique_value")]
    payload: Value,
}

impl BlockRecord {
    pub fn new(
        id: BlockId,
        entry: EntryId,
        kind: ContentBlockKind,
        position: u32,
        state: RepresentationState,
        payload: Value,
    ) -> Self {
        BlockRecord {
            id,
            entry,
            parent: None,
            kind,
            position,
            state,
            call: None,
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

    pub fn position(&self) -> u32 {
        self.position
    }

    pub fn state(&self) -> RepresentationState {
        self.state
    }

    pub fn call(&self) -> Option<BlockId> {
        self.call
    }

    pub fn payload(&self) -> &Value {
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
                let object = as_object(&self.payload)?;
                check_object(object, &block_fields(self.kind)).map_err(|error| error.at("payload"))
            }
            RepresentationState::Producing | RepresentationState::Interrupted => {
                partial::check(self.kind, &self.payload)
            }
            RepresentationState::Discarded => {
                let empty = self.payload.as_object().is_some_and(|o| o.is_empty());
                if self.kind != ContentBlockKind::Text || !empty {
                    return Err(Error::new(ErrorCode::InvalidRecord).at("payload"));
                }
                Ok(())
            }
        }
    }

    fn check_call(&self) -> Result<(), Error> {
        let needs_call = matches!(
            self.kind,
            ContentBlockKind::RuntimeToolResult | ContentBlockKind::ProviderToolResult
        );
        match (needs_call, self.call) {
            (true, None) => Err(Error::new(ErrorCode::MissingContent).at("call")),
            (false, Some(_)) => Err(Error::new(ErrorCode::InvalidRecord).at("call")),
            _ => Ok(()),
        }
    }

    fn check_parent(&self) -> Result<(), Error> {
        let Some(parent) = self.parent else {
            return Ok(());
        };
        if parent == self.id {
            return Err(Error::new(ErrorCode::InvalidRecord).at("parent"));
        }
        if !self.kind.is_plain() {
            return Err(Error::new(ErrorCode::DisallowedBlockKind).at("parent"));
        }
        if self.state != RepresentationState::Complete {
            return Err(Error::new(ErrorCode::InvalidRecord).at("state"));
        }
        Ok(())
    }
}

impl TryFrom<BlockRecordWire> for BlockRecord {
    type Error = Error;

    fn try_from(wire: BlockRecordWire) -> Result<Self, Error> {
        let record = BlockRecord {
            id: wire.id,
            entry: wire.entry,
            parent: wire.parent,
            kind: wire.kind,
            position: wire.position,
            state: wire.state,
            call: wire.call,
            payload: wire.payload,
        };
        record.check_local()?;
        Ok(record)
    }
}
