use serde::{Deserialize, Serialize};

use super::{check_content, refuse_author};
use crate::block::PlainBlock;
use crate::error::Error;
use crate::id::{EntryId, HumanId};
use crate::value::ObservedAt;

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "HumanMessageWire")]
pub struct HumanMessage {
    id: EntryId,
    human: HumanId,
    content: Vec<PlainBlock>,
    #[serde(skip_serializing_if = "Option::is_none")]
    observed_at: Option<ObservedAt>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct HumanMessageWire {
    id: EntryId,
    human: HumanId,
    content: Vec<PlainBlock>,
    #[serde(default, deserialize_with = "super::present")]
    author: bool,
    observed_at: Option<ObservedAt>,
}

impl HumanMessage {
    pub fn new(id: EntryId, human: HumanId, content: Vec<PlainBlock>) -> Result<Self, Error> {
        check_content(id, &content)?;
        Ok(HumanMessage {
            id,
            human,
            content,
            observed_at: None,
        })
    }

    pub fn with_observed_at(mut self, observed_at: ObservedAt) -> Self {
        self.observed_at = Some(observed_at);
        self
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

    pub fn observed_at(&self) -> Option<&ObservedAt> {
        self.observed_at.as_ref()
    }
}

impl TryFrom<HumanMessageWire> for HumanMessage {
    type Error = Error;

    fn try_from(wire: HumanMessageWire) -> Result<Self, Error> {
        refuse_author(wire.author, wire.id)?;
        let mut message = HumanMessage::new(wire.id, wire.human, wire.content)?;
        message.observed_at = wire.observed_at;
        Ok(message)
    }
}
