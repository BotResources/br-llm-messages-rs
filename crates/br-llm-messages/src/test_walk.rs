//! Test support: seeded random walks that grow a conversation through its
//! public construction API, piece by piece, recording each piece in the
//! order it was pushed. Several turns stay open at once and results reach
//! their turns interleaved with other entries.

use crate::block::{
    AssistantBlock, Image, RedactedThinking, Thinking, ToolCall, ToolResult, ToolResultBlock,
    UserBlock,
};
use crate::conversation::Conversation;
use crate::step::Step;
use crate::stop_reason::StopReason;
use crate::turn::TurnState;
use crate::usage::Usage;
use crate::user_input::{UserInput, UserSource};
use crate::value::{
    Author, Base64Data, ImageMime, ModelId, Signature, Text, ToolCallId, ToolName, TurnId,
};

/// One piece appended to a conversation, in the words of the intent: an
/// input, a turn started with its first step, or a step or a result reaching
/// its turn by id.
#[derive(Debug, Clone)]
pub(crate) enum Piece {
    Input(UserInput),
    Turn {
        id: TurnId,
        author: Option<Author>,
        first: Step,
    },
    Step {
        turn: TurnId,
        step: Step,
    },
    Result {
        turn: TurnId,
        result: ToolResult,
    },
}

impl Piece {
    pub(crate) fn push_onto(self, conversation: &mut Conversation) {
        match self {
            Piece::Input(input) => conversation.push_input(input),
            Piece::Turn { id, author, first } => conversation
                .push_turn(crate::turn::Turn::new(id, author, first))
                .unwrap(),
            Piece::Step { turn, step } => conversation.push_step(&turn, step).unwrap(),
            Piece::Result { turn, result } => conversation.push_result(&turn, result).unwrap(),
        }
    }
}

/// The agents that author turns in a walk; a walk also writes turns without
/// author.
pub(crate) fn agents() -> Vec<Author> {
    ["a", "b", "c"]
        .iter()
        .map(|name| Author::new(format!("id-agent-{name}"), format!("agent-{name}")).unwrap())
        .collect()
}

/// A xorshift64* generator: deterministic, so a failing seed replays.
pub(crate) struct Rng(u64);

impl Rng {
    pub(crate) fn new(seed: u64) -> Self {
        Self(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }

    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    pub(crate) fn below(&mut self, bound: usize) -> usize {
        (self.next() % bound as u64) as usize
    }

    fn chance(&mut self, percent: usize) -> bool {
        self.below(100) < percent
    }

    fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.below(items.len())]
    }
}

/// A walk of `length` pieces from `seed`. `after_each` sees the conversation
/// before and after every push.
pub(crate) fn walk(
    seed: u64,
    length: usize,
    mut after_each: impl FnMut(&Conversation, &Conversation),
) -> (Conversation, Vec<Piece>) {
    let mut rng = Rng::new(seed);
    let mut walker = Walker::default();
    let mut conversation = Conversation::new();
    let mut pieces = Vec::with_capacity(length);
    for _ in 0..length {
        let piece = walker.next_piece(&mut rng, &conversation);
        let before = conversation.clone();
        piece.clone().push_onto(&mut conversation);
        after_each(&before, &conversation);
        pieces.push(piece);
    }
    (conversation, pieces)
}

#[derive(Default)]
struct Walker {
    turns: usize,
    calls: usize,
}

impl Walker {
    fn next_piece(&mut self, rng: &mut Rng, conversation: &Conversation) -> Piece {
        let awaiting_results: Vec<(TurnId, Vec<ToolCallId>)> = conversation
            .open_turns()
            .filter_map(|turn| match turn.state() {
                TurnState::AwaitingToolResults { pending } => Some((turn.id().clone(), pending)),
                TurnState::AwaitingStep | TurnState::Finished { .. } => None,
            })
            .collect();
        let awaiting_step: Vec<TurnId> = conversation
            .open_turns()
            .filter(|turn| turn.state() == TurnState::AwaitingStep)
            .map(|turn| turn.id().clone())
            .collect();

        let roll = rng.below(100);
        if roll < 40 && !awaiting_results.is_empty() {
            let (turn, pending) = rng.pick(&awaiting_results).clone();
            let id = rng.pick(&pending).clone();
            let name = conversation
                .turn(&turn)
                .and_then(|turn| turn.tool_calls().find(|call| call.id == id))
                .map(|call| call.name.clone())
                .unwrap();
            return Piece::Result {
                turn,
                result: tool_result(rng, id, name),
            };
        }
        if roll < 60 && !awaiting_step.is_empty() {
            let turn = rng.pick(&awaiting_step).clone();
            return Piece::Step {
                turn,
                step: self.step(rng),
            };
        }
        if roll < 80 {
            self.turns += 1;
            let mut authors: Vec<Option<Author>> = agents().into_iter().map(Some).collect();
            authors.push(None);
            return Piece::Turn {
                id: TurnId::new(format!("turn-{}", self.turns)).unwrap(),
                author: rng.pick(&authors).clone(),
                first: self.step(rng),
            };
        }
        Piece::Input(input(rng))
    }

