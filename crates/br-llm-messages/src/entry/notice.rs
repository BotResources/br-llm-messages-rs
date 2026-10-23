use serde::{Deserialize, Serialize};

use super::refuse_author;
use crate::block::IdSink;
use crate::error::{Error, ErrorCode};
use crate::id::{EntryId, RequestId};

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "RequestAcceptedWire")]
pub struct RequestAccepted {
    id: EntryId,
    request: RequestId,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RequestAcceptedWire {
    id: EntryId,
    request: RequestId,
    #[serde(default, deserialize_with = "super::present")]
    author: bool,
}

impl RequestAccepted {
    pub fn new(id: EntryId, request: RequestId) -> Self {
        RequestAccepted { id, request }
    }

    pub fn id(&self) -> EntryId {
        self.id
    }

    pub fn request(&self) -> RequestId {
        self.request
    }

    pub(crate) fn collect_ids(&self, sink: &mut IdSink) -> Result<(), Error> {
        sink.insert(self.id.as_uuid())
    }
}

impl TryFrom<RequestAcceptedWire> for RequestAccepted {
    type Error = Error;

    fn try_from(wire: RequestAcceptedWire) -> Result<Self, Error> {
        refuse_author(wire.author, wire.id)?;
        Ok(RequestAccepted::new(wire.id, wire.request))
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "QueuePositionWire")]
pub struct QueuePosition {
    id: EntryId,
    request: RequestId,
    position: u32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct QueuePositionWire {
    id: EntryId,
    request: RequestId,
    position: u32,
    #[serde(default, deserialize_with = "super::present")]
    author: bool,
}

impl QueuePosition {
    pub fn new(id: EntryId, request: RequestId, position: u32) -> Result<Self, Error> {
        if position == 0 {
            return Err(Error::new(ErrorCode::InvalidContent)
                .with_id(id.as_uuid())
                .at("position"));
        }
        Ok(QueuePosition {
            id,
            request,
            position,
        })
    }

    pub fn id(&self) -> EntryId {
        self.id
    }

    pub fn request(&self) -> RequestId {
        self.request
    }

    pub fn position(&self) -> u32 {
        self.position
    }

    pub(crate) fn collect_ids(&self, sink: &mut IdSink) -> Result<(), Error> {
        sink.insert(self.id.as_uuid())
    }
}

impl TryFrom<QueuePositionWire> for QueuePosition {
    type Error = Error;

    fn try_from(wire: QueuePositionWire) -> Result<Self, Error> {
        refuse_author(wire.author, wire.id)?;
        QueuePosition::new(wire.id, wire.request, wire.position)
    }
}
