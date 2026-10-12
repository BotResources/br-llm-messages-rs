#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub struct Usage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cache_read_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cache_write_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub thinking_tokens: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn given_minimal_usage_when_serialized_then_options_omitted() {
        let usage = Usage {
            input_tokens: 10,
            output_tokens: 20,
            ..Usage::default()
        };
        let json = serde_json::to_value(&usage).unwrap();
        assert_eq!(
            json,
            serde_json::json!({ "input_tokens": 10, "output_tokens": 20 })
        );
        assert_eq!(serde_json::from_value::<Usage>(json).unwrap(), usage);
    }

    #[test]
    fn given_full_usage_when_round_tripped_then_identical() {
        let usage = Usage {
            input_tokens: 1,
            output_tokens: 2,
            cache_read_tokens: Some(3),
            cache_write_tokens: Some(4),
            thinking_tokens: Some(5),
        };
        let json = serde_json::to_value(&usage).unwrap();
        assert_eq!(serde_json::from_value::<Usage>(json).unwrap(), usage);
    }
}