    fn step(&mut self, rng: &mut Rng) -> Step {
        let mut content = Vec::new();
        if rng.chance(40) {
            content.push(AssistantBlock::Thinking(Thinking {
                text: if rng.chance(20) {
                    String::new()
                } else {
                    "hmm".to_owned()
                },
                signature: rng
                    .chance(50)
                    .then(|| Signature::new("c2lnbmF0dXJl").unwrap()),
            }));
        }
        if rng.chance(10) {
            content.push(AssistantBlock::RedactedThinking(
                RedactedThinking::new("b3BhcXVl").unwrap(),
            ));
        }
        if rng.chance(70) {
            content.push(AssistantBlock::Text {
                text: Text::new(format!("reply {}", rng.below(1000))).unwrap(),
            });
        }
        if rng.chance(15) {
            content.push(AssistantBlock::Structured {
                value: serde_json::json!({ "n": rng.below(10), "list": [1, "two", null] }),
            });
        }
        let calls = if rng.chance(55) { 1 + rng.below(3) } else { 0 };
        for _ in 0..calls {
            self.calls += 1;
            content.push(AssistantBlock::ToolCall(ToolCall {
                id: ToolCallId::new(format!("call-{}", self.calls)).unwrap(),
                name: ToolName::new(*rng.pick(&["search", "fetch_page", "run-code"])).unwrap(),
                arguments: serde_json::json!({ "q": rng.below(100) }),
            }));
        }
        if content.is_empty() {
            content.push(AssistantBlock::Text {
                text: Text::new("ok").unwrap(),
            });
        }
        let stop_reason = if calls > 0 {
            StopReason::AwaitingToolResults
        } else {
            rng.pick(&[
                StopReason::EndTurn,
                StopReason::EndTurn,
                StopReason::MaxTokens,
                StopReason::Refusal,
                StopReason::Other {
                    reason: String::new(),
                },
            ])
            .clone()
        };
        let usage = rng.chance(50).then(|| Usage {
            input_tokens: rng.below(500) as u64,
            output_tokens: rng.below(500) as u64,
            cache_read_tokens: rng.chance(50).then_some(3),
            cache_write_tokens: None,
            thinking_tokens: rng.chance(30).then_some(7),
        });
        let model = rng.chance(50).then(|| ModelId::new("model-x").unwrap());
        Step::new(content, stop_reason, usage, model).unwrap()
    }
}

fn tool_result(rng: &mut Rng, id: ToolCallId, name: ToolName) -> ToolResult {
    let content = (0..rng.below(3))
        .map(|n| ToolResultBlock::text(Text::new(format!("result {n}")).unwrap()))
        .collect();
    ToolResult::new(id, name, content, rng.chance(20))
}

fn input(rng: &mut Rng) -> UserInput {
    let source = if rng.chance(70) {
        UserSource::Human
    } else {
        UserSource::runtime(*rng.pick(&["tick", " ", "task notification"])).unwrap()
    };
    let author = rng
        .chance(60)
        .then(|| Author::new("id-user", "Alice").unwrap());
    let mut content = Vec::new();
    if rng.chance(85) {
        content.push(UserBlock::text(
            Text::new(format!("question {}", rng.below(1000))).unwrap(),
        ));
    }
    if content.is_empty() || rng.chance(20) {
        content.push(UserBlock::Image(Image {
            mime: ImageMime::Png,
            data: Base64Data::new("aGk=").unwrap(),
        }));
    }
    UserInput::new(source, author, content).unwrap()
}
