use crate::error::MessageError;

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Text(String);

impl Text {
    pub fn new(value: impl Into<String>) -> Result<Self, MessageError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(MessageError::Blank { field: "text" });
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for Text {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl TryFrom<String> for Text {
    type Error = MessageError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<Text> for String {
    fn from(value: Text) -> Self {
        value.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn given_empty_when_new_then_blank() {
        assert!(matches!(
            Text::new(""),
            Err(MessageError::Blank { field: "text" })
        ));
    }

    #[test]
    fn given_whitespace_only_when_new_then_blank() {
        assert!(matches!(
            Text::new("  \n\t "),
            Err(MessageError::Blank { field: "text" })
        ));
    }

    #[test]
    fn given_text_with_surrounding_space_when_new_then_preserved() {
        let text = Text::new("  hello  ").unwrap();
        assert_eq!(text.as_str(), "  hello  ");
    }

    #[test]
    fn given_whitespace_json_when_deserialized_then_refused() {
        assert!(serde_json::from_str::<Text>("\"   \"").is_err());
    }

    #[test]
    fn given_text_when_round_tripped_then_identical() {
        let text = Text::new("bonjour").unwrap();
        let json = serde_json::to_string(&text).unwrap();
        assert_eq!(serde_json::from_str::<Text>(&json).unwrap(), text);
    }
}
