use std::collections::{HashMap, HashSet};

use br_llm_messages::{
    AITurn, AgentMail, Citation, CitationSource, Conversation, EntryId, EntryRecord, ImageBlock,
    InputTurn, ItemRef, RecordSet, ResponseEnding, StructuredBlock, TextBlock, TextRange,
    ThinkingBlock, TurnEntryKind, TurnId, TurnKind,
};
use serde_json::json;

use super::turns::{calling, close, delivery, human_message, response, tid};
use super::{aid, at, author, bid, cid, cvid, eid, oj, plain_text, png, signature};

pub struct Lcg(u64);

impl Lcg {
    pub fn new(seed: u64) -> Self {
        Lcg(seed.wrapping_mul(7919).wrapping_add(13))
    }

    pub fn next(&mut self, bound: u64) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 33) % bound
    }
}

fn text(rng: &mut Lcg, id: &mut impl FnMut() -> u64) -> TextBlock {
    let mut block = TextBlock::new(bid(id()), "some text").unwrap();
    if rng.next(3) == 0 {
        let citation = Citation::new(cid(id()), CitationSource::new("doc").unwrap())
            .with_range(TextRange::new(0, 4).unwrap());
        block = block.with_citations(vec![citation]).unwrap();
    }
    if rng.next(3) == 0 {
        block = block
            .with_item_ref(ItemRef::new("msg_1").unwrap())
            .with_replay(signature("sig"))
            .with_observed_at(at(3));
    }
    block
}

fn image(rng: &mut Lcg, id: &mut impl FnMut() -> u64) -> ImageBlock {
    let block = png(id());
    if rng.next(2) == 0 {
        return block.with_observed_at(at(4));
    }
    block
}

fn structured(rng: &mut Lcg, id: &mut impl FnMut() -> u64) -> StructuredBlock {
    let block = StructuredBlock::new(bid(id()), oj(json!({"k": [1, 2]}))).unwrap();
    if rng.next(2) == 0 {
        return block.with_observed_at(at(5));
    }
    block
}

fn blocks(rng: &mut Lcg, id: &mut impl FnMut() -> u64) -> Vec<br_llm_messages::ContentBlock> {
    match rng.next(5) {
        0 => vec![text(rng, id).into()],
        3 => vec![image(rng, id).into(), structured(rng, id).into()],
        4 => vec![structured(rng, id).into()],
        1 => vec![
            ThinkingBlock::new(bid(id()), "private plan")
                .unwrap()
                .into(),
            text(rng, id).into(),
            image(rng, id).into(),
            structured(rng, id).into(),
            text(rng, id).into(),
        ],
        _ => vec![
            ThinkingBlock::new(bid(id()), "only thinking")
                .unwrap()
                .into(),
        ],
    }
}

fn ai_turn(rng: &mut Lcg, id: &mut impl FnMut() -> u64) -> AITurn {
    let agent = 1 + rng.next(3);
    let mut turn = AITurn::open(tid(id()), aid(agent), author(&format!("Agent {agent}")));
    for _ in 0..(1 + rng.next(3)) {
        let incoming = human_message(id());
        turn.push_entry(incoming.into()).unwrap();
        if rng.next(3) == 0 {
            let mail = AgentMail::new(
                eid(id()),
                aid(4),
                author("Mailer"),
                cvid(1),
                vec![plain_text(id(), "mail")],
            )
            .unwrap();
            turn.push_entry(mail.into()).unwrap();
        }
        if rng.next(4) == 0 {
            let call = id();
            let result = id();
            turn.push_entry(calling(id(), &[(call, "lookup")])).unwrap();
            turn.push_entry(delivery(id(), &[(result, call, "lookup")]))
                .unwrap();
        }
        let content = blocks(rng, id);
        turn.push_entry(response(id(), content, ResponseEnding::finished()))
            .unwrap();
    }
    if rng.next(2) == 0 {
        turn.push_entry(close(id(), br_llm_messages::ClosureReason::completed()))
            .unwrap();
    }
    turn
}

pub fn history(seed: u64) -> Conversation {
    let mut rng = Lcg::new(seed);
    let mut next = 1000u64;
    let mut id = || {
        next += 1;
        next
    };
    let mut conversation = Conversation::new(cvid(900));
    for _ in 0..(2 + rng.next(5)) {
        if rng.next(3) == 0 {
            let turn = tid(id());
            let entries = (0..(1 + rng.next(3)))
                .map(|_| human_message(id()).into())
                .collect();
            conversation
                .push_turn(InputTurn::new(turn, entries).unwrap().into())
                .unwrap();
        } else {
            let turn = ai_turn(&mut rng, &mut id);
            conversation.push_turn(turn.into()).unwrap();
        }
    }
    conversation
}

pub fn reduced_rows(
    full: &RecordSet,
    agent: u64,
    seed: u64,
    keep: &dyn Fn(EntryId) -> bool,
) -> RecordSet {
    let mut rng = Lcg::new(seed ^ 0xabcdef);
    let others: HashSet<TurnId> = full
        .turns()
        .iter()
        .filter(|row| row.kind() == TurnKind::AITurn && row.agent() != Some(aid(agent)))
        .map(|row| row.id())
        .collect();
    let owners: HashMap<_, _> = full
        .entries()
        .iter()
        .map(|row| (row.id(), row.turn()))
        .collect();
    let mut entries = Vec::new();
    for row in full.entries() {
        if !others.contains(&row.turn()) {
            entries.push(row.clone());
            continue;
        }
        if row.kind() != TurnEntryKind::ModelResponse {
            if row.kind().is_incoming() && keep(row.id()) {
                entries.push(row.clone());
            }
            continue;
        }
        let has_usable = full.blocks().iter().any(|block| {
            block.entry() == row.id() && block.kind().is_plain() && block.parent().is_none()
        });
        if !has_usable && rng.next(2) == 0 {
            continue;
        }
        entries.push(if rng.next(2) == 0 {
            EntryRecord::new(row.id(), row.turn(), row.kind(), row.state(), oj(json!({})))
        } else {
            row.clone()
        });
    }
    let kept: HashSet<_> = entries.iter().map(|row| row.id()).collect();
    let blocks = full
        .blocks()
        .iter()
        .filter(|row| {
            let owner = owners[&row.entry()];
            !others.contains(&owner)
                || (kept.contains(&row.entry()) && row.kind().is_plain() && row.parent().is_none())
        })
        .cloned()
        .collect();
    RecordSet::new()
        .with_turns(full.turns().to_vec())
        .with_entries(entries)
        .with_blocks(blocks)
}
