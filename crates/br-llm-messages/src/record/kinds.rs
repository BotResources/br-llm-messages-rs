use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::error::{Error, ErrorCode};
use crate::id::{AgentId, HumanId};

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RepresentationState {
    Producing,
    Complete,
    Interrupted,
}

impl RepresentationState {
    pub const ALL: &'static [RepresentationState] = &[
        RepresentationState::Producing,
        RepresentationState::Complete,
        RepresentationState::Interrupted,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            RepresentationState::Producing => "producing",
            RepresentationState::Complete => "complete",
            RepresentationState::Interrupted => "interrupted",
        }
    }
}

impl FromStr for RepresentationState {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self, Error> {
        RepresentationState::ALL
            .iter()
            .copied()
            .find(|state| state.as_str() == value)
            .ok_or_else(|| Error::new(ErrorCode::VariantMismatch))
    }
}

impl fmt::Display for RepresentationState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

tagged_enum! {
    #[non_exhaustive]
    #[derive(Debug, Clone, PartialEq)]
    pub enum ActorRef, ActorRefWire, remote = "ActorRef", [tag = "type", rename_all = "snake_case", deny_unknown_fields] {
        #[non_exhaustive]
        Human { id: HumanId },
        #[non_exhaustive]
        Agent { id: AgentId },
    }
}

impl ActorRef {
    pub fn human(id: HumanId) -> Self {
        ActorRef::Human { id }
    }

    pub fn agent(id: AgentId) -> Self {
        ActorRef::Agent { id }
    }
}
