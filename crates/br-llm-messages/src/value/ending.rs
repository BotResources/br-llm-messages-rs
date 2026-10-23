use serde::{Deserialize, Serialize};

use crate::error::{Error, ErrorCode};

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "EndingWire", tag = "type", rename_all = "snake_case")]
pub enum ResponseEnding {
    Finished,
    ToolResultsRequested,
    ProviderContinuation,
    OutputExhausted,
    StopSequence,
    Refusal,
    Filtered,
    #[non_exhaustive]
    Unclassified {
        reason: String,
    },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EndingWire {
    #[serde(rename = "type")]
    kind: String,
    reason: Option<String>,
}

impl ResponseEnding {
    pub fn unclassified(reason: impl Into<String>) -> Result<Self, Error> {
        let reason = reason.into();
        if reason.is_empty() {
            return Err(Error::new(ErrorCode::InvalidContent).at("reason"));
        }
        Ok(ResponseEnding::Unclassified { reason })
    }
}

impl TryFrom<EndingWire> for ResponseEnding {
    type Error = Error;

    fn try_from(wire: EndingWire) -> Result<Self, Error> {
        if wire.kind == "unclassified" {
            let reason = wire
                .reason
                .ok_or_else(|| Error::new(ErrorCode::MissingContent).at("reason"))?;
            return ResponseEnding::unclassified(reason);
        }
        let ending = match wire.kind.as_str() {
            "finished" => ResponseEnding::Finished,
            "tool_results_requested" => ResponseEnding::ToolResultsRequested,
            "provider_continuation" => ResponseEnding::ProviderContinuation,
            "output_exhausted" => ResponseEnding::OutputExhausted,
            "stop_sequence" => ResponseEnding::StopSequence,
            "refusal" => ResponseEnding::Refusal,
            "filtered" => ResponseEnding::Filtered,
            _ => return Err(Error::new(ErrorCode::UnsupportedVariant)),
        };
        if wire.reason.is_some() {
            return Err(Error::new(ErrorCode::UnsupportedField).at("reason"));
        }
        Ok(ending)
    }
}
