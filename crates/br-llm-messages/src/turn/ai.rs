use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};

use crate::opaque_json::OpaqueJson;

use super::declared::Declared;
use super::kind::TurnKind;
use super::rules;
use super::state::{Tracker, TurnState};
use crate::entry::TurnEntry;
use crate::error::Error;
use crate::id::{AgentId, TurnId};
use crate::value::Author;

#[derive(Debug, Clone, PartialEq)]
pub struct AITurn {
    id: TurnId,
    agent: AgentId,
    author: Author,
    entries: Vec<TurnEntry>,
    tracker: Box<Tracker>,
}

impl Serialize for AITurn {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut turn = serializer.serialize_struct("AITurn", 5)?;
        turn.serialize_field("type", TurnKind::AITurn.as_str())?;
        turn.serialize_field("id", &self.id)?;
        turn.serialize_field("agent", &self.agent)?;
        turn.serialize_field("author", &self.author)?;
        turn.serialize_field("entries", &self.entries)?;
        turn.end()
    }
}

impl AITurn {
    pub fn open(id: TurnId, agent: AgentId, author: Author) -> Self {
        AITurn {
            id,
            agent,
            author,
            entries: Vec::new(),
            tracker: Box::new(Tracker::new(id)),
        }
    }

    pub fn from_parts(
        id: TurnId,
        agent: AgentId,
        author: Author,
        entries: Vec<TurnEntry>,
    ) -> Result<Self, Error> {
        let mut turn = AITurn::open(id, agent, author);
        for (index, entry) in entries.into_iter().enumerate() {
            turn.push_entry(entry)
                .map_err(|error| error.at(format!("entries[{index}]")))?;
        }
        Ok(turn)
    }

    pub fn id(&self) -> TurnId {
        self.id
    }

    pub fn agent(&self) -> AgentId {
        self.agent
    }

    pub fn author(&self) -> &Author {
        &self.author
    }

    pub fn entries(&self) -> &[TurnEntry] {
        &self.entries
    }

    pub fn state(&self) -> TurnState {
        self.tracker.state()
    }

    pub fn is_closed(&self) -> bool {
        self.tracker.closure.is_some()
    }

    pub fn push_entry(&mut self, entry: TurnEntry) -> Result<(), Error> {
        let declared = self.check_push(&entry)?;
        self.commit_push(entry, declared);
        Ok(())
    }

    pub(crate) fn check_push(&self, entry: &TurnEntry) -> Result<Declared, Error> {
        rules::check(&self.tracker, entry, self.entries.len())
    }

    pub(crate) fn commit_push(&mut self, entry: TurnEntry, declared: Declared) {
        self.tracker.apply(&entry, declared);
        self.entries.push(entry);
    }

    pub(crate) fn declared(&self) -> Declared {
        self.tracker.declared()
    }

    pub fn to_json(&self) -> Result<String, Error> {
        crate::json::encode(self)
    }

    pub fn from_json(text: &str) -> Result<Self, Error> {
        super::json::ai_from_value(crate::json::parse(text)?)
    }

    pub fn to_value(&self) -> Result<OpaqueJson, Error> {
        crate::json::encode_value(self).map(OpaqueJson::from_inner)
    }

    pub fn from_value(value: OpaqueJson) -> Result<Self, Error> {
        let value = value.into_inner();
        super::json::ai_from_value(value)
    }
}
