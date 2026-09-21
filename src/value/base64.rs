use crate::error::MessageError;

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Base64Data(String);

impl Base64Data {
    pub fn new(value: impl Into<String>) -> Result<Self, MessageError> {
        let value = value.into();
        if value.is_empty() {
            return Err(MessageError::Blank {
                field: "base64_data",
            });
        }
        if !value.len().is_multiple_of(4) {
            return Err(MessageError::InvalidBase64);
        }
        let body_len = value.trim_end_matches('=').len();
        let padding = value.len() - body_len;
        if padding > 2 {
            return Err(MessageError::InvalidBase64);
        }
        let all_valid = value.as_bytes()[..body_len]
            .iter()
            .all(|b| b.is_ascii_alphanumeric() || *b == b'+' || *b == b'/');
        if !all_valid {
            return Err(MessageError::InvalidBase64);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for Base64Data {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl TryFrom<String> for Base64Data {
    type Error = MessageError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<Base64Data> for String {
    fn from(value: Base64Data) -> Self {
        value.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn given_empty_when_new_then_blank() {
        assert!(matches!(
            Base64Data::new(""),
            Err(MessageError::Blank {
                field: "base64_data"
            })
        ));
    }

    #[test]
    fn given_valid_padded_value_when_new_then_accepted() {
        assert!(Base64Data::new("aGVsbG8=").is_ok());
        assert!(Base64Data::new("aGVsbA==").is_ok());
        assert!(Base64Data::new("YWJjZA==").is_ok());
        assert!(Base64Data::new("QUJDRA==").is_ok());
    }

    #[test]
    fn given_length_not_multiple_of_four_when_new_then_refused() {
        assert!(matches!(
            Base64Data::new("aGVsbG8"),
            Err(MessageError::InvalidBase64)
        ));
    }

    #[test]
    fn given_char_outside_alphabet_when_new_then_refused() {
        assert!(matches!(
            Base64Data::new("aGV*bG8="),
            Err(MessageError::InvalidBase64)
        ));
    }

    #[test]
    fn given_padding_in_the_middle_when_new_then_refused() {
        assert!(matches!(
            Base64Data::new("aG=lbG8="),
            Err(MessageError::InvalidBase64)
        ));
    }

    #[test]
    fn given_more_than_two_padding_chars_when_new_then_refused() {
        assert!(matches!(
            Base64Data::new("aGVs===="),
            Err(MessageError::InvalidBase64)
        ));
    }

    #[test]
    fn given_illegal_json_when_deserialized_then_refused() {
        assert!(serde_json::from_str::<Base64Data>("\"aGVsbG8\"").is_err());
    }

    #[test]
    fn given_valid_value_when_round_tripped_then_identical() {
        let data = Base64Data::new("aGVsbG8=").unwrap();
        let json = serde_json::to_string(&data).unwrap();
        assert_eq!(serde_json::from_str::<Base64Data>(&json).unwrap(), data);
    }
}
