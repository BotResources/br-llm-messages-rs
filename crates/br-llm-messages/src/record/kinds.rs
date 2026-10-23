use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::error::{Error, ErrorCode};
use crate::id::{AgentId, HumanId};

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RepresentationState {
    Producing,
    Complete,
    Interrupted,
    Discarded,
}

impl RepresentationState {
    pub const ALL: [RepresentationState; 4] = [
        RepresentationState::Producing,
        RepresentationState::Complete,
        RepresentationState::Interrupted,
        RepresentationState::Discarded,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            RepresentationState::Producing => "producing",
            RepresentationState::Complete => "complete",
            RepresentationState::Interrupted => "interrupted",
            RepresentationState::Discarded => "discarded",
        }
    }
}

impl FromStr for RepresentationState {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self, Error> {
        RepresentationState::ALL
            .into_iter()
            .find(|state| state.as_str() == value)
            .ok_or_else(|| Error::new(ErrorCode::UnsupportedVariant))
    }
}

impl fmt::Display for RepresentationState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum ActorRef {
    #[non_exhaustive]
    Human { id: HumanId },
    #[non_exhaustive]
    Agent { id: AgentId },
}

impl ActorRef {
    pub fn human(id: HumanId) -> Self {
        ActorRef::Human { id }
    }

    pub fn agent(id: AgentId) -> Self {
        ActorRef::Agent { id }
    }
}
