use std::collections::HashSet;

use super::kinds::RepresentationState;
use super::reconstruct::rebuild_without;
use super::reduced::rebuild_thread;
use super::set::RecordSet;
use crate::conversation::Conversation;
use crate::entry::TurnEntryKind;
use crate::error::{Error, ErrorCode};
use crate::id::{AgentId, ConversationId, EntryId, TurnId};
use crate::projection::ProjectedThread;
use crate::stream::DraftSnapshot;
use crate::turn::TurnKind;

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unfinished {
    Restore,
    Drop,
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub struct Recovered<History> {
    history: History,
    drafts: Vec<DraftSnapshot>,
    interrupted: Vec<DraftSnapshot>,
}

impl<History> Recovered<History> {
    pub fn history(&self) -> &History {
        &self.history
    }

    pub fn drafts(&self) -> &[DraftSnapshot] {
        &self.drafts
    }

    pub fn interrupted(&self) -> &[DraftSnapshot] {
        &self.interrupted
    }

    pub fn into_parts(self) -> (History, Vec<DraftSnapshot>, Vec<DraftSnapshot>) {
        (self.history, self.drafts, self.interrupted)
    }

    fn separate(history: History, snapshots: Vec<DraftSnapshot>) -> Self {
        let (drafts, interrupted) = snapshots
            .into_iter()
            .partition(|snapshot| snapshot.entry().state() == RepresentationState::Producing);
        Recovered {
            history,
            drafts,
            interrupted,
        }
    }
}

fn own_turns(set: &RecordSet, agent: Option<AgentId>) -> HashSet<TurnId> {
    set.turns()
        .iter()
        .filter(|row| row.kind() == TurnKind::AITurn)
        .filter(|row| agent.is_none() || row.agent() == agent)
        .map(|row| row.id())
        .collect()
}

fn unfinished_entries(set: &RecordSet, turns: &HashSet<TurnId>) -> HashSet<EntryId> {
    set.entries()
        .iter()
        .filter(|row| {
            row.kind() == TurnEntryKind::ModelResponse
                && row.state() != RepresentationState::Complete
                && turns.contains(&row.turn())
        })
        .map(|row| row.id())
        .collect()
}

fn snapshots_of(
    set: &RecordSet,
    unfinished: &HashSet<EntryId>,
) -> Result<Vec<DraftSnapshot>, Error> {
    let mut snapshots = Vec::with_capacity(unfinished.len());
    for (index, row) in set.entries().iter().enumerate() {
        if !unfinished.contains(&row.id()) {
            continue;
        }
        let blocks = set
            .blocks()
            .iter()
            .filter(|block| block.entry() == row.id())
            .cloned()
            .collect();
        let snapshot = DraftSnapshot::new(row.clone(), blocks)
            .map_err(|error| error.at(format!("entries[{index}]")))?;
        snapshots.push(snapshot);
    }
    Ok(snapshots)
}

impl RecordSet {
    pub fn recover_conversation(
        &self,
        id: ConversationId,
        unfinished: Unfinished,
    ) -> Result<Recovered<Conversation>, Error> {
        let skipped = unfinished_entries(self, &own_turns(self, None));
        let turns = rebuild_without(self, &skipped)?;
        let history = Conversation::from_parts(id, turns)?;
        let snapshots = match unfinished {
            Unfinished::Restore => snapshots_of(self, &skipped)?,
            Unfinished::Drop => Vec::new(),
        };
        for snapshot in &snapshots {
            ensure_new_identities(&history, snapshot)?;
        }
        Ok(Recovered::separate(history, snapshots))
    }

    pub fn recover_projected_thread(
        &self,
        conversation: ConversationId,
        agent: AgentId,
        unfinished: Unfinished,
    ) -> Result<Recovered<ProjectedThread>, Error> {
        let history = rebuild_thread(self, conversation, agent, &[], Some(unfinished))?;
        let snapshots = match unfinished {
            Unfinished::Restore => snapshots_of(
                self,
                &unfinished_entries(self, &own_turns(self, Some(agent))),
            )?,
            Unfinished::Drop => Vec::new(),
        };
        Ok(Recovered::separate(history, snapshots))
    }
}

fn ensure_new_identities(history: &Conversation, snapshot: &DraftSnapshot) -> Result<(), Error> {
    let draft = crate::stream::ResponseDraft::from_snapshot(snapshot)?;
    match draft
        .identities()
        .into_iter()
        .find(|id| history.holds_identity(*id))
    {
        Some(id) => Err(Error::new(ErrorCode::ReusedIdentity).with_id(id)),
        None => Ok(()),
    }
}
