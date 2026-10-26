mod common;
use common::Raw as _;
mod projected_thread_support;

use br_llm_messages::{
    Citation, CitationSource, ErrorCode, IncomingEntry, PlainBlock, TextBlock, TextRange, Turn,
};
use common::{aid, bid, eid, refused};
use projected_thread_support::*;
use serde_json::{Value, json};

fn block_json(kind: &str, extra: Value) -> Value {
    let mut block = match kind {
        "image" => json!({"type": "image", "id": bid(700).to_string(),
            "source": {"type": "inline", "media_type": "image/png", "data": "QUJD"}}),
        "structured" => json!({"type": "structured_content", "id": bid(700).to_string(),
            "value": {"a": 1}}),
        _ => json!({"type": "text", "id": bid(700).to_string(), "text": "x"}),
    };
    for (key, value) in extra.as_object().unwrap() {
        block[key] = value.clone();
    }
    block
}

#[test]
fn given_text_image_and_structured_blocks_in_an_output_when_loaded_then_they_are_accepted() {
    let content = vec![
        block_json("text", json!({})),
        block_json("image", json!({"id": bid(701).to_string()})),
        block_json("structured", json!({"id": bid(702).to_string()})),
    ];
    let mut document = document();
    document["turns"][2]["entries"][0]["content"] = Value::Array(content);

    let loaded = load(document).unwrap();

    let Turn::InputTurn(input) = &loaded.turns()[2] else {
        panic!("input turn expected");
    };
    let IncomingEntry::AgentOutput(output) = &input.entries()[0] else {
        panic!("output expected");
    };
    assert_eq!(output.content().len(), 3);
}

#[test]
fn given_decorated_or_foreign_or_empty_content_in_an_output_when_loaded_then_it_is_refused() {
    let at = json!({"observed_at": "2026-10-23T08:00:00Z"});
    let citation = Citation::new(common::cid(750), CitationSource::new("doc").unwrap())
        .with_range(TextRange::new(0, 1).unwrap());
    let cited = PlainBlock::try_from(
        TextBlock::new(bid(705), "x")
            .unwrap()
            .with_citations(vec![citation])
            .unwrap(),
    )
    .unwrap()
    .to_value()
    .unwrap()
    .raw();
    let thinking = json!({"type": "thinking", "id": bid(703).to_string(), "text": "plan"});
    let call = json!({"type": "runtime_tool_call", "id": bid(704).to_string(),
        "name": "lookup", "arguments": {}});
    let cases = [
        (
            vec![block_json("image", at.clone())],
            ErrorCode::DisallowedKind,
        ),
        (
            vec![block_json("structured", at.clone())],
            ErrorCode::DisallowedKind,
        ),
        (vec![block_json("text", at)], ErrorCode::DisallowedKind),
        (vec![cited], ErrorCode::DisallowedKind),
        (
            vec![block_json("text", json!({"item_ref": "msg_1"}))],
            ErrorCode::DisallowedKind,
        ),
        (vec![thinking], ErrorCode::DisallowedKind),
        (vec![call], ErrorCode::DisallowedKind),
        (vec![], ErrorCode::InvalidContent),
        (
            vec![block_json("text", json!({"text": "  "}))],
            ErrorCode::InvalidContent,
        ),
    ];
    for (content, code) in cases {
        let mut document = document();
        document["turns"][2]["entries"][0]["content"] = Value::Array(content.clone());

        let error = refused(load(document));

        assert_eq!(error.code(), code, "{content:?}");
    }
}

#[test]
fn given_recorded_outputs_and_partials_in_any_order_and_number_when_loaded_then_they_are_accepted()
{
    let recorded = IncomingEntry::from(common::turns::human_message(950))
        .to_value()
        .unwrap()
        .raw();
    let recorded2 = IncomingEntry::from(common::turns::human_message(951))
        .to_value()
        .unwrap()
        .raw();
    let partial = document()["turns"][3]["entries"][0].clone();
    let mut output = document()["turns"][2]["entries"][0].clone();
    output["agent"] = partial["agent"].clone();
    output["id"] = json!(eid(970).to_string());
    output["content"][0]["id"] = json!(bid(971).to_string());
    let mut second = partial.clone();
    second["id"] = json!(eid(960).to_string());
    second["content"][0]["id"] = json!(bid(961).to_string());
    let cases = [
        vec![output.clone(), recorded.clone()],
        vec![recorded.clone(), output.clone(), recorded2],
        vec![partial.clone(), output],
        vec![partial, second],
    ];
    for entries in cases {
        let mut document = document();
        document["turns"][3]["entries"] = Value::Array(entries);

        let loaded = load(document);

        assert!(loaded.is_ok(), "{loaded:?}");
    }
}

#[test]
fn given_outputs_of_two_agents_or_two_authors_in_one_turn_when_loaded_then_invalid_record_structure()
 {
    let output = document()["turns"][2]["entries"][0].clone();
    let mut other_agent = output.clone();
    other_agent["id"] = json!(eid(960).to_string());
    other_agent["agent"] = json!(aid(4).to_string());
    other_agent["content"][0]["id"] = json!(bid(961).to_string());
    let mut other_author = other_agent.clone();
    other_author["agent"] = output["agent"].clone();
    other_author["author"] = json!("Someone else");
    for second in [other_agent, other_author] {
        let mut document = document();
        document["turns"][3]["entries"] = json!([output.clone(), second]);

        let error = refused(load(document));

        assert_eq!(error.code(), ErrorCode::InvalidRecordStructure);
        assert_eq!(error.location(), Some("turns[3].entries[1]"));
    }
}
