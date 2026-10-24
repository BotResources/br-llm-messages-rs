use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use super::text::PlainText;
use crate::error::{Error, ErrorCode};
use crate::id::CitationId;

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "TextRangeWire")]
pub struct TextRange {
    start: u64,
    end: u64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TextRangeWire {
    start: u64,
    end: u64,
}

impl TextRange {
    pub fn new(start: u64, end: u64) -> Result<Self, Error> {
        if start >= end {
            return Err(Error::new(ErrorCode::InvalidContent).at("range"));
        }
        Ok(TextRange { start, end })
    }

    pub fn start(&self) -> u64 {
        self.start
    }

    pub fn end(&self) -> u64 {
        self.end
    }
}

impl TryFrom<TextRangeWire> for TextRange {
    type Error = Error;

    fn try_from(wire: TextRangeWire) -> Result<Self, Error> {
        TextRange::new(wire.start, wire.end)
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "CitationSourceWire")]
pub struct CitationSource {
    reference: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CitationSourceWire {
    reference: String,
    title: Option<String>,
}

impl CitationSource {
    pub fn new(reference: impl Into<String>) -> Result<Self, Error> {
        let reference = reference.into();
        if reference.is_empty() {
            return Err(Error::new(ErrorCode::InvalidContent).at("reference"));
        }
        Ok(CitationSource {
            reference,
            title: None,
        })
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn reference(&self) -> &str {
        &self.reference
    }

    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }
}

impl TryFrom<CitationSourceWire> for CitationSource {
    type Error = Error;

    fn try_from(wire: CitationSourceWire) -> Result<Self, Error> {
        let source = CitationSource::new(wire.reference)?;
        Ok(match wire.title {
            Some(title) => source.with_title(title),
            None => source,
        })
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "CitationWire")]
pub struct Citation {
    id: CitationId,
    source: CitationSource,
    #[serde(skip_serializing_if = "Option::is_none")]
    range: Option<TextRange>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CitationWire {
    id: CitationId,
    source: CitationSource,
    range: Option<TextRange>,
}

impl Citation {
    pub fn new(id: CitationId, source: CitationSource) -> Self {
        Citation {
            id,
            source,
            range: None,
        }
    }

    pub fn with_range(mut self, range: TextRange) -> Self {
        self.range = Some(range);
        self
    }

    pub fn id(&self) -> CitationId {
        self.id
    }

    pub fn source(&self) -> &CitationSource {
        &self.source
    }

    pub fn range(&self) -> Option<TextRange> {
        self.range.clone()
    }

    pub(crate) fn validate_against(&self, text: &PlainText) -> Result<(), Error> {
        match &self.range {
            Some(range) if range.end > text.scalar_count() => {
                Err(Error::new(ErrorCode::InvalidContent)
                    .with_id(self.id.as_uuid())
                    .at("range"))
            }
            _ => Ok(()),
        }
    }

    pub(crate) fn check_all(text: &PlainText, citations: &[Citation]) -> Result<(), Error> {
        let mut seen = HashSet::with_capacity(citations.len());
        for (index, citation) in citations.iter().enumerate() {
            let located = |error: Error| error.at(format!("citations[{index}]"));
            if !seen.insert(citation.id) {
                return Err(located(
                    Error::new(ErrorCode::ReusedIdentity).with_id(citation.id.as_uuid()),
                ));
            }
            citation.validate_against(text).map_err(located)?;
        }
        Ok(())
    }
}

impl TryFrom<CitationWire> for Citation {
    type Error = Error;

    fn try_from(wire: CitationWire) -> Result<Self, Error> {
        let citation = Citation::new(wire.id, wire.source);
        Ok(match wire.range {
            Some(range) => citation.with_range(range),
            None => citation,
        })
    }
}
