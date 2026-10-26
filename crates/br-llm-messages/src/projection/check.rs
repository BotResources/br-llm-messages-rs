use std::collections::{HashMap, HashSet};

use uuid::Uuid;

use super::ProjectedThread;
use super::pending::PendingResponse;
use super::source::{Holder, Item, Source, Sources};
use crate::entry::IncomingEntry;
use crate::error::{Error, ErrorCode};
use crate::id::{AgentId, ConversationId, TurnId};
use crate::turn::{InputTurn, Turn};

pub(crate) struct Checked {
    sources: Vec<Source>,
    pending: HashMap<TurnId, Vec<IncomingEntry>>,
}

impl Sources {
    pub(crate) fn check(
        self,
        conversation: ConversationId,
        pending: &[PendingResponse<'_>],
    ) -> Result<Checked, Error> {
        let mut seen: HashSet<Uuid> = HashSet::from([conversation.as_uuid()]);
        for identity in &self.identities {
            if !seen.insert(identity.id) {
                return Err(Error::new(ErrorCode::ReusedIdentity)
                    .with_id(identity.id)
                    .at(&identity.at));
            }
        }
        let holders: HashMap<TurnId, Holder<'_>> = self
            .sources
            .iter()
            .filter_map(|source| Some((source.id(), source.holder()?)))
            .collect();
        let mut placed: HashMap<TurnId, Vec<IncomingEntry>> = HashMap::new();
        for (index, item) in pending.iter().enumerate() {
            let at = format!("pending[{index}]");
            let refused = |code| Error::new(code).with_id(item.entry().as_uuid()).at(&at);
            let owner = match holders.get(&item.turn()) {
                None => return Err(refused(ErrorCode::UnknownReference)),
                Some(Holder::Target) => return Err(refused(ErrorCode::DisallowedKind)),
                Some(Holder::Foreign(owner)) => *owner,
            };
            for id in item.identities() {
                if !seen.insert(id) {
                    return Err(Error::new(ErrorCode::ReusedIdentity).with_id(id).at(&at));
                }
            }
            if let Some(entry) = item.output().and_then(|output| output.entry_for(owner)) {
                placed.entry(item.turn()).or_default().push(entry);
            }
        }
        Ok(Checked {
            sources: self.sources,
            pending: placed,
        })
    }
}

impl Checked {
    pub(crate) fn unique(sources: Sources) -> Self {
        Checked {
            sources: sources.sources,
            pending: HashMap::new(),
        }
    }

    pub(crate) fn project(
        mut self,
        conversation: ConversationId,
        agent: AgentId,
    ) -> ProjectedThread {
        let mut turns = Vec::with_capacity(self.sources.len());
        for source in self.sources {
            match source {
                Source::Kept(turn) => turns.push(turn),
                Source::Foreign(foreign) => {
                    let mut entries: Vec<IncomingEntry> = foreign
                        .items
                        .into_iter()
                        .filter_map(|item| match item {
                            Item::Incoming(entry) => Some(entry),
                            Item::Output(output) => output.entry_for(&foreign.owner),
                        })
                        .collect();
                    entries.extend(self.pending.remove(&foreign.id).unwrap_or_default());
                    turns.extend(InputTurn::translated(foreign.id, entries).map(Turn::InputTurn));
                }
            }
        }
        ProjectedThread::unchecked(conversation, agent, turns)
    }
}
