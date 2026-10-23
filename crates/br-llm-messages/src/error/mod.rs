mod code;
mod json;

pub use code::ErrorCode;

use uuid::Uuid;

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    code: ErrorCode,
    id: Option<Uuid>,
    location: Option<String>,
}

impl Error {
    pub(crate) fn new(code: ErrorCode) -> Self {
        Error {
            code,
            id: None,
            location: None,
        }
    }

    pub(crate) fn with_id(mut self, id: Uuid) -> Self {
        self.id.get_or_insert(id);
        self
    }

    pub(crate) fn at(mut self, segment: impl AsRef<str>) -> Self {
        let segment = segment.as_ref();
        self.location = Some(match self.location.take() {
            None => segment.to_owned(),
            Some(rest) if rest.starts_with('[') => format!("{segment}{rest}"),
            Some(rest) => format!("{segment}.{rest}"),
        });
        self
    }

    pub fn code(&self) -> ErrorCode {
        self.code
    }

    pub fn object_id(&self) -> Option<Uuid> {
        self.id
    }

    pub fn location(&self) -> Option<&str> {
        self.location.as_deref()
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.code)?;
        if let Some(id) = &self.id {
            write!(f, " id={}", id.hyphenated())?;
        }
        if let Some(location) = &self.location {
            write!(f, " at {location}")?;
        }
        Ok(())
    }
}

impl std::error::Error for Error {}
