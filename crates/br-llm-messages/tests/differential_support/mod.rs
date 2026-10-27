#![allow(
    dead_code,
    reason = "each integration test crate uses a different subset of these helpers"
)]

mod drafts;
mod pending;
mod scenario;

pub use pending::{Pending, draft_document};
pub use scenario::{HOSTILE, Hostile, Scenario, scenario};

use std::collections::HashSet;

use br_llm_messages::{
    EntryId, Error, PendingResponse, Perspective, ProjectedThread, RecordSet, TurnEntryKind,
    TurnKind, TurnRecord,
};
use serde_json::json;

use crate::common::{aid, cvid, oj};

pub type Select = fn(EntryId) -> bool;

pub fn perspective(agent: u64, select: Option<Select>) -> Perspective {
    let perspective = Perspective::of(aid(agent));
    match select {
        Some(select) => perspective.keeping(move |entry| select(entry.id())),
        None => perspective,
    }
}

fn memory_pending(scenario: &Scenario) -> Vec<PendingResponse<'_>> {
    let interrupted = scenario.interrupted.iter().map(PendingResponse::from);
    interrupted
        .chain(scenario.pending.iter().map(Pending::as_pending))
        .collect()
}

pub fn memory_documents(scenario: &Scenario) -> Vec<String> {
    let interrupted = scenario.interrupted.iter().map(draft_document);
    interrupted
        .chain(scenario.pending.iter().map(Pending::document))
        .collect()
}

pub fn memory(
    scenario: &Scenario,
    agent: u64,
    select: Option<Select>,
) -> Result<ProjectedThread, Error> {
    let perspective = perspective(agent, select);
    let pending = memory_pending(scenario);
    let projected = perspective.project_pending(cvid(900), &scenario.turns, &pending);
    if let (Some(stored), true) = (&scenario.stored, pending.is_empty()) {
        assert_eq!(
            Some(perspective.project(stored)),
            projected.clone().ok(),
            "the stored conversation path and the turns path differ"
        );
    }
    projected
}

fn foreign(turn: &TurnRecord, agent: u64) -> bool {
    turn.kind() == TurnKind::AITurn && turn.agent() != Some(aid(agent))
}

pub fn reduced(scenario: &Scenario, agent: u64, select: Option<Select>) -> RecordSet {
    let (mut turns, mut entries, mut blocks) = (Vec::new(), Vec::new(), Vec::new());
    for turn in &scenario.turns {
        let rows = RecordSet::from_turn(turn).unwrap();
        turns.extend(rows.turns().iter().cloned());
        if !rows.turns().iter().any(|row| foreign(row, agent)) {
            entries.extend(rows.entries().iter().cloned());
            blocks.extend(rows.blocks().iter().cloned());
            continue;
        }
        let mut sent = HashSet::new();
        let mut responses = HashSet::new();
        for row in rows.entries() {
            let response = row.kind() == TurnEntryKind::ModelResponse;
            let kept = response
                || (row.kind().is_incoming() && select.is_none_or(|select| select(row.id())));
            if !kept {
                continue;
            }
            sent.insert(row.id());
            if response {
                responses.insert(row.id());
            }
            let blank = response && row.id().as_uuid().as_u128() % 2 == 0;
            entries.push(match blank {
                true => br_llm_messages::EntryRecord::new(
                    row.id(),
                    row.turn(),
                    row.kind(),
                    row.state(),
                    oj(json!({})),
                ),
                false => row.clone(),
            });
        }
        blocks.extend(
            rows.blocks()
                .iter()
                .filter(|row| sent.contains(&row.entry()))
                .filter(|row| {
                    !responses.contains(&row.entry())
                        || (row.kind().is_plain() && row.parent().is_none())
                })
                .cloned(),
        );
    }
    for draft in &scenario.interrupted {
        let snapshot = draft.to_snapshot().unwrap();
        entries.push(snapshot.entry().clone());
        blocks.extend(snapshot.blocks().iter().cloned());
    }
    RecordSet::new()
        .with_turns(turns)
        .with_entries(entries)
        .with_blocks(blocks)
}

