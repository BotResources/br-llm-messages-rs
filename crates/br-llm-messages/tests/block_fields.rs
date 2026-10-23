mod common;
use common::Raw as _;

use br_llm_messages::{
    ContentBlock, ErrorCode, ItemRef, ProviderToolCall, RuntimeToolCall, TextBlock,
};
use common::{bid, refused, tool, v7};
use serde_json::json;

#[test]
fn given_non_object_arguments_when_a_runtime_call_is_built_then_invalid_content() {
    for arguments in [json!(null), json!([1]), json!("x"), json!(3), json!(true)] {
        let error = refused(RuntimeToolCall::new(
            bid(1),
            tool("search"),
            common::oj(arguments),
        ));

        assert_eq!(error.code(), ErrorCode::InvalidContent);
        assert_eq!(error.object_id(), Some(v7(1)));
        assert_eq!(error.location(), Some("arguments"));
    }
}

#[test]
fn given_non_object_arguments_in_json_when_a_runtime_call_is_loaded_then_invalid_content() {
    for arguments in [json!(null), json!([1]), json!("x")] {
        let wire = json!({"type": "runtime_tool_call", "id": v7(1).to_string(), "name": "t", "arguments": arguments});

        let error = ContentBlock::from_json(&wire.to_string()).unwrap_err();

        assert_eq!(error.code(), ErrorCode::InvalidContent);
    }
}

#[test]
fn given_non_object_arguments_when_a_provider_call_is_built_then_they_stay_unrestricted() {
    for arguments in [json!(null), json!([1]), json!("x")] {
        let call = ProviderToolCall::new(bid(1))
            .with_arguments(common::oj(arguments.clone()))
            .unwrap();

        assert_eq!(call.arguments().map(|value| value.raw()), Some(arguments));
    }
}

#[test]
fn given_an_item_reference_when_set_on_text_call_and_provider_call_then_it_is_kept_and_round_trips()
{
    let item = || ItemRef::new("msg_01").unwrap();
    let text = ContentBlock::from(TextBlock::new(bid(1), "x").unwrap().with_item_ref(item()));
    let call = ContentBlock::from(
        RuntimeToolCall::new(bid(2), tool("t"), common::oj(json!({})))
            .unwrap()
            .with_item_ref(item()),
    );
    let provider = ContentBlock::from(ProviderToolCall::new(bid(3)).with_item_ref(item()));

    for block in [text, call, provider] {
        let again = ContentBlock::from_json(&block.to_json().unwrap()).unwrap();

        assert_eq!(again, block);
        assert!(block.to_json().unwrap().contains("\"item_ref\":\"msg_01\""));
    }
}

#[test]
fn given_an_empty_item_reference_when_built_or_loaded_then_invalid_content() {
    let wire = json!({"type": "text", "id": v7(1).to_string(), "text": "x", "item_ref": ""});

    assert_eq!(refused(ItemRef::new("")).code(), ErrorCode::InvalidContent);
    assert_eq!(
        ContentBlock::from_json(&wire.to_string())
            .unwrap_err()
            .code(),
        ErrorCode::InvalidContent
    );
}

#[test]
fn given_an_item_reference_on_the_blocks_that_have_none_when_loaded_then_invalid_content() {
    let image = json!({"type": "image", "id": v7(1).to_string(), "item_ref": "x",
        "source": {"type": "inline", "media_type": "image/png", "data": "QUJD"}});
    let structured =
        json!({"type": "structured_content", "id": v7(1).to_string(), "item_ref": "x", "value": 1});
    let thinking =
        json!({"type": "thinking", "id": v7(1).to_string(), "text": "t", "item_ref": "x"});

    for wire in [image, structured, thinking] {
        assert_eq!(
            ContentBlock::from_json(&wire.to_string())
                .unwrap_err()
                .code(),
            ErrorCode::InvalidContent
        );
    }
}
