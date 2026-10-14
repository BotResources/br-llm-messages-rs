nonempty_string_newtype!(ToolCallId, "tool_call_id");
nonempty_string_newtype!(AuthorId, "author_id");
nonempty_string_newtype!(AuthorName, "author_name");
nonempty_string_newtype!(ModelId, "model_id");
nonempty_string_newtype!(TurnId, "turn_id");
nonempty_string_newtype!(RuntimeKind, "runtime_kind");

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::MessageError;

    #[test]
    fn given_empty_string_when_new_then_blank() {
        assert!(matches!(
            ToolCallId::new(""),
            Err(MessageError::Blank {
                field: "tool_call_id"
            })
        ));
        assert!(matches!(
            AuthorId::new(""),
            Err(MessageError::Blank { field: "author_id" })
        ));
        assert!(matches!(
            AuthorName::new(""),
            Err(MessageError::Blank {
                field: "author_name"
            })
        ));
        assert!(matches!(
            ModelId::new(""),
            Err(MessageError::Blank { field: "model_id" })
        ));
        assert!(matches!(
            TurnId::new(""),
            Err(MessageError::Blank { field: "turn_id" })
        ));
        assert!(matches!(
            RuntimeKind::new(""),
            Err(MessageError::Blank {
                field: "runtime_kind"
            })
        ));
    }

    #[test]
    fn given_nonempty_when_new_then_preserved_verbatim() {
        let id = ToolCallId::new("call_42").unwrap();
        assert_eq!(id.as_str(), "call_42");
        assert_eq!(id.to_string(), "call_42");
    }

    #[test]
    fn given_single_space_when_new_then_accepted() {
        assert!(ToolCallId::new(" ").is_ok());
        assert!(ModelId::new(" ").is_ok());
        assert!(TurnId::new(" ").is_ok());
        assert!(RuntimeKind::new(" ").is_ok());
    }

    #[test]
    fn given_valid_json_string_when_deserialized_then_round_trips() {
        let id = TurnId::new("0192-abc").unwrap();
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "\"0192-abc\"");
        assert_eq!(serde_json::from_str::<TurnId>(&json).unwrap(), id);
    }

    #[test]
    fn given_empty_json_string_when_deserialized_then_refused() {
        assert!(serde_json::from_str::<ToolCallId>("\"\"").is_err());
        assert!(serde_json::from_str::<RuntimeKind>("\"\"").is_err());
    }
}
