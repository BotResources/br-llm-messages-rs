mod common;
mod stream_support;

use br_llm_messages::{ContentBlock, ErrorCode, ToolName};
use common::{bid, call, code_of, refused};
use serde_json::json;
use stream_support::{finalize, json, ok, run, start_call};

#[test]
fn given_names_around_the_limit_when_built_then_256_passes_and_257_is_invalid_content() {
    let longest = "a".repeat(256);
    let too_long = "a".repeat(257);

    assert_eq!(ToolName::new(longest.clone()).unwrap().as_str(), longest);
    assert_eq!(
        refused(ToolName::new(too_long)).code(),
        ErrorCode::InvalidContent
    );
}

#[test]
fn given_a_long_name_when_the_charset_or_first_character_is_wrong_then_it_stays_invalid() {
    for invalid in [
        format!("1{}", "a".repeat(100)),
        format!("-{}", "a".repeat(100)),
        format!("{}é", "a".repeat(100)),
        format!("{} b", "a".repeat(100)),
        format!("{}.b", "a".repeat(200)),
    ] {
        assert_eq!(
            refused(ToolName::new(invalid)).code(),
            ErrorCode::InvalidContent
        );
    }
}

#[test]
fn given_a_json_runtime_call_when_the_name_is_256_then_it_loads_and_257_is_invalid_content() {
    let block = |name: String| {
        let mut block = serde_json::to_value(ContentBlock::from(call(11, "a"))).unwrap();
        block["name"] = json!(name);
        block.to_string()
    };

    let ContentBlock::RuntimeToolCall(loaded) =
        ContentBlock::from_json(&block("n".repeat(256))).unwrap()
    else {
        panic!("expected a runtime call");
    };
    assert_eq!(loaded.name().as_str().len(), 256);
    assert_eq!(
        code_of::<ContentBlock>(&block("n".repeat(257))),
        ErrorCode::InvalidContent
    );
}

#[test]
fn given_a_draft_when_a_call_starts_with_a_256_name_then_the_finished_call_carries_it() {
    let name = "n".repeat(256);
    let mut draft = run(vec![start_call(11, Some(&name), None)]);
    ok(&mut draft, json(11, "{}"));
    ok(&mut draft, finalize(11));
    ok(
        &mut draft,
        stream_support::complete(br_llm_messages::ResponseEnding::tool_use()),
    );

    let response = draft.into_response().unwrap();

    let ContentBlock::RuntimeToolCall(finished) = &response.blocks()[0] else {
        panic!("expected a runtime call");
    };
    assert_eq!(finished.name().as_str(), name);
    assert_eq!(finished.id(), bid(11));
}
