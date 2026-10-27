mod common;
use common::Raw as _;

use br_llm_messages::{
    ContentBlock, Conversation, ErrorCode, IncomingEntry, InputTurn, ObservedAt, PlainBlock,
    RuntimeToolResult, StructuredBlock, Turn, TurnEntry,
};
use common::turns::tid;
use common::{at, bid, oj, png, text_block};
use serde_json::{Value, json};

#[test]
fn given_the_sum_types_when_asked_for_the_time_then_each_variant_answers_with_its_own() {
    let message = common::turns::human_message(10).with_observed_at(at(1));
    let blocks: Vec<ContentBlock> = vec![
        text_block(20, "x").with_observed_at(at(2)).into(),
        png(21).with_observed_at(at(3)).into(),
    ];

    let entry = TurnEntry::from(message.clone());
    let incoming = IncomingEntry::from(message.clone());
    let turn = Turn::from(
        InputTurn::new(tid(5), vec![message.into()])
            .unwrap()
            .with_observed_at(at(4)),
    );

    assert_eq!(entry.observed_at(), Some(&at(1)));
    assert_eq!(incoming.observed_at(), Some(&at(1)));
    assert_eq!(turn.observed_at(), Some(&at(4)));
    assert_eq!(blocks[0].observed_at(), Some(&at(2)));
    assert_eq!(blocks[1].observed_at(), Some(&at(3)));
    let plain = PlainBlock::try_from(blocks[0].clone()).unwrap();
    assert_eq!(plain.observed_at(), Some(&at(2)));
}

#[test]
fn given_blocks_nested_in_results_when_stamped_then_each_keeps_its_own_time_through_json() {
    let nested = vec![
        PlainBlock::try_from(text_block(11, "a").with_observed_at(at(11))).unwrap(),
        PlainBlock::Image(png(12).with_observed_at(at(12))),
        PlainBlock::StructuredContent(
            StructuredBlock::new(bid(13), oj(json!({"k": 1})))
                .unwrap()
                .with_observed_at(at(13)),
        ),
    ];
    let outer = RuntimeToolResult::new(bid(10), bid(9), common::tool("t"), false, nested)
        .unwrap()
        .with_observed_at(at(10));
    let block = ContentBlock::from(outer);

    let document = block.to_value().unwrap().raw();
    let reloaded = ContentBlock::from_value(oj(document.clone())).unwrap();

    assert_eq!(reloaded, block);
    let times: Vec<_> = reloaded
        .children()
        .iter()
        .map(|child| child.observed_at().map(ObservedAt::as_str))
        .collect();
    assert_eq!(
        times,
        [
            Some(at(11).as_str()),
            Some(at(12).as_str()),
            Some(at(13).as_str())
        ]
    );
    for index in 0..3 {
        let mut broken = document.clone();
        broken["content"][index]["observed_at"] = json!("2026-10-23T08:00:00+00:00");
        let error = ContentBlock::from_value(oj(broken)).unwrap_err();
        assert_eq!(error.code(), ErrorCode::InvalidContent, "child {index}");
    }
}

#[test]
fn given_a_stored_conversation_with_a_bad_time_at_any_depth_when_loaded_then_invalid_content() {
    let golden: Value =
        serde_json::from_str(include_str!("golden/rich_conversation.json")).unwrap();
    let mut paths = Vec::new();
    collect(&golden, String::new(), &mut paths);
    assert!(paths.len() >= 20, "{paths:?}");

    for path in paths {
        let mut broken = golden.clone();
        *broken.pointer_mut(&path).unwrap() = json!("2026-10-23T08:00:00.5+02:00");
        let error = Conversation::from_value(oj(broken)).unwrap_err();
        assert_eq!(error.code(), ErrorCode::InvalidContent, "{path}");
    }
}

fn collect(value: &Value, path: String, found: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                let here = format!("{path}/{key}");
                if key == "observed_at" {
                    found.push(here);
                } else {
                    collect(child, here, found);
                }
            }
        }
        Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                collect(child, format!("{path}/{index}"), found);
            }
        }
        _ => {}
    }
}

#[test]
fn given_fraction_digits_when_an_input_turn_round_trips_then_the_text_is_not_normalized() {
    let kept = [
        "2026-10-23T08:00:00Z",
        "2026-10-23T08:00:00.500Z",
        "2026-10-23T08:00:00.000000001Z",
    ];
    for text in kept {
        let message =
            common::turns::human_message(10).with_observed_at(ObservedAt::new(text).unwrap());
        let turn = InputTurn::new(tid(1), vec![message.into()]).unwrap();

        let reloaded = InputTurn::from_json(&turn.to_json().unwrap()).unwrap();

        let IncomingEntry::HumanMessage(entry) = &reloaded.entries()[0] else {
            unreachable!()
        };
        assert_eq!(entry.observed_at().map(ObservedAt::as_str), Some(text));
    }
}