pub fn records(scenario: &Scenario, rows: RecordSet, agent: u64) -> Result<ProjectedThread, Error> {
    let pending: Vec<PendingResponse<'_>> =
        scenario.pending.iter().map(Pending::as_pending).collect();
    rows.into_projected_thread_pending(cvid(900), aid(agent), &pending)
}

fn index_after<'a>(location: &'a str, head: &str) -> Option<(usize, &'a str)> {
    let rest = location.strip_prefix(head)?.strip_prefix('[')?;
    let (index, tail) = rest.split_once(']')?;
    Some((index.parse().ok()?, tail))
}

fn names(document: &str, error: &Error) -> bool {
    let id = error.object_id().expect("the error names an identity");
    document.contains(&id.hyphenated().to_string())
}

pub fn memory_object(scenario: &Scenario, location: &str) -> String {
    if let Some((index, _)) = index_after(location, "pending") {
        return memory_documents(scenario)[index].clone();
    }
    let (turn, tail) = index_after(location, "turns").expect("a turn location");
    let value: serde_json::Value =
        serde_json::from_str(&scenario.turns[turn].to_json().unwrap()).unwrap();
    match index_after(tail, ".entries") {
        Some((entry, _)) => value["entries"][entry].to_string(),
        None => value["id"].to_string(),
    }
}

pub fn record_object(scenario: &Scenario, rows: &RecordSet, location: &str) -> String {
    if let Some((index, _)) = index_after(location, "pending") {
        return scenario.pending[index].document();
    }
    if let Some((index, _)) = index_after(location, "entries") {
        return text(&rows.entries()[index]);
    }
    if let Some((index, _)) = index_after(location, "blocks") {
        return text(&rows.blocks()[index]);
    }
    let (turn, tail) = index_after(location, "turns").expect("a row location");
    let row = &rows.turns()[turn];
    match index_after(tail, ".entries") {
        None => text(row),
        Some((entry, _)) => {
            let entry = rows
                .entries()
                .iter()
                .filter(|candidate| candidate.turn() == row.id())
                .nth(entry)
                .expect("the entry row exists");
            let blocks: Vec<String> = rows
                .blocks()
                .iter()
                .filter(|block| block.entry() == entry.id())
                .map(text)
                .collect();
            format!("{}{}", text(entry), blocks.join(""))
        }
    }
}

fn text<T: serde::Serialize>(row: &T) -> String {
    serde_json::to_string(row).unwrap()
}

pub fn agree(scenario: &Scenario, agent: u64, select: Option<Select>, label: &str) -> bool {
    let rows = reduced(scenario, agent, select);
    let live = memory(scenario, agent, select);
    let stored = records(scenario, rows.clone(), agent);
    match (live, stored) {
        (Ok(live), Ok(stored)) => {
            let (a, b) = (live.to_json().unwrap(), stored.to_json().unwrap());
            if a != b {
                let at = a
                    .bytes()
                    .zip(b.bytes())
                    .position(|(x, y)| x != y)
                    .unwrap_or(a.len().min(b.len()));
                let from = at.saturating_sub(300);
                panic!(
                    "{label}\nlive:   {}\nstored: {}",
                    &a[from..(at + 300).min(a.len())],
                    &b[from..(at + 300).min(b.len())]
                );
            }
            let reloaded = ProjectedThread::from_json(&live.to_json().unwrap()).unwrap();
            assert_eq!(
                reloaded, live,
                "{label}: the projection must pass its own reload rules"
            );
            true
        }
        (Err(live), Err(stored)) => {
            assert_eq!(live.code(), stored.code(), "{label}: {live} / {stored}");
            assert_eq!(
                live.object_id(),
                stored.object_id(),
                "{label}: {live} / {stored}"
            );
            let at_live = live
                .location()
                .unwrap_or_else(|| panic!("{label}: {live} has no location"));
            let at_stored = stored
                .location()
                .unwrap_or_else(|| panic!("{label}: {stored} has no location"));
            assert!(
                names(&memory_object(scenario, at_live), &live),
                "{label}: {live}"
            );
            assert!(
                names(&record_object(scenario, &rows, at_stored), &stored),
                "{label}: {stored}"
            );
            false
        }
        (live, stored) => panic!("{label}: the two paths disagree: {live:?} / {stored:?}"),
    }
}
