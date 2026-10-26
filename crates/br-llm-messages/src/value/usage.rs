use serde::{Deserialize, Serialize};

use crate::error::{Error, ErrorCode};

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "UsageWire")]
pub struct Usage {
    #[serde(skip_serializing_if = "Option::is_none")]
    input_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    output_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cache_read_input_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cache_creation_input_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    thinking_tokens: Option<u64>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct UsageWire {
    #[serde(default, deserialize_with = "crate::json::optional")]
    input_tokens: Option<u64>,
    #[serde(default, deserialize_with = "crate::json::optional")]
    output_tokens: Option<u64>,
    #[serde(default, deserialize_with = "crate::json::optional")]
    cache_read_input_tokens: Option<u64>,
    #[serde(default, deserialize_with = "crate::json::optional")]
    cache_creation_input_tokens: Option<u64>,
    #[serde(default, deserialize_with = "crate::json::optional")]
    thinking_tokens: Option<u64>,
}

impl TryFrom<UsageWire> for Usage {
    type Error = Error;

    fn try_from(wire: UsageWire) -> Result<Self, Error> {
        let observed = [
            wire.input_tokens,
            wire.output_tokens,
            wire.cache_read_input_tokens,
            wire.cache_creation_input_tokens,
            wire.thinking_tokens,
        ];
        if observed.iter().all(Option::is_none) {
            return Err(Error::new(ErrorCode::InvalidContent).at("usage"));
        }
        Ok(Usage {
            input_tokens: wire.input_tokens,
            output_tokens: wire.output_tokens,
            cache_read_input_tokens: wire.cache_read_input_tokens,
            cache_creation_input_tokens: wire.cache_creation_input_tokens,
            thinking_tokens: wire.thinking_tokens,
        })
    }
}

impl Usage {
    fn unobserved() -> Self {
        Usage {
            input_tokens: None,
            output_tokens: None,
            cache_read_input_tokens: None,
            cache_creation_input_tokens: None,
            thinking_tokens: None,
        }
    }

    pub fn of_input_tokens(count: u64) -> Self {
        Usage::unobserved().with_input_tokens(count)
    }

    pub fn of_output_tokens(count: u64) -> Self {
        Usage::unobserved().with_output_tokens(count)
    }

    pub fn of_cache_read_input_tokens(count: u64) -> Self {
        Usage::unobserved().with_cache_read_input_tokens(count)
    }

    pub fn of_cache_creation_input_tokens(count: u64) -> Self {
        Usage::unobserved().with_cache_creation_input_tokens(count)
    }

    pub fn of_thinking_tokens(count: u64) -> Self {
        Usage::unobserved().with_thinking_tokens(count)
    }

    pub fn with_input_tokens(mut self, count: u64) -> Self {
        self.input_tokens = Some(count);
        self
    }

    pub fn with_output_tokens(mut self, count: u64) -> Self {
        self.output_tokens = Some(count);
        self
    }

    pub fn with_cache_read_input_tokens(mut self, count: u64) -> Self {
        self.cache_read_input_tokens = Some(count);
        self
    }

    pub fn with_cache_creation_input_tokens(mut self, count: u64) -> Self {
        self.cache_creation_input_tokens = Some(count);
        self
    }

    pub fn with_thinking_tokens(mut self, count: u64) -> Self {
        self.thinking_tokens = Some(count);
        self
    }

    pub fn input_tokens(&self) -> Option<u64> {
        self.input_tokens
    }

    pub fn output_tokens(&self) -> Option<u64> {
        self.output_tokens
    }

    pub fn cache_read_input_tokens(&self) -> Option<u64> {
        self.cache_read_input_tokens
    }

    pub fn cache_creation_input_tokens(&self) -> Option<u64> {
        self.cache_creation_input_tokens
    }

    pub fn thinking_tokens(&self) -> Option<u64> {
        self.thinking_tokens
    }
}
