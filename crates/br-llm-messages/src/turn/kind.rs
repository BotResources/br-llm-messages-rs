use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::error::{Error, ErrorCode};

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TurnKind {
    #[serde(rename = "input_turn")]
    InputTurn,
    #[serde(rename = "ai_turn")]
    AITurn,
}

impl TurnKind {
    pub const ALL: &'static [TurnKind] = &[TurnKind::InputTurn, TurnKind::AITurn];

    pub fn as_str(self) -> &'static str {
        match self {
            TurnKind::InputTurn => "input_turn",
            TurnKind::AITurn => "ai_turn",
        }
    }
}

impl FromStr for TurnKind {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self, Error> {
        TurnKind::ALL
            .iter()
            .copied()
            .find(|kind| kind.as_str() == value)
            .ok_or_else(|| Error::new(ErrorCode::VariantMismatch))
    }
}

impl fmt::Display for TurnKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
