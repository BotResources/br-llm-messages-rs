use std::collections::HashSet;

use super::block::BlockRecord;
use super::entry::EntryRecord;
use super::set::RecordSet;
use super::turn::TurnRecord;
use crate::id::EntryId;

pub(super) struct Kept<'a> {
    turns: &'a [TurnRecord],
    entries: Vec<(usize, &'a EntryRecord)>,
    blocks: Vec<(usize, &'a BlockRecord)>,
}

impl<'a> Kept<'a> {
    pub(super) fn without(set: &'a RecordSet, dropped: &HashSet<EntryId>) -> Self {
        Kept {
            turns: set.turns(),
            entries: set
                .entries()
                .iter()
                .enumerate()
                .filter(|(_, row)| !dropped.contains(&row.id()))
                .collect(),
            blocks: set
                .blocks()
                .iter()
                .enumerate()
                .filter(|(_, row)| !dropped.contains(&row.entry()))
                .collect(),
        }
    }

    pub(super) fn all(set: &'a RecordSet) -> Self {
        Kept::without(set, &HashSet::new())
    }

    pub(super) fn turns(&self) -> &'a [TurnRecord] {
        self.turns
    }

    pub(super) fn entries(&self) -> impl Iterator<Item = (usize, &'a EntryRecord)> + '_ {
        self.entries.iter().copied()
    }

    pub(super) fn blocks(&self) -> impl Iterator<Item = (usize, &'a BlockRecord)> + '_ {
        self.blocks.iter().copied()
    }
}
