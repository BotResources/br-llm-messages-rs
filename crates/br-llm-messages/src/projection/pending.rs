use uuid::Uuid;

use super::output::{Output, Progress, accumulated_text, usable_block};
use crate::block::{ContentBlockKind, PlainBlock};
use crate::entry::ModelResponse;
use crate::id::{EntryId, TurnId};
use crate::stream::{DraftPhase, ResponseDraft};
use crate::turn::places_of_response;

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
    pub fn response(turn: TurnId, response: &'a ModelResponse) -> Self {
        PendingResponse::Response { turn, response }
    }

    pub(crate) fn turn(&self) -> TurnId {
        match self {
            PendingResponse::Draft(draft) => draft.turn(),
            PendingResponse::Response { turn, .. } => *turn,
        }
    }

    pub(crate) fn entry(&self) -> EntryId {
        match self {
            PendingResponse::Draft(draft) => draft.entry(),
            PendingResponse::Response { response, .. } => response.id(),
        }
    }

    pub(crate) fn identities(&self) -> Vec<Uuid> {
        match self {
            PendingResponse::Draft(draft) => std::iter::once(draft.entry().as_uuid())
                .chain(
                    draft
                        .view()
                        .slots()
                        .iter()
                        .map(|slot| slot.block().as_uuid()),
                )
                .collect(),
            PendingResponse::Response { response, .. } => places_of_response(response)
                .into_iter()
                .map(|(id, _)| id)
                .collect(),
        }
    }

    pub(crate) fn output(&self) -> Option<Output> {
        match self {
            PendingResponse::Response { response, .. } => Some(Output::of_response(response)),
            PendingResponse::Draft(draft) => match draft.phase() {
                DraftPhase::Completed { .. } => draft.response().map(Output::of_response),
                DraftPhase::Producing { .. } => Some(Output::new(
                    draft.entry(),
                    Progress::Producing,
                    producing_blocks(draft),
                )),
                DraftPhase::Unstarted { .. } | DraftPhase::Interrupted { .. } => None,
            },
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
