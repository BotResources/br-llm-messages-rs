use uuid::Uuid;

use super::output::{Output, Owner};
use crate::entry::IncomingEntry;
use crate::id::TurnId;
use crate::turn::Turn;

pub(crate) enum Item {
    Incoming(IncomingEntry),
    Output(Output),
}

pub(crate) struct Foreign {
    pub(crate) id: TurnId,
    pub(crate) owner: Owner,
    pub(crate) items: Vec<Item>,
}

pub(crate) enum Source {
    Kept(Turn),
    Foreign(Foreign),
}

pub(crate) struct Identity {
    pub(crate) id: Uuid,
    pub(crate) at: String,
}

#[derive(Default)]
pub(crate) struct Sources {
    pub(super) sources: Vec<Source>,
    pub(super) identities: Vec<Identity>,
}

impl Sources {
    pub(crate) fn push(&mut self, source: Source) {
        self.sources.push(source);
    }

    pub(crate) fn declare(&mut self, id: Uuid, at: String) {
        self.identities.push(Identity { id, at });
    }
}

pub(crate) enum Holder<'a> {
    Target,
    Foreign(&'a Owner),
}

impl Source {
    pub(super) fn id(&self) -> TurnId {
        match self {
            Source::Kept(turn) => turn.id(),
            Source::Foreign(foreign) => foreign.id,
        }
    }

    pub(super) fn holder(&self) -> Option<Holder<'_>> {
        match self {
            Source::Kept(Turn::AITurn(_)) => Some(Holder::Target),
            Source::Kept(Turn::InputTurn(_)) => None,
            Source::Foreign(foreign) => Some(Holder::Foreign(&foreign.owner)),
        }
    }
}
