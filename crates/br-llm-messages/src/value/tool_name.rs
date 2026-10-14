use crate::error::MessageError;

/// The longest tool name every main provider accepts.
const MAX_LEN: usize = 64;

/// A tool name every main provider accepts: 1 to 64 ASCII characters, each a
/// letter, a digit, `_` or `-`, the first a letter or `_`
/// (`^[A-Za-z_][A-Za-z0-9_-]{0,63}$`). A name a provider would refuse cannot
/// be built, and a stored conversation that holds one does not load.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ToolName(String);

impl ToolName {
    pub fn new(value: impl Into<String>) -> Result<Self, MessageError> {
        let value = value.into();
        if !is_valid(&value) {
            return Err(MessageError::InvalidToolName);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

fn is_valid(name: &str) -> bool {
    let bytes = name.as_bytes();
    let Some((first, rest)) = bytes.split_first() else {
        return false;
    };
    bytes.len() <= MAX_LEN
        && (first.is_ascii_alphabetic() || *first == b'_')
        && rest
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
}

impl std::fmt::Display for ToolName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl TryFrom<String> for ToolName {
    type Error = MessageError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<ToolName> for String {
    fn from(value: ToolName) -> Self {
        value.0
    }
}

#[cfg(test)]
#[path = "tool_name_tests.rs"]
mod tests;
