use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::{Error, ErrorCode};
use crate::value::{ToolName, opaque};

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    try_from = "ToolDefinitionWire",
    tag = "type",
    rename_all = "snake_case"
)]
pub enum ToolDefinition {
    #[non_exhaustive]
    Inline {
        name: ToolName,
        description: String,
        input_schema: Value,
    },
    #[non_exhaustive]
    Versioned { name: ToolName, version: String },
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
enum ToolDefinitionWire {
    Inline {
        name: ToolName,
        description: String,
        #[serde(deserialize_with = "crate::json::unique_value")]
        input_schema: Value,
    },
    Versioned {
        name: ToolName,
        version: String,
    },
}

impl ToolDefinition {
    pub fn inline(
        name: ToolName,
        description: impl Into<String>,
        input_schema: Value,
    ) -> Result<Self, Error> {
        if !input_schema.is_object() {
            return Err(Error::new(ErrorCode::InvalidContent).at("input_schema"));
        }
        opaque::check(&input_schema).map_err(|error| error.at("input_schema"))?;
        Ok(ToolDefinition::Inline {
            name,
            description: description.into(),
            input_schema,
        })
    }

    pub fn versioned(name: ToolName, version: impl Into<String>) -> Result<Self, Error> {
        let version = version.into();
        if version.is_empty() {
            return Err(Error::new(ErrorCode::InvalidContent).at("version"));
        }
        Ok(ToolDefinition::Versioned { name, version })
    }

    pub fn name(&self) -> &ToolName {
        match self {
            ToolDefinition::Inline { name, .. } | ToolDefinition::Versioned { name, .. } => name,
        }
    }
}

impl TryFrom<ToolDefinitionWire> for ToolDefinition {
    type Error = Error;

    fn try_from(wire: ToolDefinitionWire) -> Result<Self, Error> {
        match wire {
            ToolDefinitionWire::Inline {
                name,
                description,
                input_schema,
            } => ToolDefinition::inline(name, description, input_schema),
            ToolDefinitionWire::Versioned { name, version } => {
                ToolDefinition::versioned(name, version)
            }
        }
    }
}
