use super::ProjectedThread;
use crate::conversation::{BlockRef, EntryView, Lookup, PlacedEntry, placed_entries};
use crate::error::Error;
use crate::id::{BlockId, EntryId, TurnId};
use crate::turn::Turn;

impl ProjectedThread {
    fn lookup(&self) -> Lookup<'_> {
        Lookup::new(&self.turns, &self.index)
    }

    pub fn turn(&self, id: &TurnId) -> Result<&Turn, Error> {
        self.lookup().turn(id)
    }

    pub fn entry(&self, id: &EntryId) -> Result<EntryView<'_>, Error> {
        self.lookup().entry(id)
    }

    pub fn block(&self, id: &BlockId) -> Result<BlockRef<'_>, Error> {
        self.lookup().block(id)
    }

    pub fn entries(&self) -> impl Iterator<Item = PlacedEntry<'_>> {
        placed_entries(&self.turns)
    }
}
