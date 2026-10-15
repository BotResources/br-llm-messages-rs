use crate::block::UserBlock;
use crate::error::MessageError;
use crate::value::{Author, InputId, RuntimeKind};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum UserSource {
    Human,
    Runtime { kind: RuntimeKind },
}

impl UserSource {
    pub fn runtime(kind: impl Into<String>) -> Result<Self, MessageError> {
        Ok(UserSource::Runtime {
            kind: RuntimeKind::new(kind)?,
        })
    }
}

/// What a human or the runtime says to the conversation. Its id is chosen
/// by whoever creates the input: the library never makes one up.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(try_from = "RawUserInput")]
pub struct UserInput {
    id: InputId,
    source: UserSource,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    author: Option<Author>,
    content: Vec<UserBlock>,
}

#[derive(serde::Deserialize)]
struct RawUserInput {
    id: InputId,
    source: UserSource,
    #[serde(default)]
    author: Option<Author>,
    content: Vec<UserBlock>,
}

impl TryFrom<RawUserInput> for UserInput {
    type Error = MessageError;

    fn try_from(raw: RawUserInput) -> Result<Self, Self::Error> {
        UserInput::new(raw.id, raw.source, raw.author, raw.content)
    }
}

impl UserInput {
    pub fn new(
        id: InputId,
        source: UserSource,
        author: Option<Author>,
        content: Vec<UserBlock>,
    ) -> Result<Self, MessageError> {
        if content.is_empty() {
            return Err(MessageError::EmptyUserContent);
        }
        Ok(Self {
            id,
            source,
            author,
            content,
        })
    }

    pub fn id(&self) -> &InputId {
        &self.id
    }

    pub fn source(&self) -> &UserSource {
        &self.source
    }

    pub fn author(&self) -> Option<&Author> {
        self.author.as_ref()
    }

    pub fn content(&self) -> &[UserBlock] {
        &self.content
    }
}

#[cfg(test)]
#[path = "user_input_tests.rs"]
mod tests;
