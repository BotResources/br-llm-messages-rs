mod ai;
mod declared;
mod input;
mod json;
mod kind;
mod mounting;
mod results;
mod rules;
mod state;

pub use ai::AITurn;
pub use input::InputTurn;
pub use kind::TurnKind;
pub use state::TurnState;

pub(crate) use declared::{
    Body, Class, Declared, Place, body_of_entry, body_of_incoming, places_of_incoming, turn_place,
};

use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

use crate::error::Error;
use crate::id::TurnId;

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum Turn {
    InputTurn(InputTurn),
    AITurn(AITurn),
}

impl Turn {
    pub fn id(&self) -> TurnId {
        match self {
            Turn::InputTurn(turn) => turn.id(),
            Turn::AITurn(turn) => turn.id(),
        }
    }

    pub fn kind(&self) -> TurnKind {
        match self {
            Turn::InputTurn(_) => TurnKind::InputTurn,
            Turn::AITurn(_) => TurnKind::AITurn,
        }
    }

    pub fn as_input(&self) -> Option<&InputTurn> {
        match self {
            Turn::InputTurn(turn) => Some(turn),
            Turn::AITurn(_) => None,
        }
    }

    pub fn as_ai(&self) -> Option<&AITurn> {
        match self {
            Turn::AITurn(turn) => Some(turn),
            Turn::InputTurn(_) => None,
        }
    }

    pub fn to_json(&self) -> Result<String, Error> {
        crate::json::encode(self)
    }

    pub fn from_json(text: &str) -> Result<Self, Error> {
        json::turn_from_value(crate::json::decode(text)?)
    }

    pub fn to_value(&self) -> Result<Value, Error> {
        crate::json::encode_value(self)
    }

    pub fn from_value(value: Value) -> Result<Self, Error> {
        json::turn_from_value(value)
    }
}

impl From<InputTurn> for Turn {
    fn from(turn: InputTurn) -> Self {
        Turn::InputTurn(turn)
    }
}

impl From<AITurn> for Turn {
    fn from(turn: AITurn) -> Self {
        Turn::AITurn(turn)
    }
}

macro_rules! deserialize_via_value {
    ($($turn:ty => $loader:path),+ $(,)?) => {
        $(
            impl<'de> Deserialize<'de> for $turn {
                fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                    let value = Value::deserialize(deserializer)?;
                    $loader(value).map_err(serde::de::Error::custom)
                }
            }
        )+
    };
}

deserialize_via_value!(
    Turn => json::turn_from_value,
    InputTurn => json::input_from_value,
    AITurn => json::ai_from_value,
);
