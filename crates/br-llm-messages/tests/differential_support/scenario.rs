use br_llm_messages::{
    AITurn, BlockId, Conversation, EntryId, HumanMessage, InputTurn, ModelResponse, ResponseDraft,
    ResponseEnding, ResponseMetadata, TextBlock, ThinkingBlock, Turn, TurnId, Uuid,
};
use serde_json::Value;

use super::drafts::{Extras, draft, enriched, finished, stopped};
use super::pending::Pending;

use crate::common::histories::{Lcg, history};
use crate::common::turns::tid;
use crate::common::{aid, author, bid, cid, eid, hid};

pub struct Scenario {
    pub turns: Vec<Turn>,
    pub stored: Option<Conversation>,
    pub interrupted: Vec<ResponseDraft>,
    pub interrupted_owner: Option<u64>,
    pub pending: Vec<Pending>,
}

#[derive(Clone, Copy, Debug)]
pub enum Hostile {
    Clean,
    InputEntry,
    InputBlock,
    ForeignEntry,
    ForeignBlock,
    PendingEntry,
    PendingBlock,
    InterruptedEntry,
    InterruptedBlock,
    InterruptedChild,
}

pub const HOSTILE: [Hostile; 10] = [
    Hostile::Clean,
    Hostile::InputEntry,
    Hostile::InputBlock,
    Hostile::ForeignEntry,
    Hostile::ForeignBlock,
    Hostile::PendingEntry,
    Hostile::PendingBlock,
    Hostile::InterruptedEntry,
    Hostile::InterruptedBlock,
    Hostile::InterruptedChild,
];

fn identities(value: &Value, out: &mut Vec<Uuid>) {
    match value {
        Value::Object(object) => {
            if let Some(Value::String(id)) = object.get("id")
                && let Ok(id) = Uuid::try_parse(id)
            {
                out.push(id);
            }
            object.values().for_each(|nested| identities(nested, out));
        }
        Value::Array(items) => items.iter().for_each(|nested| identities(nested, out)),
        _ => {}
    }
}

fn all_ids(conversation: &Conversation) -> Vec<Uuid> {
    let mut out = Vec::new();
    identities(
        &serde_json::from_str(&conversation.to_json().unwrap()).unwrap(),
        &mut out,
    );
    out
}

struct Fresh(u64);

impl Fresh {
    fn next(&mut self) -> u64 {
        self.0 += 1;
        self.0
    }
}

fn spoken(entry: EntryId, thinking: BlockId, block: BlockId) -> ModelResponse {
    ModelResponse::new(
        entry,
        vec![
            ThinkingBlock::new(thinking, "hidden").unwrap().into(),
            TextBlock::new(block, "late").unwrap().into(),
        ],
        ResponseEnding::finished(),
        ResponseMetadata::new(),
    )
    .unwrap()
}

fn ai_turns(turns: &[Turn]) -> Vec<(TurnId, u64)> {
    turns
        .iter()
        .filter_map(|turn| turn.as_ai())
        .map(|ai| (ai.id(), (1..=4).find(|n| aid(*n) == ai.agent()).unwrap()))
        .collect()
}

fn extras(fresh: &mut Fresh) -> Extras {
    Extras {
        call: bid(fresh.next()),
        result: bid(fresh.next()),
        child: bid(fresh.next()),
        cited: bid(fresh.next()),
        citation: cid(fresh.next()),
    }
}

fn pending_of(rng: &mut Lcg, fresh: &mut Fresh, turns: &[Turn]) -> Vec<Pending> {
    let mut targets: Vec<TurnId> = ai_turns(turns).into_iter().map(|(id, _)| id).collect();
    targets.extend(
        turns
            .iter()
            .filter(|t| t.as_input().is_some())
            .map(Turn::id)
            .take(1),
    );
    let mut pending = Vec::new();
    for _ in 0..rng.next(3) {
        let turn = targets[rng.next(targets.len() as u64) as usize];
        let (entry, thinking, text) = (eid(fresh.next()), bid(fresh.next()), bid(fresh.next()));
        pending.push(match rng.next(8) {
            0 => Pending::Draft(Box::new(draft(turn, entry, thinking, text, "so far"))),
            1 => Pending::Draft(Box::new(draft(turn, entry, thinking, text, "  "))),
            2 => Pending::Draft(Box::new(finished(
                draft(turn, entry, thinking, text, "done"),
                text,
            ))),
            3 => Pending::Draft(Box::new(stopped(draft(turn, entry, thinking, text, "cut")))),
            4 => Pending::Draft(Box::new(ResponseDraft::new(turn, entry))),
            5 => Pending::Draft(Box::new(finished(
                enriched(draft(turn, entry, thinking, text, "done"), extras(fresh)),
                text,
            ))),
            6 => Pending::Draft(Box::new(enriched(
                draft(turn, entry, thinking, text, "so far"),
                extras(fresh),
            ))),
            _ => Pending::Response(turn, Box::new(spoken(entry, thinking, text))),
        });
    }
    pending
}

