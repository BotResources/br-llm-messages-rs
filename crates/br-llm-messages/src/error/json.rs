use serde_json::error::Category;
use uuid::Uuid;

use super::{Error, ErrorCode};

const RECURSION_LIMIT: &str = "recursion limit exceeded";

impl Error {
    pub(crate) fn from_json_error(error: &(dyn std::error::Error + 'static)) -> Error {
        match error.downcast_ref::<serde_json::Error>() {
            Some(error) => Error::from_serde_json(error),
            None => from_data_message(&error.to_string()),
        }
    }

    pub(crate) fn from_data_error(error: &serde_json::Error) -> Error {
        from_data_message(&error.to_string())
    }

    fn from_serde_json(error: &serde_json::Error) -> Error {
        match error.classify() {
            Category::Syntax | Category::Eof | Category::Io => {
                let code = if error.to_string().starts_with(RECURSION_LIMIT) {
                    ErrorCode::InvalidContent
                } else {
                    ErrorCode::MalformedJson
                };
                let mut mapped = Error::new(code);
                if error.line() > 0 {
                    mapped.location =
                        Some(format!("line {} column {}", error.line(), error.column()));
                }
                mapped
            }
            Category::Data => from_data_message(&error.to_string()),
        }
    }
}

fn from_data_message(message: &str) -> Error {
    let message = strip_position(message);
    if let Some(embedded) = embedded(message) {
        return embedded;
    }
    if message.starts_with("duplicate field") {
        return Error::new(ErrorCode::MalformedJson);
    }
    if message.starts_with("unknown variant") {
        return Error::new(ErrorCode::VariantMismatch);
    }
    if let Some(field) = missing_field(message) {
        let code = if field == "type" {
            ErrorCode::VariantMismatch
        } else {
            ErrorCode::InvalidContent
        };
        let mut missing = Error::new(code);
        missing.location = Some(field.to_owned());
        return missing;
    }
    Error::new(ErrorCode::InvalidContent)
}

fn strip_position(message: &str) -> &str {
    const MARKER: &str = " at line ";
    let Some((head, tail)) = message.rsplit_once(MARKER) else {
        return message;
    };
    let mut parts = tail.split(" column ");
    let line = parts.next().unwrap_or_default();
    let column = parts.next().unwrap_or_default();
    let numeric = |text: &str| !text.is_empty() && text.bytes().all(|b| b.is_ascii_digit());
    if numeric(line) && numeric(column) && parts.next().is_none() {
        head
    } else {
        message
    }
}

fn embedded(message: &str) -> Option<Error> {
    let (token, mut rest) = match message.split_once(' ') {
        Some((token, rest)) => (token, Some(rest)),
        None => (message, None),
    };
    let mut error = Error::new(ErrorCode::parse(token)?);
    if let Some(after) = rest.and_then(|text| text.strip_prefix("id=")) {
        let (raw, remainder) = match after.split_once(' ') {
            Some((raw, remainder)) => (raw, Some(remainder)),
            None => (after, None),
        };
        error.id = Some(Uuid::try_parse(raw).ok()?);
        rest = remainder;
    }
    if let Some(location) = rest.and_then(|text| text.strip_prefix("at ")) {
        error.location = Some(location.to_owned());
        rest = None;
    }
    if rest.is_some() {
        return None;
    }
    (error.to_string() == message).then_some(error)
}

fn missing_field(message: &str) -> Option<&str> {
    message
        .strip_prefix("missing field `")
        .and_then(|rest| rest.strip_suffix('`'))
}

#[cfg(test)]
#[path = "json_tests.rs"]
mod tests;
