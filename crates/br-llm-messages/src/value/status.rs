use serde::{Deserialize, Serialize};

use crate::error::{Error, ErrorCode};

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Incomplete,
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResultStatus {
    Success,
    Error,
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "FailureDetailWire")]
pub struct FailureDetail {
    code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FailureDetailWire {
    code: String,
    message: Option<String>,
}

impl FailureDetail {
    pub fn new(code: impl Into<String>) -> Result<Self, Error> {
        let code = code.into();
        if code.is_empty() {
            return Err(Error::new(ErrorCode::InvalidContent).at("code"));
        }
        Ok(FailureDetail {
            code,
            message: None,
        })
    }

    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    pub fn code(&self) -> &str {
        &self.code
    }

    pub fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }
}

impl TryFrom<FailureDetailWire> for FailureDetail {
    type Error = Error;

    fn try_from(wire: FailureDetailWire) -> Result<Self, Error> {
        let detail = FailureDetail::new(wire.code)?;
        Ok(match wire.message {
            Some(message) => detail.with_message(message),
            None => detail,
        })
    }
}
