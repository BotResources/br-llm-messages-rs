use super::state::Tracker;
use crate::entry::TurnEntry;
use crate::error::{Error, ErrorCode};

pub(crate) fn check(tracker: &Tracker, entry: &TurnEntry) -> Result<(), Error> {
    let answers_calls = matches!(entry, TurnEntry::RuntimeToolResults(_));
    let code = match (tracker.closure.is_some(), answers_calls) {
        (true, false) => ErrorCode::ClosedTurn,
        (true, true) if tracker.outstanding.is_empty() => ErrorCode::ClosedTurn,
        (false, true) if tracker.outstanding.is_empty() => ErrorCode::EntryOutOfOrder,
        _ => return Ok(()),
    };
    Err(Error::new(code).with_id(entry.id().as_uuid()))
}
