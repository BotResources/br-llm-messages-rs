use std::collections::HashSet;

use uuid::Uuid;

use super::declared::{Class, Declared, places_of_turn_entry};
use super::state::Tracker;
use crate::block::{ContentBlock, ContentBlockKind};
use crate::entry::{ClosureReason, ModelResponse, TurnEntry};
use crate::error::{Error, ErrorCode};
use crate::id::EntryId;
use crate::value::ResponseEnding;

pub(crate) fn check(tracker: &Tracker, entry: &TurnEntry, index: usize) -> Result<Declared, Error> {
    let id = entry.id();
    if tracker.closure.is_some() {
        return Err(Error::new(ErrorCode::TurnClosed).with_id(id.as_uuid()));
    }
    let declared = places_of_turn_entry(entry, index);
    check_fresh(tracker, &declared)?;
    match entry {
        TurnEntry::ModelResponse(response) => check_response(tracker, response, &declared)?,
        TurnEntry::RuntimeToolResults(delivery) => {
            super::results::check(tracker, delivery.results())?
        }
        TurnEntry::ToolUnmounted(unmounted) => super::mounting::check_unmount(tracker, unmounted)?,
        TurnEntry::TurnClosure(closure) => check_closure(tracker, closure.id(), closure.reason())?,
        _ => {}
    }
    Ok(declared)
}

fn check_fresh(tracker: &Tracker, declared: &Declared) -> Result<(), Error> {
    let mut seen = HashSet::new();
    for (id, _) in declared {
        if tracker.ids.contains_key(id) || !seen.insert(*id) {
            return Err(Error::new(ErrorCode::ReusedIdentity).with_id(*id));
        }
    }
    Ok(())
}

fn check_response(
    tracker: &Tracker,
    response: &ModelResponse,
    declared: &Declared,
) -> Result<(), Error> {
    if let Some(call) = tracker.outstanding.first() {
        return Err(Error::new(ErrorCode::UnresolvedExecution).with_id(call.as_uuid()));
    }
    for (index, block) in response.blocks().iter().enumerate() {
        let ContentBlock::ProviderToolResult(result) = block else {
            continue;
        };
        let target = result.call().as_uuid();
        let located = |code| {
            Error::new(code)
                .with_id(target)
                .at(format!("content[{index}].call"))
        };
        match class_of(tracker, declared, target) {
            None => return Err(located(ErrorCode::UnknownReference)),
            Some(Class::Block(ContentBlockKind::ProviderToolCall)) => {}
            Some(_) => return Err(located(ErrorCode::WrongKindReference)),
        }
    }
    Ok(())
}

fn class_of(tracker: &Tracker, declared: &Declared, id: Uuid) -> Option<Class> {
    tracker.ids.get(&id).map(|place| place.class).or_else(|| {
        declared
            .iter()
            .find(|(candidate, _)| *candidate == id)
            .map(|(_, place)| place.class)
    })
}

fn check_closure(tracker: &Tracker, id: EntryId, reason: &ClosureReason) -> Result<(), Error> {
    if !matches!(reason, ClosureReason::Completed) {
        return Ok(());
    }
    let unresolved = tracker
        .outstanding
        .first()
        .or_else(|| tracker.pending.first());
    if let Some(block) = unresolved {
        return Err(Error::new(ErrorCode::UnresolvedExecution)
            .with_id(block.as_uuid())
            .at("reason"));
    }
    if tracker.last_ending == Some(ResponseEnding::ProviderContinuation) {
        return Err(Error::new(ErrorCode::ContinuationPending)
            .with_id(id.as_uuid())
            .at("reason"));
    }
    Ok(())
}
