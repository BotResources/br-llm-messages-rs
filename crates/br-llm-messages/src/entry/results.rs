use std::collections::HashSet;

use serde::{Deserialize, Serialize, Serializer};

use super::{locate, refuse_author};
use crate::block::{ContentBlock, IdSink, RuntimeToolResult};
use crate::error::{Error, ErrorCode};
use crate::id::EntryId;
use crate::value::ObservedAt;

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "RuntimeToolResultsWire")]
pub struct RuntimeToolResults {
    id: EntryId,
    #[serde(serialize_with = "serialize_results")]
    results: Vec<RuntimeToolResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    observed_at: Option<ObservedAt>,
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ResultItem<'a> {
    RuntimeToolResult(&'a RuntimeToolResult),
}

fn serialize_results<S: Serializer>(
    results: &[RuntimeToolResult],
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.collect_seq(results.iter().map(ResultItem::RuntimeToolResult))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RuntimeToolResultsWire {
    id: EntryId,
    results: Vec<ContentBlock>,
    #[serde(default, deserialize_with = "super::present")]
    author: bool,
    #[serde(default, deserialize_with = "crate::json::optional")]
    observed_at: Option<ObservedAt>,
}

impl RuntimeToolResults {
    pub fn new(id: EntryId, results: Vec<RuntimeToolResult>) -> Result<Self, Error> {
        let entry = RuntimeToolResults {
            id,
            results,
            observed_at: None,
        };
        entry.validate().map_err(|error| locate(error, id))?;
        Ok(entry)
    }

    pub fn with_observed_at(mut self, observed_at: ObservedAt) -> Self {
        self.observed_at = Some(observed_at);
        self
    }

    pub fn id(&self) -> EntryId {
        self.id
    }

    pub fn results(&self) -> &[RuntimeToolResult] {
        &self.results
    }

    pub fn observed_at(&self) -> Option<&ObservedAt> {
        self.observed_at.as_ref()
    }

    pub(crate) fn collect_ids(&self, sink: &mut IdSink) -> Result<(), Error> {
        sink.insert(self.id.as_uuid())?;
        for (index, result) in self.results.iter().enumerate() {
            result
                .collect_ids(sink)
                .map_err(|error| error.at(format!("results[{index}]")))?;
        }
        Ok(())
    }

    fn validate(&self) -> Result<(), Error> {
        if self.results.is_empty() {
            return Err(Error::new(ErrorCode::InvalidContent).at("results"));
        }
        self.collect_ids(&mut IdSink::default())?;
        let mut answered = HashSet::new();
        for (index, result) in self.results.iter().enumerate() {
            if !answered.insert(result.call()) {
                return Err(Error::new(ErrorCode::DuplicateResult)
                    .with_id(result.call().as_uuid())
                    .at(format!("results[{index}]")));
            }
        }
        Ok(())
    }
}

impl TryFrom<RuntimeToolResultsWire> for RuntimeToolResults {
    type Error = Error;

    fn try_from(wire: RuntimeToolResultsWire) -> Result<Self, Error> {
        refuse_author(wire.author, wire.id)?;
        let mut results = Vec::with_capacity(wire.results.len());
        for (index, block) in wire.results.into_iter().enumerate() {
            match block {
                ContentBlock::RuntimeToolResult(result) => results.push(result),
                other => {
                    return Err(Error::new(ErrorCode::DisallowedKind)
                        .with_id(other.id().as_uuid())
                        .at(format!("results[{index}]")));
                }
            }
        }
        let entry = RuntimeToolResults::new(wire.id, results)?;
        Ok(match wire.observed_at {
            Some(observed_at) => entry.with_observed_at(observed_at),
            None => entry,
        })
    }
}
