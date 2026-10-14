//! A conversation stored piece by piece: a user input, a step and a tool
//! result each serialize and load on their own, and pushing them back in
//! their order rebuilds a conversation equal to the original.

use super::*;
use crate::block::ToolResult;
use crate::test_walk::{Piece, walk};
use crate::turn::TurnItem;
use crate::value::{Author, ToolCallId, ToolName};

/// A piece as a store keeps it: each value serialized on its own.
enum Row {
    Input(String),
    Turn {
        id: String,
        author: Option<String>,
        first: String,
    },
    Step {
        turn: String,
        step: String,
    },
    Result {
        turn: String,
        result: String,
    },
}

fn json<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_string(value).unwrap()
}

fn load<T: serde::de::DeserializeOwned>(text: &str) -> T {
    serde_json::from_str(text).unwrap()
}

fn store(piece: &Piece) -> Row {
    match piece {
        Piece::Input(input) => Row::Input(json(input)),
        Piece::Turn { id, author, first } => Row::Turn {
            id: json(id),
            author: author.as_ref().map(json),
            first: json(first),
        },
        Piece::Step { turn, step } => Row::Step {
            turn: json(turn),
            step: json(step),
        },
        Piece::Result { turn, result } => Row::Result {
            turn: json(turn),
            result: json(result),
        },
    }
}

fn reload(row: &Row) -> Piece {
    match row {
        Row::Input(input) => Piece::Input(load(input)),
        Row::Turn { id, author, first } => Piece::Turn {
            id: load(id),
            author: author.as_deref().map(load),
            first: load(first),
        },
        Row::Step { turn, step } => Piece::Step {
            turn: load(turn),
            step: load(step),
        },
        Row::Result { turn, result } => Piece::Result {
            turn: load(turn),
            result: load(result),
        },
    }
}

fn rebuild(rows: &[Row]) -> Conversation {
    let mut conversation = Conversation::new();
    for row in rows {
        reload(row).push_onto(&mut conversation);
    }
    conversation
}

/// The pieces of a conversation, entry by entry: an input, or a turn started
/// with its first step followed by its later steps and results.
fn pieces_of(conversation: &Conversation) -> Vec<Piece> {
    let mut pieces = Vec::new();
    for entry in conversation.entries() {
        let turn = match entry {
            Entry::UserInput(input) => {
                pieces.push(Piece::Input(input.clone()));
                continue;
            }
            Entry::Turn(turn) => turn,
        };
        for (position, item) in turn.items().iter().enumerate() {
            match (position, item) {
                (0, TurnItem::Step(first)) => pieces.push(Piece::Turn {
                    id: turn.id().clone(),
                    author: turn.author().cloned(),
                    first: first.clone(),
                }),
                (_, TurnItem::Step(step)) => pieces.push(Piece::Step {
                    turn: turn.id().clone(),
                    step: step.clone(),
                }),
                (_, TurnItem::ToolResults(results)) => {
                    for result in results.results() {
                        pieces.push(Piece::Result {
                            turn: turn.id().clone(),
                            result: result.clone(),
                        });
                    }
                }
            }
        }
    }
    pieces
}

const SEEDS: u64 = 200;
const LENGTH: usize = 60;

#[test]
fn given_walks_when_pieces_stored_and_pushed_back_in_their_order_then_equal_conversation() {
    let mut most_open_turns = 0;
    let mut interleaved_results = 0;
    for seed in 0..SEEDS {
        let (original, pieces) = walk(seed, LENGTH, |_, _| {});
        let rows: Vec<Row> = pieces.iter().map(store).collect();

        let rebuilt = rebuild(&rows);

        assert_eq!(rebuilt, original, "seed {seed}");
        assert_eq!(json(&rebuilt), json(&original), "seed {seed}");
        most_open_turns = most_open_turns.max(original.open_turns().count());
        interleaved_results += pieces
            .windows(2)
            .filter(|pair| match pair {
                [Piece::Input(_), Piece::Result { .. }] => true,
                [
                    Piece::Turn { id: other, .. } | Piece::Step { turn: other, .. },
                    Piece::Result { turn, .. },
                ] => other != turn,
                _ => false,
            })
            .count();
    }
    assert!(most_open_turns >= 3, "walks keep several turns open");
    assert!(interleaved_results > 0, "walks interleave results");
}

