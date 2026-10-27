use serde::{Deserialize, Serialize};

use super::{locate, refuse_author};
use crate::error::Error;
use crate::id::EntryId;
use crate::value::{ObservedAt, PlainText};

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "RuntimeNoticeWire")]
pub struct RuntimeNotice {
    id: EntryId,
    text: PlainText,
    #[serde(skip_serializing_if = "Option::is_none")]
    observed_at: Option<ObservedAt>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RuntimeNoticeWire {
    id: EntryId,
    text: String,
    #[serde(default, deserialize_with = "super::present")]
    author: bool,
    #[serde(default, deserialize_with = "crate::json::optional")]
    observed_at: Option<ObservedAt>,
}

impl RuntimeNotice {
    pub fn new(id: EntryId, text: impl Into<String>) -> Result<Self, Error> {
        let text = PlainText::new(text).map_err(|error| locate(error.at("text"), id))?;
        Ok(RuntimeNotice {
            id,
            text,
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

    pub fn text(&self) -> &str {
        self.text.as_str()
    }

    pub fn observed_at(&self) -> Option<&ObservedAt> {
        self.observed_at.as_ref()
    }
}

impl TryFrom<RuntimeNoticeWire> for RuntimeNotice {
    type Error = Error;

    fn try_from(wire: RuntimeNoticeWire) -> Result<Self, Error> {
        refuse_author(wire.author, wire.id)?;
        let notice = RuntimeNotice::new(wire.id, wire.text)?;
        Ok(match wire.observed_at {
            Some(observed_at) => notice.with_observed_at(observed_at),
            None => notice,
        })
    }
}
