use serde::{Deserialize, Serialize};

use super::{locate, refuse_author};
use crate::block::IdSink;
use crate::error::{Error, ErrorCode};
use crate::id::EntryId;
use crate::value::{ObservedAt, PlainText};

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    try_from = "ClosureReasonWire",
    tag = "type",
    rename_all = "snake_case"
)]
pub enum ClosureReason {
    #[non_exhaustive]
    Completed {
        #[serde(skip_serializing_if = "Option::is_none")]
        detail: Option<PlainText>,
    },
    #[non_exhaustive]
    Cancelled {
        #[serde(skip_serializing_if = "Option::is_none")]
        detail: Option<PlainText>,
    },
    #[non_exhaustive]
    Interrupted {
        #[serde(skip_serializing_if = "Option::is_none")]
        detail: Option<PlainText>,
    },
    #[non_exhaustive]
    BudgetExhausted {
        #[serde(skip_serializing_if = "Option::is_none")]
        detail: Option<PlainText>,
    },
    #[non_exhaustive]
    Failed { detail: PlainText },
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
enum ClosureReasonWire {
    Completed { detail: Option<String> },
    Cancelled { detail: Option<String> },
    Interrupted { detail: Option<String> },
    BudgetExhausted { detail: Option<String> },
    Failed { detail: Option<String> },
}

fn detail_at(text: String) -> Result<PlainText, Error> {
    PlainText::new(text).map_err(|error| error.at("detail"))
}

impl ClosureReason {
    pub fn completed() -> Self {
        ClosureReason::Completed { detail: None }
    }

    pub fn cancelled() -> Self {
        ClosureReason::Cancelled { detail: None }
    }

    pub fn interrupted() -> Self {
        ClosureReason::Interrupted { detail: None }
    }

    pub fn budget_exhausted() -> Self {
        ClosureReason::BudgetExhausted { detail: None }
    }

    pub fn failed(detail: impl Into<String>) -> Result<Self, Error> {
        Ok(ClosureReason::Failed {
            detail: detail_at(detail.into())?,
        })
    }

    pub fn with_detail(self, detail: impl Into<String>) -> Result<Self, Error> {
        let detail = detail_at(detail.into())?;
        Ok(match self {
            ClosureReason::Completed { .. } => ClosureReason::Completed {
                detail: Some(detail),
            },
            ClosureReason::Cancelled { .. } => ClosureReason::Cancelled {
                detail: Some(detail),
            },
            ClosureReason::Interrupted { .. } => ClosureReason::Interrupted {
                detail: Some(detail),
            },
            ClosureReason::BudgetExhausted { .. } => ClosureReason::BudgetExhausted {
                detail: Some(detail),
            },
            ClosureReason::Failed { .. } => ClosureReason::Failed { detail },
        })
    }

    pub fn detail(&self) -> Option<&str> {
        match self {
            ClosureReason::Completed { detail }
            | ClosureReason::Cancelled { detail }
            | ClosureReason::Interrupted { detail }
            | ClosureReason::BudgetExhausted { detail } => detail.as_ref().map(PlainText::as_str),
            ClosureReason::Failed { detail } => Some(detail.as_str()),
        }
    }
}

impl TryFrom<ClosureReasonWire> for ClosureReason {
    type Error = Error;

    fn try_from(wire: ClosureReasonWire) -> Result<Self, Error> {
        let (bare, detail) = match wire {
            ClosureReasonWire::Completed { detail } => (ClosureReason::completed(), detail),
            ClosureReasonWire::Cancelled { detail } => (ClosureReason::cancelled(), detail),
            ClosureReasonWire::Interrupted { detail } => (ClosureReason::interrupted(), detail),
            ClosureReasonWire::BudgetExhausted { detail } => {
                (ClosureReason::budget_exhausted(), detail)
            }
            ClosureReasonWire::Failed { detail } => {
                let detail =
                    detail.ok_or_else(|| Error::new(ErrorCode::InvalidContent).at("detail"))?;
                return ClosureReason::failed(detail);
            }
        };
        match detail {
            Some(detail) => bare.with_detail(detail),
            None => Ok(bare),
        }
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "TurnClosureWire")]
pub struct TurnClosure {
    id: EntryId,
    reason: ClosureReason,
    #[serde(skip_serializing_if = "Option::is_none")]
    observed_at: Option<ObservedAt>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TurnClosureWire {
    id: EntryId,
    reason: ClosureReason,
    #[serde(default, deserialize_with = "super::present")]
    author: bool,
    observed_at: Option<ObservedAt>,
}

impl TurnClosure {
    pub fn new(id: EntryId, reason: ClosureReason) -> Self {
        TurnClosure {
            id,
            reason,
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

    pub fn reason(&self) -> &ClosureReason {
        &self.reason
    }

    pub fn observed_at(&self) -> Option<&ObservedAt> {
        self.observed_at.as_ref()
    }

    pub(crate) fn collect_ids(&self, sink: &mut IdSink) -> Result<(), Error> {
        sink.insert(self.id.as_uuid())
    }
}

impl TryFrom<TurnClosureWire> for TurnClosure {
    type Error = Error;

    fn try_from(wire: TurnClosureWire) -> Result<Self, Error> {
        refuse_author(wire.author, wire.id).map_err(|error| locate(error, wire.id))?;
        let closure = TurnClosure::new(wire.id, wire.reason);
        Ok(match wire.observed_at {
            Some(observed_at) => closure.with_observed_at(observed_at),
            None => closure,
        })
    }
}
