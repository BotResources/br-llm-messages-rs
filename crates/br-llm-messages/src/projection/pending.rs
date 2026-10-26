use std::collections::{HashMap, HashSet};

use uuid::Uuid;

use super::perspective::{accumulated_text, blocks_of, built, usable_block};
use crate::block::{ContentBlockKind, PlainBlock};
use crate::entry::{AgentOutput, IncomingEntry, ModelResponse, PartialAgentOutput};
use crate::error::{Error, ErrorCode};
use crate::id::{AgentId, EntryId, TurnId};
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

pub(crate) fn output_of(
    agent: AgentId,
    author: &Author,
    response: &ModelResponse,
) -> Option<IncomingEntry> {
    built(blocks_of(response.blocks()), |blocks| {
        AgentOutput::from_content(response.id(), agent, author.clone(), blocks)
    })
    .map(IncomingEntry::AgentOutput)
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
    let (agent, author) = owner;
    match pending {
        PendingResponse::Response { response, .. } => output_of(*agent, author, response),
        PendingResponse::Draft(draft) => match draft.phase() {
            DraftPhase::Completed { .. } => draft
                .response()
                .and_then(|response| output_of(*agent, author, response)),
            DraftPhase::Producing { .. } => built(producing_blocks(draft), |blocks| {
                PartialAgentOutput::from_content(draft.entry(), *agent, author.clone(), blocks)
            })
            .map(IncomingEntry::PartialAgentOutput),
            _ => None,
        },
    }
}

pub(crate) fn pending_entries(
    target: AgentId,
    pending: &[PendingResponse<'_>],
    owner_of: impl Fn(TurnId) -> Option<Owner>,
    taken: &HashSet<Uuid>,
) -> Result<HashMap<TurnId, Vec<IncomingEntry>>, Error> {
    let mut entries: HashMap<TurnId, Vec<IncomingEntry>> = HashMap::new();
    let mut seen: HashSet<Uuid> = HashSet::new();
    for item in pending {
        let refused = |code| Error::new(code).with_id(item.entry().as_uuid());
        let Some(owner) = owner_of(item.turn()) else {
            return Err(refused(ErrorCode::UnknownReference));
        };
        if owner.0 == target {
            return Err(refused(ErrorCode::DisallowedKind));
        }
        if taken.contains(&item.entry().as_uuid()) || !seen.insert(item.entry().as_uuid()) {
            return Err(refused(ErrorCode::ReusedIdentity));
        }
        if let Some(entry) = entry_of(item, &owner) {
            entries.entry(item.turn()).or_default().push(entry);
        }
    }
    Ok(entries)
}
