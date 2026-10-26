use serde::Deserialize;
use serde::Serialize;
use serde::de::IgnoredAny;
use serde_json::Value;

use super::check_content;
use crate::block::PlainBlock;
use crate::error::{Error, ErrorCode};
use crate::id::{AgentId, EntryId};
use crate::value::Author;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OutputWire {
    #[serde(rename = "type")]
    _kind: IgnoredAny,
    id: EntryId,
    agent: AgentId,
    author: Author,
    content: Vec<PlainBlock>,
}

fn checked_content(id: EntryId, content: &[PlainBlock]) -> Result<(), Error> {
    check_content(id, content)?;
    for (index, block) in content.iter().enumerate() {
        if *block != block.bare() {
            return Err(Error::new(ErrorCode::DisallowedKind)
                .with_id(block.id().as_uuid())
                .at(format!("content[{index}]")));
        }
    }
    Ok(())
}

macro_rules! projected_output {
    ($name:ident) => {
        #[non_exhaustive]
        #[derive(Debug, Clone, PartialEq, Serialize)]
        pub struct $name {
            id: EntryId,
            agent: AgentId,
            author: Author,
            content: Vec<PlainBlock>,
        }

        impl $name {
            pub fn id(&self) -> EntryId {
                self.id
            }

            pub fn agent(&self) -> AgentId {
                self.agent
            }

            pub fn author(&self) -> &Author {
                &self.author
            }

            pub fn content(&self) -> &[PlainBlock] {
                &self.content
            }

            pub(crate) fn from_content(
                id: EntryId,
                agent: AgentId,
                author: Author,
                content: Vec<PlainBlock>,
            ) -> Result<Self, Error> {
                checked_content(id, &content)?;
                Ok($name {
                    id,
                    agent,
                    author,
                    content,
                })
            }

            pub(crate) fn from_projected_value(value: Value) -> Result<Self, Error> {
                let wire: OutputWire = crate::json::decode_value(value)?;
                $name::from_content(wire.id, wire.agent, wire.author, wire.content)
            }
        }
    };
}

projected_output!(AgentOutput);
projected_output!(PartialAgentOutput);
