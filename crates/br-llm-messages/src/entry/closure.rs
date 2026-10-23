use serde::{Deserialize, Serialize};

use super::{locate, refuse_author};
use crate::block::IdSink;
use crate::error::{Error, ErrorCode};
use crate::id::EntryId;
use crate::value::FailureDetail;

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BudgetKind {
    Tokens,
    ToolCalls,
    Responses,
    WallClock,
    Cost,
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    try_from = "ClosureReasonWire",
    tag = "type",
    rename_all = "snake_case"
)]
pub enum ClosureReason {
    Completed,
    #[non_exhaustive]
    Cancelled {
        #[serde(skip_serializing_if = "Option::is_none")]
        detail: Option<String>,
    },
    #[non_exhaustive]
    Interrupted {
        detail: FailureDetail,
    },
    #[non_exhaustive]
    BudgetExhausted {
        budget: BudgetKind,
    },
    #[non_exhaustive]
    Failed {
        failure: FailureDetail,
    },
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
enum ClosureReasonWire {
    Completed {},
    Cancelled { detail: Option<String> },
    Interrupted { detail: FailureDetail },
    BudgetExhausted { budget: BudgetKind },
    Failed { failure: FailureDetail },
}

impl ClosureReason {
    pub fn completed() -> Self {
        ClosureReason::Completed
    }

    pub fn cancelled(detail: Option<String>) -> Result<Self, Error> {
        if detail.as_deref() == Some("") {
            return Err(Error::new(ErrorCode::InvalidContent).at("detail"));
        }
        Ok(ClosureReason::Cancelled { detail })
    }

    pub fn interrupted(detail: FailureDetail) -> Self {
        ClosureReason::Interrupted { detail }
    }

    pub fn budget_exhausted(budget: BudgetKind) -> Self {
        ClosureReason::BudgetExhausted { budget }
    }

    pub fn failed(failure: FailureDetail) -> Self {
        ClosureReason::Failed { failure }
    }
}

impl TryFrom<ClosureReasonWire> for ClosureReason {
    type Error = Error;

    fn try_from(wire: ClosureReasonWire) -> Result<Self, Error> {
        match wire {
            ClosureReasonWire::Completed {} => Ok(ClosureReason::completed()),
            ClosureReasonWire::Cancelled { detail } => ClosureReason::cancelled(detail),
            ClosureReasonWire::Interrupted { detail } => Ok(ClosureReason::interrupted(detail)),
            ClosureReasonWire::BudgetExhausted { budget } => {
                Ok(ClosureReason::budget_exhausted(budget))
            }
            ClosureReasonWire::Failed { failure } => Ok(ClosureReason::failed(failure)),
        }
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "TurnClosureWire")]
pub struct TurnClosure {
    id: EntryId,
    reason: ClosureReason,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TurnClosureWire {
    id: EntryId,
    reason: ClosureReason,
    #[serde(default, deserialize_with = "super::present")]
    author: bool,
}

impl TurnClosure {
    pub fn new(id: EntryId, reason: ClosureReason) -> Self {
        TurnClosure { id, reason }
    }

    pub fn id(&self) -> EntryId {
        self.id
    }

    pub fn reason(&self) -> &ClosureReason {
        &self.reason
    }

    pub(crate) fn collect_ids(&self, sink: &mut IdSink) -> Result<(), Error> {
        sink.insert(self.id.as_uuid())
    }
}

impl TryFrom<TurnClosureWire> for TurnClosure {
    type Error = Error;

    fn try_from(wire: TurnClosureWire) -> Result<Self, Error> {
        refuse_author(wire.author, wire.id).map_err(|error| locate(error, wire.id))?;
        Ok(TurnClosure::new(wire.id, wire.reason))
    }
}
