use serde::{Deserialize, Serialize};

use crate::error::{Error, ErrorCode};

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "EndingWire", tag = "type", rename_all = "snake_case")]
pub enum ResponseEnding {
    #[non_exhaustive]
    Finished {},
    #[non_exhaustive]
    ToolUse {},
    #[non_exhaustive]
    ProviderContinuation {},
    #[non_exhaustive]
    OutputExhaustion {},
    #[non_exhaustive]
    StopSequence {
        #[serde(skip_serializing_if = "Option::is_none")]
        sequence: Option<String>,
    },
    #[non_exhaustive]
    Refusal {},
    #[non_exhaustive]
    Filtering {},
    #[non_exhaustive]
    Unclassified { reason: String },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EndingWire {
    #[serde(rename = "type", deserialize_with = "crate::tagged::tag")]
    kind: String,
    #[serde(default, deserialize_with = "crate::json::optional")]
    reason: Option<String>,
    #[serde(default, deserialize_with = "crate::json::optional")]
    sequence: Option<String>,
}

impl ResponseEnding {
    pub fn finished() -> Self {
        ResponseEnding::Finished {}
    }

    pub fn tool_use() -> Self {
        ResponseEnding::ToolUse {}
    }

    pub fn provider_continuation() -> Self {
        ResponseEnding::ProviderContinuation {}
    }

    pub fn output_exhaustion() -> Self {
        ResponseEnding::OutputExhaustion {}
    }

    pub fn stop_sequence() -> Self {
        ResponseEnding::StopSequence { sequence: None }
    }

    pub fn matched_stop_sequence(sequence: impl Into<String>) -> Result<Self, Error> {
        let sequence = sequence.into();
        if sequence.is_empty() {
            return Err(Error::new(ErrorCode::InvalidContent).at("sequence"));
        }
        Ok(ResponseEnding::StopSequence {
            sequence: Some(sequence),
        })
    }

    pub fn refusal() -> Self {
        ResponseEnding::Refusal {}
    }

    pub fn filtering() -> Self {
        ResponseEnding::Filtering {}
    }

    pub fn unclassified(reason: impl Into<String>) -> Result<Self, Error> {
        let reason = reason.into();
        if reason.is_empty() {
            return Err(Error::new(ErrorCode::InvalidContent).at("reason"));
        }
        Ok(ResponseEnding::Unclassified { reason })
    }

    pub fn matched_sequence(&self) -> Option<&str> {
        match self {
            ResponseEnding::StopSequence { sequence } => sequence.as_deref(),
            _ => None,
        }
    }
}

impl TryFrom<EndingWire> for ResponseEnding {
    type Error = Error;

    fn try_from(wire: EndingWire) -> Result<Self, Error> {
        let forbid = |present: bool, field: &str| {
            if present {
                Err(Error::new(ErrorCode::InvalidContent).at(field))
            } else {
                Ok(())
            }
        };
        match wire.kind.as_str() {
            "unclassified" => {
                forbid(wire.sequence.is_some(), "sequence")?;
                let reason = wire
                    .reason
                    .ok_or_else(|| Error::new(ErrorCode::InvalidContent).at("reason"))?;
                ResponseEnding::unclassified(reason)
            }
            "stop_sequence" => {
                forbid(wire.reason.is_some(), "reason")?;
                match wire.sequence {
                    Some(sequence) => ResponseEnding::matched_stop_sequence(sequence),
                    None => Ok(ResponseEnding::stop_sequence()),
                }
            }
            other => {
                let ending = match other {
                    "finished" => ResponseEnding::finished(),
                    "tool_use" => ResponseEnding::tool_use(),
                    "provider_continuation" => ResponseEnding::provider_continuation(),
                    "output_exhaustion" => ResponseEnding::output_exhaustion(),
                    "refusal" => ResponseEnding::refusal(),
                    "filtering" => ResponseEnding::filtering(),
                    _ => return Err(Error::new(ErrorCode::VariantMismatch)),
                };
                forbid(wire.reason.is_some(), "reason")?;
                forbid(wire.sequence.is_some(), "sequence")?;
                Ok(ending)
            }
        }
    }
}
