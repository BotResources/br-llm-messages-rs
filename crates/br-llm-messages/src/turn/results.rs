use std::collections::HashSet;

use super::declared::Class;
use super::state::Tracker;
use crate::block::{ContentBlockKind, RuntimeToolResult};
use crate::error::{Error, ErrorCode};

pub(crate) fn check(tracker: &Tracker, results: &[RuntimeToolResult]) -> Result<(), Error> {
    let mut delivered = HashSet::new();
    for (index, result) in results.iter().enumerate() {
        let call = result.call();
        let fail = |code| {
            Error::new(code)
                .with_id(call.as_uuid())
                .at(format!("results[{index}]"))
        };
        match tracker.ids.get(&call.as_uuid()).map(|place| place.class) {
            None => return Err(fail(ErrorCode::UnknownReference)),
            Some(Class::Block(ContentBlockKind::RuntimeToolCall)) => {}
            Some(_) => return Err(fail(ErrorCode::WrongKindReference)),
        }
        if tracker.answered.contains(&call) || !delivered.insert(call) {
            return Err(fail(ErrorCode::DuplicateToolResult));
        }
        if tracker.calls.get(&call) != Some(result.tool_name()) {
            return Err(fail(ErrorCode::ToolResultMismatch));
        }
    }
    Ok(())
}
