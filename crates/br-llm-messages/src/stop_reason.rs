#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StopReason {
    EndTurn,
    AwaitingToolResults,
    MaxTokens,
    StopSequence,
    Refusal,
    ContentFilter,
    Other { reason: String },
}

impl std::fmt::Display for StopReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StopReason::EndTurn => f.write_str("end_turn"),
            StopReason::AwaitingToolResults => f.write_str("awaiting_tool_results"),
            StopReason::MaxTokens => f.write_str("max_tokens"),
            StopReason::StopSequence => f.write_str("stop_sequence"),
            StopReason::Refusal => f.write_str("refusal"),
            StopReason::ContentFilter => f.write_str("content_filter"),
            StopReason::Other { reason } => write!(f, "other({reason})"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn given_known_reason_when_serialized_then_type_tagged() {
        let json = serde_json::to_value(&StopReason::EndTurn).unwrap();
        assert_eq!(json, serde_json::json!({ "type": "end_turn" }));
    }

    #[test]
    fn given_other_reason_when_serialized_then_carries_verbatim() {
        let reason = StopReason::Other {
            reason: "pause_turn".to_owned(),
        };
        let json = serde_json::to_value(&reason).unwrap();
        assert_eq!(
            json,
            serde_json::json!({ "type": "other", "reason": "pause_turn" })
        );
        assert_eq!(serde_json::from_value::<StopReason>(json).unwrap(), reason);
    }

    #[test]
    fn given_every_variant_when_round_tripped_then_identical() {
        for reason in [
            StopReason::EndTurn,
            StopReason::AwaitingToolResults,
            StopReason::MaxTokens,
            StopReason::StopSequence,
            StopReason::Refusal,
            StopReason::ContentFilter,
            StopReason::Other {
                reason: "model_length".to_owned(),
            },
        ] {
            let json = serde_json::to_value(&reason).unwrap();
            assert_eq!(serde_json::from_value::<StopReason>(json).unwrap(), reason);
        }
    }
}
