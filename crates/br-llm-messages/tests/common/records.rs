use super::Raw as _;
use std::collections::VecDeque;

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
    let mut document = set.to_value().unwrap().raw();
    change(&mut document);
    RecordSet::from_value(super::oj(document))
}

pub fn edited(set: &RecordSet, change: impl FnOnce(&mut Value)) -> RecordSet {
    try_edit(set, change).expect("the edited document stays locally valid")
}

pub fn interleaved(set: &RecordSet) -> RecordSet {
    RecordSet::new()
        .with_turns(set.turns().to_vec())
        .with_entries(round_robin(set.entries().to_vec(), |row| row.turn()))
        .with_blocks(round_robin(set.blocks().to_vec(), |row| row.entry()))
}

fn round_robin<T, K: PartialEq + Copy>(rows: Vec<T>, key: impl Fn(&T) -> K) -> Vec<T> {
    let mut groups: Vec<(K, VecDeque<T>)> = Vec::new();
    for row in rows {
        let group = key(&row);
        match groups.iter_mut().find(|(known, _)| *known == group) {
            Some((_, queue)) => queue.push_back(row),
            None => groups.push((group, VecDeque::from([row]))),
        }
    }
    let mut out = Vec::new();
    loop {
        let before = out.len();
        for (_, queue) in &mut groups {
            out.extend(queue.pop_front());
        }
        if out.len() == before {
            return out;
        }
    }
}
