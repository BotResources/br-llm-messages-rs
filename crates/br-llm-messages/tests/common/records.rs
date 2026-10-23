use br_llm_messages::{Error, RecordSet, Uuid};
use serde_json::Value;

use super::conversations::rich;

pub fn rich_set() -> RecordSet {
    RecordSet::from_conversation(&rich()).unwrap()
}

pub fn row<'a>(document: &'a mut Value, collection: &str, id: Uuid) -> &'a mut Value {
    let wanted = id.hyphenated().to_string();
    document[collection]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|row| row["id"] == wanted)
        .expect("row present")
}

pub fn try_edit(set: &RecordSet, change: impl FnOnce(&mut Value)) -> Result<RecordSet, Error> {
    let mut document = set.to_value().unwrap();
    change(&mut document);
    RecordSet::from_value(document)
}

pub fn edited(set: &RecordSet, change: impl FnOnce(&mut Value)) -> RecordSet {
    try_edit(set, change).expect("the edited document stays locally valid")
}

pub fn shuffled(set: &RecordSet) -> RecordSet {
    let mut turns = set.turns().to_vec();
    let mut entries = set.entries().to_vec();
    let mut blocks = set.blocks().to_vec();
    turns.reverse();
    entries.reverse();
    blocks.reverse();
    let third = blocks.len() / 3;
    let half = entries.len() / 2;
    blocks.rotate_left(third);
    entries.rotate_left(half);
    RecordSet::new(turns, entries, blocks)
}
