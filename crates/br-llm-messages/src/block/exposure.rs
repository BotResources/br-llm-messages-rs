#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResultExposure {
    Readable,
    Opaque,
    Mixed,
    NotExposed,
}

impl ResultExposure {
    pub fn as_str(self) -> &'static str {
        match self {
            ResultExposure::Readable => "readable",
            ResultExposure::Opaque => "opaque",
            ResultExposure::Mixed => "mixed",
            ResultExposure::NotExposed => "not_exposed",
        }
    }

    pub(crate) fn of(has_content: bool, has_replay: bool) -> Self {
        match (has_content, has_replay) {
            (true, false) => ResultExposure::Readable,
            (false, true) => ResultExposure::Opaque,
            (true, true) => ResultExposure::Mixed,
            (false, false) => ResultExposure::NotExposed,
        }
    }
}

impl std::fmt::Display for ResultExposure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}
