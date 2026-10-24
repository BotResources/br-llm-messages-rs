use super::state::Tracker;
use crate::entry::{ClosureReason, TurnEntry, TurnEntryKind};
use crate::error::{Error, ErrorCode};
use crate::value::ResponseEnding;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Next {
    Incoming,
    Response,
    Results,
    AbnormalClosure,
    CompletedClosure,
}

impl Next {
    fn of(entry: &TurnEntry) -> Next {
        match entry {
            TurnEntry::ModelResponse(_) => Next::Response,
            TurnEntry::RuntimeToolResults(_) => Next::Results,
            TurnEntry::TurnClosure(closure) => match closure.reason() {
                ClosureReason::Completed { .. } => Next::CompletedClosure,
                _ => Next::AbnormalClosure,
            },
            _ => Next::Incoming,
        }
    }
}

pub(crate) fn check(tracker: &Tracker, entry: &TurnEntry) -> Result<(), Error> {
    if accepted(tracker, Next::of(entry)) {
        Ok(())
    } else {
        Err(Error::new(ErrorCode::EntryOutOfOrder).with_id(entry.id().as_uuid()))
    }
}

fn accepted(tracker: &Tracker, next: Next) -> bool {
    use Next::{AbnormalClosure, CompletedClosure, Incoming, Response, Results};
    let abnormal_or = |allowed: Next| next == allowed || next == AbnormalClosure;
    let anything_but_results = matches!(
        next,
        Incoming | Response | CompletedClosure | AbnormalClosure
    );
    let closes = matches!(next, CompletedClosure | AbnormalClosure);
    let continuing = tracker.last_ending == Some(ResponseEnding::provider_continuation())
        || !tracker.pending.is_empty();
    match tracker.last_entry {
        None => anything_but_results,
        Some(kind) if kind.is_incoming() => anything_but_results,
        Some(TurnEntryKind::ModelResponse | TurnEntryKind::RuntimeToolResults)
            if !tracker.outstanding.is_empty() =>
        {
            abnormal_or(Results)
        }
        Some(TurnEntryKind::ModelResponse | TurnEntryKind::RuntimeToolResults) if continuing => {
            abnormal_or(Response)
        }
        Some(TurnEntryKind::ModelResponse)
            if tracker.last_ending != Some(ResponseEnding::output_exhaustion()) =>
        {
            next == Incoming || closes
        }
        Some(TurnEntryKind::ModelResponse | TurnEntryKind::RuntimeToolResults) => {
            anything_but_results
        }
        Some(_) => false,
    }
}
