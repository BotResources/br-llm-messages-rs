use serde::{Deserialize, Serialize};

use crate::error::{Error, ErrorCode};

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "EndingWire", tag = "type", rename_all = "snake_case")]
pub enum ResponseEnding {
    #[non_exhaustive]
    Finished {},
    #[non_exhaustive]
    ToolResultsRequested {},
    #[non_exhaustive]
    ProviderContinuation {},
    #[non_exhaustive]
    OutputExhausted {},
    #[non_exhaustive]
    StopSequence {},
    #[non_exhaustive]
    Refusal {},
    #[non_exhaustive]
    Filtered {},
    #[non_exhaustive]
    Unclassified { reason: String },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EndingWire {
    #[serde(rename = "type")]
    kind: String,
    reason: Option<String>,
}

impl ResponseEnding {
    pub fn finished() -> Self {
        ResponseEnding::Finished {}
    }

    pub fn tool_results_requested() -> Self {
        ResponseEnding::ToolResultsRequested {}
    }

    pub fn provider_continuation() -> Self {
        ResponseEnding::ProviderContinuation {}
    }

    pub fn output_exhausted() -> Self {
        ResponseEnding::OutputExhausted {}
    }

    pub fn stop_sequence() -> Self {
        ResponseEnding::StopSequence {}
    }

    pub fn refusal() -> Self {
        ResponseEnding::Refusal {}
    }

    pub fn filtered() -> Self {
        ResponseEnding::Filtered {}
    }

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
            "finished" => ResponseEnding::finished(),
            "tool_results_requested" => ResponseEnding::tool_results_requested(),
            "provider_continuation" => ResponseEnding::provider_continuation(),
            "output_exhausted" => ResponseEnding::output_exhausted(),
            "stop_sequence" => ResponseEnding::stop_sequence(),
            "refusal" => ResponseEnding::refusal(),
            "filtered" => ResponseEnding::filtered(),
            _ => return Err(Error::new(ErrorCode::UnsupportedVariant)),
        };
        if wire.reason.is_some() {
            return Err(Error::new(ErrorCode::UnsupportedField).at("reason"));
        }
        Ok(ending)
    }
}
