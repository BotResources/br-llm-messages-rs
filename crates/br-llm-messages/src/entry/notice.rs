use serde::{Deserialize, Serialize};

use super::refuse_author;
use crate::error::{Error, ErrorCode};
use crate::id::{EntryId, RequestId};
use crate::value::ObservedAt;

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "RequestAcceptedWire")]
pub struct RequestAccepted {
    id: EntryId,
    request: RequestId,
    #[serde(skip_serializing_if = "Option::is_none")]
    observed_at: Option<ObservedAt>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RequestAcceptedWire {
    id: EntryId,
    request: RequestId,
    #[serde(default, deserialize_with = "super::present")]
    author: bool,
    observed_at: Option<ObservedAt>,
}

impl RequestAccepted {
    pub fn new(id: EntryId, request: RequestId) -> Self {
        RequestAccepted {
            id,
            request,
            observed_at: None,
        }
    }

    pub fn with_observed_at(mut self, observed_at: ObservedAt) -> Self {
        self.observed_at = Some(observed_at);
        self
    }

    pub fn id(&self) -> EntryId {
        self.id
    }

    pub fn request(&self) -> RequestId {
        self.request
    }

    pub fn observed_at(&self) -> Option<&ObservedAt> {
        self.observed_at.as_ref()
    }
}

impl TryFrom<RequestAcceptedWire> for RequestAccepted {
    type Error = Error;

    fn try_from(wire: RequestAcceptedWire) -> Result<Self, Error> {
        refuse_author(wire.author, wire.id)?;
        let accepted = RequestAccepted::new(wire.id, wire.request);
        Ok(match wire.observed_at {
            Some(observed_at) => accepted.with_observed_at(observed_at),
            None => accepted,
        })
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "QueuePositionWire")]
pub struct QueuePosition {
    id: EntryId,
    request: RequestId,
    position: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    observed_at: Option<ObservedAt>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct QueuePositionWire {
    id: EntryId,
    request: RequestId,
    position: u32,
    #[serde(default, deserialize_with = "super::present")]
    author: bool,
    observed_at: Option<ObservedAt>,
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
            observed_at: None,
        })
    }

    pub fn with_observed_at(mut self, observed_at: ObservedAt) -> Self {
        self.observed_at = Some(observed_at);
        self
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

    pub fn observed_at(&self) -> Option<&ObservedAt> {
        self.observed_at.as_ref()
    }
}

impl TryFrom<QueuePositionWire> for QueuePosition {
    type Error = Error;

    fn try_from(wire: QueuePositionWire) -> Result<Self, Error> {
        refuse_author(wire.author, wire.id)?;
        let queued = QueuePosition::new(wire.id, wire.request, wire.position)?;
        Ok(match wire.observed_at {
            Some(observed_at) => queued.with_observed_at(observed_at),
            None => queued,
        })
    }
}
