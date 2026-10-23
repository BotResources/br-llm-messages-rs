use serde::de::IgnoredAny;
use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;

use super::{DRAFT_SCHEMA, DraftSnapshot};
use crate::error::{Error, ErrorCode};
use crate::record::{BlockRecord, EntryRecord};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Wire {
    #[serde(rename = "schema")]
    _schema: IgnoredAny,
    entry: Value,
    blocks: Vec<Value>,
}

pub(super) fn from_value(value: Value) -> Result<DraftSnapshot, Error> {
    match value.get("schema") {
        Some(Value::String(schema)) if schema == DRAFT_SCHEMA => {}
        _ => return Err(Error::new(ErrorCode::SchemaMismatch).at("schema")),
    }
    let wire: Wire = crate::json::decode_value(value)?;
    let entry: EntryRecord =
        crate::json::decode_value(wire.entry).map_err(|error| error.at("entry"))?;
    let mut blocks = Vec::with_capacity(wire.blocks.len());
    for (index, raw) in wire.blocks.into_iter().enumerate() {
        let block: BlockRecord =
            crate::json::decode_value(raw).map_err(|error| error.at(format!("blocks[{index}]")))?;
        blocks.push(block);
    }
    DraftSnapshot::new(entry, blocks)
}

impl Serialize for DraftSnapshot {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut snapshot = serializer.serialize_struct("DraftSnapshot", 3)?;
        snapshot.serialize_field("schema", DRAFT_SCHEMA)?;
        snapshot.serialize_field("entry", &self.entry)?;
        snapshot.serialize_field("blocks", &self.blocks)?;
        snapshot.end()
    }
}

impl<'de> Deserialize<'de> for DraftSnapshot {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = crate::json::unique_value(deserializer)?;
        from_value(value).map_err(serde::de::Error::custom)
    }
}