#[test]
fn given_loaded_conversations_when_decomposed_and_pushed_back_entry_by_entry_then_equal() {
    for seed in 0..SEEDS {
        let (original, _) = walk(seed, LENGTH, |_, _| {});
        let loaded: Conversation = load(&json(&original));
        let rows: Vec<Row> = pieces_of(&loaded).iter().map(store).collect();

        let rebuilt = rebuild(&rows);

        assert_eq!(rebuilt, original, "seed {seed}");
        assert_eq!(json(&rebuilt), json(&original), "seed {seed}");
    }
}

/// `after` only appends to `before`: every earlier entry is unchanged, but
/// for a turn that received a step or results at its end.
fn only_appends(before: &Conversation, after: &Conversation) -> bool {
    let (old, new) = (before.entries(), after.entries());
    new.len() >= old.len()
        && old.iter().zip(new).all(|pair| match pair {
            (Entry::Turn(old), Entry::Turn(new)) if old.id() == new.id() => {
                turn_only_appends(old.items(), new.items())
            }
            (old, new) => old == new,
        })
}

fn turn_only_appends(old: &[TurnItem], new: &[TurnItem]) -> bool {
    let Some((last, kept)) = old.split_last() else {
        return new.is_empty();
    };
    new.len() >= old.len()
        && new[..kept.len()] == *kept
        && match (last, &new[kept.len()]) {
            (TurnItem::ToolResults(old), TurnItem::ToolResults(new)) => {
                new.results().starts_with(old.results())
            }
            (old, new) => old == new,
        }
}

#[test]
fn given_walks_when_each_piece_pushed_then_every_earlier_position_keeps_its_content() {
    for seed in 0..SEEDS {
        walk(seed, LENGTH, |before, after| {
            assert!(only_appends(before, after), "seed {seed}");
        });
    }
}

#[test]
fn given_standalone_pieces_breaking_their_rules_when_loaded_then_refused() {
    let step = serde_json::json!({
        "content": [ { "type": "tool_call", "id": "c1", "name": "web.search", "arguments": {} } ],
        "stop_reason": { "type": "awaiting_tool_results" }
    });
    assert!(serde_json::from_value::<Step>(step).is_err());
    let result =
        serde_json::json!({ "tool_call_id": "", "tool_name": "search", "is_error": false });
    assert!(serde_json::from_value::<ToolResult>(result).is_err());
    let result = serde_json::json!({ "tool_call_id": "c1", "tool_name": "a b", "is_error": false });
    assert!(serde_json::from_value::<ToolResult>(result).is_err());
    let input = serde_json::json!({ "source": { "type": "human" }, "content": [] });
    assert!(serde_json::from_value::<UserInput>(input).is_err());
}

#[test]
fn given_stored_pieces_construction_refuses_when_pushed_back_then_refused_by_the_same_rule() {
    let author = Author::new("id-agent-a", "agent-a").unwrap();
    let first: Step = load(
        r#"{"content":[{"type":"tool_call","id":"c1","name":"search","arguments":{}}],"stop_reason":{"type":"awaiting_tool_results"}}"#,
    );
    let mut conversation = Conversation::new();
    conversation
        .push_turn(crate::turn::Turn::new(
            load(r#""t1""#),
            Some(author.clone()),
            first.clone(),
        ))
        .unwrap();

    let wrong_tool: ToolResult =
        load(r#"{"tool_call_id":"c1","tool_name":"fetch","is_error":false}"#);
    assert_eq!(
        conversation.push_result(&load(r#""t1""#), wrong_tool),
        Err(MessageError::ToolNameMismatch {
            id: ToolCallId::new("c1").unwrap(),
            call: ToolName::new("search").unwrap(),
            result: ToolName::new("fetch").unwrap(),
        })
    );
    assert_eq!(
        conversation.push_turn(crate::turn::Turn::new(load(r#""t2""#), Some(author), first)),
        Err(MessageError::DuplicateToolCallIdInConversation {
            id: ToolCallId::new("c1").unwrap()
        })
    );
}
