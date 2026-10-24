use std::fmt;

#[derive(Clone, PartialEq, Default)]
pub(in crate::stream) struct RawJson(String);

impl RawJson {
    pub(in crate::stream) fn push_str(&mut self, fragment: &str) {
        self.0.push_str(fragment);
    }

    pub(in crate::stream) fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for RawJson {
    fn from(text: String) -> Self {
        RawJson(text)
    }
}

impl fmt::Debug for RawJson {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("<redacted>")
    }
}
