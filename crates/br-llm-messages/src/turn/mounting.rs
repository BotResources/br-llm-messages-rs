use super::state::Tracker;
use crate::entry::ToolUnmounted;
use crate::error::{Error, ErrorCode};

pub(crate) fn check_unmount(tracker: &Tracker, unmounted: &ToolUnmounted) -> Result<(), Error> {
    let target = unmounted.mounting();
    let fail = |code| Error::new(code).with_id(target.as_uuid()).at("mounting");
    match tracker.mountings.get(&target) {
        Some(true) => Ok(()),
        Some(false) => Err(fail(ErrorCode::InvalidContent)),
        None if tracker.ids.contains_key(&target.as_uuid()) => {
            Err(fail(ErrorCode::WrongKindReference))
        }
        None => Err(fail(ErrorCode::UnknownReference)),
    }
}
