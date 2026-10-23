use serde::{Deserialize, Serialize};

use super::{check_content, collect_content, refuse_author};
use crate::block::{IdSink, PlainBlock};
use crate::error::Error;
use crate::id::{EntryId, HumanId};

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "HumanMessageWire")]
pub struct HumanMessage {
    id: EntryId,
    human: HumanId,
    content: Vec<PlainBlock>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct HumanMessageWire {
    id: EntryId,
    human: HumanId,
    content: Vec<PlainBlock>,
    #[serde(default, deserialize_with = "super::present")]
    author: bool,
}

impl HumanMessage {
    pub fn new(id: EntryId, human: HumanId, content: Vec<PlainBlock>) -> Result<Self, Error> {
        check_content(id, &content)?;
        Ok(HumanMessage { id, human, content })
    }

    pub fn id(&self) -> EntryId {
        self.id
    }

    pub fn human(&self) -> HumanId {
        self.human
    }

    pub fn content(&self) -> &[PlainBlock] {
        &self.content
    }

    pub(crate) fn collect_ids(&self, sink: &mut IdSink) -> Result<(), Error> {
        collect_content(self.id, &self.content, sink)
    }
}

impl TryFrom<HumanMessageWire> for HumanMessage {
    type Error = Error;

    fn try_from(wire: HumanMessageWire) -> Result<Self, Error> {
        refuse_author(wire.author, wire.id)?;
        HumanMessage::new(wire.id, wire.human, wire.content)
    }
}
