use uuid::Uuid;

use super::output::{Output, Progress, accumulated_text, usable_block};
use crate::block::{ContentBlockKind, PlainBlock};
use crate::entry::ModelResponse;
use crate::error::Error;
use crate::id::{EntryId, TurnId};
use crate::stream::{DraftPhase, ResponseDraft};
use crate::turn::places_of_response;

#[derive(Debug, Clone, Copy)]
enum Pending<'a> {
    Draft(&'a ResponseDraft),
    Response {
        turn: TurnId,
        response: &'a ModelResponse,
    },
}

#[derive(Debug, Clone, Copy)]
pub struct PendingResponse<'a>(Pending<'a>);

impl<'a> PendingResponse<'a> {
    pub fn response(turn: TurnId, response: &'a ModelResponse) -> Self {
        PendingResponse(Pending::Response { turn, response })
    }

    pub(crate) fn turn(&self) -> TurnId {
        match self.0 {
            Pending::Draft(draft) => draft.turn(),
            Pending::Response { turn, .. } => turn,
        }
    }

    pub(crate) fn entry(&self) -> EntryId {
        match self.0 {
            Pending::Draft(draft) => draft.entry(),
            Pending::Response { response, .. } => response.id(),
        }
    }

    pub(crate) fn identities(&self) -> Vec<Uuid> {
        match self.0 {
            Pending::Draft(draft) => draft.identities(),
            Pending::Response { response, .. } => places_of_response(response)
                .into_iter()
                .map(|(id, _)| id)
                .collect(),
        }
    }

    pub(crate) fn output(&self) -> Result<Option<Output>, Error> {
        match self.0 {
            Pending::Response { response, .. } => Ok(Some(Output::of_response(response))),
            Pending::Draft(draft) => match draft.phase() {
                DraftPhase::Completed { .. } => Ok(draft.response().map(Output::of_response)),
                DraftPhase::Producing { .. } => Ok(Some(Output::new(
                    draft.entry(),
                    Progress::Producing,
                    producing_blocks(draft)?,
                ))),
                DraftPhase::Unstarted { .. } | DraftPhase::Interrupted { .. } => Ok(None),
            },
        }
    }
}

impl<'a> From<&'a ResponseDraft> for PendingResponse<'a> {
    fn from(draft: &'a ResponseDraft) -> Self {
        PendingResponse(Pending::Draft(draft))
    }
}

fn producing_blocks(draft: &ResponseDraft) -> Result<Vec<PlainBlock>, Error> {
    let mut blocks = Vec::new();
    for slot in draft.view().slots() {
        match (slot.finalized(), slot.kind(), slot.text()) {
            (Some(block), _, _) => blocks.extend(usable_block(block)),
            (None, ContentBlockKind::Text, Some(text)) => {
                blocks.extend(accumulated_text(slot.block(), text)?.map(PlainBlock::Text));
            }
            (None, _, _) => {}
        }
    }
    Ok(blocks)
}
