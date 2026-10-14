use std::hash::{Hash, Hasher};

use super::{AuthorId, AuthorName};
use crate::error::MessageError;

/// Who speaks: a stable id and a display name. Two authors are the same
/// author when their ids are equal, whatever their names, so equality and
/// hashing look at the id alone and a rename keeps the author.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Author {
    id: AuthorId,
    name: AuthorName,
}

impl Author {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Result<Self, MessageError> {
        Ok(Self::from_parts(AuthorId::new(id)?, AuthorName::new(name)?))
    }

    pub fn from_parts(id: AuthorId, name: AuthorName) -> Self {
        Self { id, name }
    }

    pub fn id(&self) -> &AuthorId {
        &self.id
    }

    pub fn name(&self) -> &AuthorName {
        &self.name
    }
}

impl PartialEq for Author {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Author {}

impl Hash for Author {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl std::fmt::Display for Author {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name.as_str())
    }
}

#[cfg(test)]
#[path = "author_tests.rs"]
mod tests;
