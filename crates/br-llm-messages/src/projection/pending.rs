use std::collections::{HashMap, HashSet};

use uuid::Uuid;

use super::output::{output_entry, output_of};
use super::perspective::{accumulated_text, usable_block};
use crate::block::{ContentBlockKind, PlainBlock};
use crate::entry::{IncomingEntry, ModelResponse};
use crate::error::{Error, ErrorCode};
use crate::id::{AgentId, ConversationId, EntryId, TurnId};
use crate::stream::{DraftPhase, ResponseDraft};
use crate::value::Author;

pub(crate) type Owner = (AgentId, Author);

#[non_exhaustive]
#[derive(Debug, Clone, Copy)]
pub enum PendingResponse<'a> {
    Draft(&'a ResponseDraft),
    Response {
        turn: TurnId,
        response: &'a ModelResponse,
    },
}

impl<'a> PendingResponse<'a> {
    pub fn draft(draft: &'a ResponseDraft) -> Self {
        PendingResponse::Draft(draft)
    }

    pub fn response(turn: TurnId, response: &'a ModelResponse) -> Self {
        PendingResponse::Response { turn, response }
    }

    pub fn turn(&self) -> TurnId {
        match self {
            PendingResponse::Draft(draft) => draft.turn(),
            PendingResponse::Response { turn, .. } => *turn,
        }
    }

    pub fn entry(&self) -> EntryId {
        match self {
            PendingResponse::Draft(draft) => draft.entry(),
            PendingResponse::Response { response, .. } => response.id(),
        }
    }
}

impl<'a> From<&'a ResponseDraft> for PendingResponse<'a> {
    fn from(draft: &'a ResponseDraft) -> Self {
        PendingResponse::Draft(draft)
    }
}

fn producing_blocks(draft: &ResponseDraft) -> Vec<PlainBlock> {
    draft
        .view()
        .slots()
        .iter()
        .filter_map(|slot| match slot.finalized() {
            Some(block) => usable_block(block),
            None if slot.kind() == ContentBlockKind::Text => {
                accumulated_text(slot.block(), slot.text()?).map(PlainBlock::Text)
            }
            None => None,
        })
        .collect()
}

fn entry_of(pending: &PendingResponse<'_>, owner: &Owner) -> Option<IncomingEntry> {
    match pending {
        PendingResponse::Response { response, .. } => output_of(owner, response),
        PendingResponse::Draft(draft) => match draft.phase() {
            DraftPhase::Completed { .. } => draft
                .response()
                .and_then(|response| output_of(owner, response)),
            DraftPhase::Producing { .. } => {
                output_entry(draft.entry(), owner, false, producing_blocks(draft))
                    .ok()
                    .flatten()
            }
            _ => None,
        },
    }
}

pub(crate) fn pending_entries(
    target: AgentId,
    conversation: ConversationId,
    pending: &[PendingResponse<'_>],
    owners: impl IntoIterator<Item = (TurnId, Owner)>,
    declared: impl IntoIterator<Item = Uuid>,
) -> Result<HashMap<TurnId, Vec<IncomingEntry>>, Error> {
    let owners: HashMap<TurnId, Owner> = owners.into_iter().collect();
    let mut taken: HashSet<Uuid> = HashSet::from([conversation.as_uuid()]);
    taken.extend(declared);
    let mut entries: HashMap<TurnId, Vec<IncomingEntry>> = HashMap::new();
    let mut seen: HashSet<Uuid> = HashSet::new();
    for item in pending {
        let refused = |code| Error::new(code).with_id(item.entry().as_uuid());
        let Some(owner) = owners.get(&item.turn()) else {
            return Err(refused(ErrorCode::UnknownReference));
        };
        if owner.0 == target {
            return Err(refused(ErrorCode::DisallowedKind));
        }
        if taken.contains(&item.entry().as_uuid()) || !seen.insert(item.entry().as_uuid()) {
            return Err(refused(ErrorCode::ReusedIdentity));
        }
        if let Some(entry) = entry_of(item, owner) {
            entries.entry(item.turn()).or_default().push(entry);
        }
    }
    Ok(entries)
}
