mod common;

use br_llm_messages::{
    Citation, CitationSource, ContentBlock, ContentBlockKind, ErrorCode, ExternalId, PlainBlock,
    RedactedThinkingBlock, RuntimeToolCall, RuntimeToolResult, StructuredBlock, TextBlock,
    TextRange, ThinkingBlock,
};
use common::Raw as _;
use common::{bid, cid, envelope, plain_text, png, refused, signature, text_block, tool, v7};
use serde_json::{Value, json};

fn cite(n: u64, start: u64, end: u64) -> Citation {
    Citation::new(cid(n), CitationSource::new("doc").unwrap())
        .with_range(TextRange::new(start, end).unwrap())
}

fn result_with(
    id: u64,
    content: Vec<PlainBlock>,
) -> Result<RuntimeToolResult, br_llm_messages::Error> {
    RuntimeToolResult::new(bid(id), bid(99), tool("search"), false, content)
}

#[test]
fn given_a_nested_content_block_reusing_the_result_id_when_built_then_reused_identity() {
    let error = refused(result_with(1, vec![plain_text(1, "x")]));

    assert_eq!(error.code(), ErrorCode::ReusedIdentity);
    assert_eq!(error.object_id(), Some(v7(1)));
    assert_eq!(error.location(), Some("content[0]"));
}

#[test]
fn given_two_nested_blocks_sharing_an_id_when_built_then_reused_identity() {
    let error = refused(result_with(
        1,
        vec![plain_text(2, "a"), plain_text(3, "b"), plain_text(3, "c")],
    ));

    assert_eq!(error.code(), ErrorCode::ReusedIdentity);
    assert_eq!(error.object_id(), Some(v7(3)));
    assert_eq!(error.location(), Some("content[2]"));
}

#[test]
fn given_a_text_citation_reusing_the_block_id_or_another_citation_when_built_then_reused_identity()
{
    let own = TextBlock::new(bid(1), "hello").and_then(|b| b.with_citations(vec![cite(1, 0, 2)]));
    let twin = TextBlock::new(bid(1), "hello")
        .and_then(|b| b.with_citations(vec![cite(5, 0, 2), cite(5, 1, 3)]));

    let own = refused(own);
    let twin = refused(twin);

    assert_eq!(own.code(), ErrorCode::ReusedIdentity);
    assert_eq!(own.object_id(), Some(v7(1)));
    assert_eq!(twin.code(), ErrorCode::ReusedIdentity);
    assert_eq!(twin.object_id(), Some(v7(5)));
    assert_eq!(twin.location(), Some("citations[1]"));
}

#[test]
fn given_a_container_where_a_nested_id_repeats_a_sibling_block_when_checked_then_reused_identity_with_path()
 {
    let call =
        ContentBlock::from(RuntimeToolCall::new(bid(1), tool("t"), common::oj(json!({}))).unwrap());
    let nested = ContentBlock::from(result_with(2, vec![plain_text(1, "dup")]).unwrap());

    let error = refused(ContentBlock::ensure_distinct_ids(&[call, nested]));

    assert_eq!(error.code(), ErrorCode::ReusedIdentity);
    assert_eq!(error.object_id(), Some(v7(1)));
    assert_eq!(error.location(), Some("[1].content[0]"));
}

#[test]
fn given_a_citation_id_equal_to_a_sibling_block_id_when_checked_then_reused_identity() {
    let cited = TextBlock::new(bid(1), "hello")
        .unwrap()
        .with_citations(vec![cite(2, 0, 2)])
        .unwrap();
    let sibling = ThinkingBlock::new(bid(2), "t").unwrap();

    let error = refused(ContentBlock::ensure_distinct_ids(&[
        cited.into(),
        sibling.into(),
    ]));

    assert_eq!(error.code(), ErrorCode::ReusedIdentity);
    assert_eq!(error.object_id(), Some(v7(2)));
}

#[test]
fn given_distinct_ids_everywhere_when_checked_then_accepted() {
    let blocks = [
        ContentBlock::from(text_block(1, "a")),
        ContentBlock::from(png(2)),
        ContentBlock::from(result_with(3, vec![plain_text(4, "n")]).unwrap()),
    ];

    assert!(ContentBlock::ensure_distinct_ids(&blocks).is_ok());
    assert_eq!(blocks[2].id(), bid(3));
    assert_eq!(blocks[2].children().len(), 1);
    assert!(blocks[0].children().is_empty());
}

#[test]
fn given_blank_text_when_built_then_invalid_content_and_nothing_is_normalized() {
    for blank in ["", " ", "\n\t \u{00a0}"] {
        assert_eq!(
            refused(TextBlock::new(bid(1), blank)).code(),
            ErrorCode::InvalidContent
        );
    }
    let padded = TextBlock::new(bid(1), "  keep\n").unwrap();

    assert_eq!(padded.text().as_str(), "  keep\n");
}

