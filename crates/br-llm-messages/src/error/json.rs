use serde_json::error::Category;
use uuid::Uuid;

use super::{Error, ErrorCode};

impl Error {
    pub fn from_json_error(error: &serde_json::Error) -> Error {
        match error.classify() {
            Category::Syntax | Category::Eof | Category::Io => {
                let mut mapped = Error::new(ErrorCode::MalformedJson);
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
    if message.starts_with("unknown variant") {
        return Error::new(ErrorCode::UnsupportedVariant);
    }
    if message.starts_with("unknown field") {
        return Error::new(ErrorCode::UnsupportedField);
    }
    if let Some(field) = missing_field(message) {
        let mut missing = Error::new(ErrorCode::MissingContent);
        missing.location = Some(field.to_owned());
        return missing;
    }
    Error::new(ErrorCode::InvalidContent)
}

fn strip_position(message: &str) -> &str {
    const MARKER: &str = " at line ";
    let Some(start) = message.rfind(MARKER) else {
        return message;
    };
    let tail = &message[start + MARKER.len()..];
    let mut parts = tail.split(" column ");
    let line = parts.next().unwrap_or_default();
    let column = parts.next().unwrap_or_default();
    let numeric = |text: &str| !text.is_empty() && text.bytes().all(|b| b.is_ascii_digit());
    if numeric(line) && numeric(column) && parts.next().is_none() {
        &message[..start]
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
