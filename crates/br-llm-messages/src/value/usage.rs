use serde::{Deserialize, Serialize};

#[non_exhaustive]
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Usage {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    input_tokens: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    output_tokens: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    cache_read_input_tokens: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    cache_creation_input_tokens: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    thinking_tokens: Option<u64>,
}

impl Usage {
    pub fn new() -> Self {
        Usage::default()
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