#[test]
fn given_a_citation_range_past_the_text_when_built_then_invalid_content() {
    let past = TextBlock::new(bid(1), "é😀é").and_then(|b| b.with_citations(vec![cite(2, 0, 4)]));
    let exact = TextBlock::new(bid(1), "é😀é")
        .unwrap()
        .with_citations(vec![cite(2, 0, 3)])
        .unwrap();

    assert_eq!(refused(past).code(), ErrorCode::InvalidContent);
    assert_eq!(exact.citations().len(), 1);
    assert_eq!(exact.citations()[0].range().unwrap().end(), 3);
}

#[test]
fn given_a_built_text_block_when_inspected_then_text_and_replay_are_exactly_what_was_given() {
    let block = TextBlock::new(bid(1), "hi")
        .unwrap()
        .with_replay(signature("s"));

    assert_eq!(block.text().as_str(), "hi");
    assert_eq!(block.replay(), Some(&signature("s")));
    assert_eq!(block.id(), bid(1));
}

#[test]
fn given_images_when_built_then_media_type_and_data_are_kept() {
    let image = png(1);

    assert_eq!(
        serde_json::to_value(image.source()).unwrap(),
        json!({"type": "inline", "media_type": "image/png", "data": "QUJD"})
    );
}

#[test]
fn given_any_json_value_when_structured_then_kept_exactly() {
    let values: Vec<Value> = vec![
        Value::Null,
        json!(0),
        json!(true),
        json!("s"),
        json!([]),
        json!({"a": {"b": [1, 2.5, null]}}),
        json!(u64::MAX),
    ];

    for value in values {
        let block = StructuredBlock::new(bid(1), common::oj(value.clone())).unwrap();
        let again = ContentBlock::from_json(&ContentBlock::from(block).to_json().unwrap()).unwrap();

        assert!(matches!(&again, ContentBlock::StructuredContent(b) if b.value().raw() == value));
    }
}

#[test]
fn given_empty_thinking_text_when_it_carries_replay_then_it_is_kept_verbatim() {
    let replay_only = ThinkingBlock::replay_only(bid(1), envelope(json!({"enc": "AAA="})));
    let verbose = ThinkingBlock::new(bid(3), "  x\n").unwrap();

    assert_eq!(replay_only.text(), "");
    assert_eq!(
        replay_only.replay(),
        Some(&envelope(json!({"enc": "AAA="})))
    );
    assert_eq!(verbose.text(), "  x\n");
    assert_eq!(verbose.replay(), None);
}

#[test]
fn given_empty_thinking_text_and_no_replay_when_built_then_invalid_content() {
    let error = refused(ThinkingBlock::new(bid(2), ""));

    assert_eq!(error.code(), ErrorCode::InvalidContent);
    assert_eq!(error.object_id(), Some(v7(2)));
    assert_eq!(error.location(), Some("text"));
}

#[test]
fn given_whitespace_only_thinking_when_built_then_it_is_kept_because_only_exactly_empty_is_refused()
{
    let block = ThinkingBlock::new(bid(2), " \n").unwrap();

    assert_eq!(block.text(), " \n");
}

#[test]
fn given_empty_thinking_without_replay_in_json_when_loaded_then_invalid_content() {
    let bare = json!({"type": "thinking", "id": v7(2).to_string(), "text": ""});
    let sealed = json!({"type": "thinking", "id": v7(2).to_string(), "text": "",
        "replay": {"format": "f", "payload": {"type": "text", "value": "s"}}});

    let refusal = ContentBlock::from_json(&bare.to_string()).unwrap_err();

    assert_eq!(refusal.code(), ErrorCode::InvalidContent);
    assert!(ContentBlock::from_json(&sealed.to_string()).is_ok());
}

#[test]
fn given_redacted_thinking_when_built_then_material_is_preserved_unchanged() {
    let material = signature("opaque+/==");
    let block = RedactedThinkingBlock::new(bid(1), material.clone());

    assert_eq!(block.material(), &material);
    assert_eq!(block.id(), bid(1));
}

#[test]
fn given_a_runtime_call_when_built_then_its_id_is_the_call_identity_and_externals_are_optional() {
    let bare = RuntimeToolCall::new(bid(1), tool("search"), common::oj(json!({"q": "x"}))).unwrap();
    let full = RuntimeToolCall::new(bid(2), tool("search"), common::oj(json!({})))
        .unwrap()
        .with_external_id(ExternalId::new("toolu_01").unwrap());

    assert_eq!(bare.id(), bid(1));
    assert_eq!(bare.external_id(), None);
    assert_eq!(full.external_id().map(|e| e.as_str()), Some("toolu_01"));
    assert_eq!(full.arguments().raw(), json!({}));
    assert_eq!(bare.name().as_str(), "search");
}

#[test]
fn given_empty_result_content_when_built_then_accepted_and_call_linked() {
    let result = result_with(1, vec![]).unwrap();

    assert!(result.content().is_empty());
    assert_eq!(result.call(), bid(99));
    assert_eq!(result.tool_name().as_str(), "search");
    assert!(!result.is_error());
}

#[test]
fn given_a_kind_list_when_inspected_then_ten_kinds_and_plain_ones_are_three() {
    assert_eq!(ContentBlockKind::ALL.len(), 10);
    assert_eq!(
        ContentBlockKind::ALL
            .iter()
            .filter(|k| k.is_plain())
            .count(),
        3
    );
}