fn reused(rng: &mut Lcg, ids: &[Uuid]) -> Uuid {
    ids[rng.next(ids.len() as u64) as usize]
}

pub fn scenario(seed: u64, hostile: Hostile) -> Scenario {
    let conversation = history(seed);
    let mut rng = Lcg::new(seed ^ 0x5eed);
    let mut fresh = Fresh(50_000 + seed * 100);
    let ids = all_ids(&conversation);
    let x = reused(&mut rng, &ids);
    let mut turns = conversation.turns().to_vec();
    let pending = pending_of(&mut rng, &mut fresh, &turns);
    let mut scenario = Scenario {
        turns: Vec::new(),
        stored: None,
        interrupted: Vec::new(),
        interrupted_owner: None,
        pending,
    };
    let first_ai = ai_turns(&turns).into_iter().next();
    let foreign = first_ai.map(|(id, _)| id);
    let (entry, block) = (eid(fresh.next()), bid(fresh.next()));
    let as_entry = EntryId::from_uuid(x).unwrap();
    let as_block = BlockId::from_uuid(x).unwrap();
    match hostile {
        Hostile::Clean => {}
        Hostile::InputEntry | Hostile::InputBlock => {
            let (e, b) = match hostile {
                Hostile::InputEntry => (as_entry, block),
                _ => (entry, as_block),
            };
            let message = HumanMessage::new(e, hid(1), vec![plain_block(b)]).unwrap();
            turns.push(
                InputTurn::new(tid(fresh.next()), vec![message.into()])
                    .unwrap()
                    .into(),
            );
        }
        Hostile::ForeignEntry | Hostile::ForeignBlock => {
            let (e, b) = match hostile {
                Hostile::ForeignEntry => (as_entry, block),
                _ => (entry, as_block),
            };
            let words = TextBlock::new(b, "again").unwrap().into();
            let response = ModelResponse::new(
                e,
                vec![words],
                ResponseEnding::finished(),
                ResponseMetadata::new(),
            )
            .unwrap();
            let turn = AITurn::from_parts(
                tid(fresh.next()),
                aid(2),
                author("Agent 2"),
                vec![response.into()],
            )
            .unwrap();
            turns.push(turn.into());
        }
        Hostile::PendingEntry | Hostile::PendingBlock => {
            if let Some(turn) = foreign {
                let (e, b) = match hostile {
                    Hostile::PendingEntry => (as_entry, block),
                    _ => (entry, as_block),
                };
                let thinking = bid(fresh.next());
                scenario
                    .pending
                    .push(Pending::Response(turn, Box::new(spoken(e, thinking, b))));
            }
        }
        Hostile::InterruptedEntry | Hostile::InterruptedBlock => {
            if let Some(turn) = foreign {
                let (e, b) = match hostile {
                    Hostile::InterruptedEntry => (as_entry, block),
                    _ => (entry, as_block),
                };
                let thinking = bid(fresh.next());
                scenario
                    .interrupted
                    .push(stopped(draft(turn, e, thinking, b, "cut")));
                scenario.interrupted_owner = first_ai.map(|(_, agent)| agent);
            }
        }
        Hostile::InterruptedChild => {
            if let Some(turn) = foreign {
                let held = extras(&mut fresh);
                let citation = BlockId::from_uuid(held.citation.as_uuid()).unwrap();
                let reused = [held.call, held.result, held.child, held.cited, citation]
                    [rng.next(5) as usize];
                let (e, thinking, retry) =
                    (eid(fresh.next()), bid(fresh.next()), eid(fresh.next()));
                scenario.interrupted.push(stopped(enriched(
                    draft(turn, e, thinking, bid(fresh.next()), "cut"),
                    held,
                )));
                scenario.pending.push(Pending::Draft(Box::new(draft(
                    turn,
                    retry,
                    bid(fresh.next()),
                    reused,
                    "again",
                ))));
                scenario.interrupted_owner = first_ai.map(|(_, agent)| agent);
            }
        }
    }
    scenario.stored = Conversation::from_parts(conversation.id(), turns.clone()).ok();
    scenario.turns = turns;
    scenario
}

fn plain_block(id: BlockId) -> br_llm_messages::PlainBlock {
    br_llm_messages::PlainBlock::Text(TextBlock::new(id, "hostile").unwrap())
}
