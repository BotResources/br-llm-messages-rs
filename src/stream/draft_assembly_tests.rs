use super::*;

fn draft() -> StepDraft {
    StepDraft::new()
}

#[test]
fn given_full_stream_when_folded_then_step_built_in_order() {
    let mut draft = draft();
    draft
        .apply(StreamEvent::BlockStart {
            index: 0,
            kind: BlockKind::Thinking,
        })
        .unwrap();
    draft
        .apply(StreamEvent::ThinkingDelta {
            index: 0,
            text: "let me ".to_owned(),
        })
        .unwrap();
    draft
        .apply(StreamEvent::ThinkingDelta {
            index: 0,
            text: "think".to_owned(),
        })
        .unwrap();
    draft
        .apply(StreamEvent::SignatureDelta {
            index: 0,
            signature: "sig==".to_owned(),
        })
        .unwrap();
    draft.apply(StreamEvent::BlockEnd { index: 0 }).unwrap();
    draft
        .apply(StreamEvent::ToolCallStart {
            index: 1,
            id: ToolCallId::new("call_1").unwrap(),
            name: ToolName::new("search").unwrap(),
        })
        .unwrap();
    draft
        .apply(StreamEvent::ToolCallArgumentsDelta {
            index: 1,
            json_fragment: "{\"q\":".to_owned(),
        })
        .unwrap();
    draft
        .apply(StreamEvent::ToolCallArgumentsDelta {
            index: 1,
            json_fragment: "\"rust\"}".to_owned(),
        })
        .unwrap();
    draft.apply(StreamEvent::BlockEnd { index: 1 }).unwrap();
    draft
        .apply(StreamEvent::Finish {
            stop_reason: StopReason::AwaitingToolResults,
            usage: None,
        })
        .unwrap();

    let step = draft
        .finish(Some(ModelId::new("claude-fable-5-1").unwrap()))
        .unwrap();
    assert_eq!(step.thinking().count(), 1);
    let call = step.tool_calls().next().unwrap();
    assert_eq!(call.arguments, serde_json::json!({ "q": "rust" }));
}

#[test]
fn given_redacted_thinking_stream_when_finished_then_data_preserved() {
    let mut draft = draft();
    draft
        .apply(StreamEvent::BlockStart {
            index: 0,
            kind: BlockKind::RedactedThinking,
        })
        .unwrap();
    draft
        .apply(StreamEvent::RedactedThinkingData {
            index: 0,
            data: "enc==".to_owned(),
        })
        .unwrap();
    draft.apply(StreamEvent::BlockEnd { index: 0 }).unwrap();
    draft
        .apply(StreamEvent::Finish {
            stop_reason: StopReason::EndTurn,
            usage: None,
        })
        .unwrap();
    let step = draft.finish(None).unwrap();
    assert!(matches!(
        step.content(),
        [AssistantBlock::RedactedThinking(redacted)] if redacted.data() == "enc=="
    ));
}

#[test]
fn given_valid_structured_stream_when_finish_then_value_parsed() {
    let mut draft = draft();
    draft
        .apply(StreamEvent::BlockStart {
            index: 0,
            kind: BlockKind::Structured,
        })
        .unwrap();
    draft
        .apply(StreamEvent::StructuredDelta {
            index: 0,
            json_fragment: "{\"a\":".to_owned(),
        })
        .unwrap();
    draft
        .apply(StreamEvent::StructuredDelta {
            index: 0,
            json_fragment: "1}".to_owned(),
        })
        .unwrap();
    draft.apply(StreamEvent::BlockEnd { index: 0 }).unwrap();
    draft
        .apply(StreamEvent::Finish {
            stop_reason: StopReason::EndTurn,
            usage: None,
        })
        .unwrap();
    let step = draft.finish(None).unwrap();
    assert_eq!(
        step.structured().next().unwrap(),
        &serde_json::json!({ "a": 1 })
    );
}

#[test]
fn given_tool_call_without_arguments_when_finish_then_empty_object() {
    let mut draft = draft();
    draft
        .apply(StreamEvent::ToolCallStart {
            index: 0,
            id: ToolCallId::new("call_1").unwrap(),
            name: ToolName::new("noop").unwrap(),
        })
        .unwrap();
    draft.apply(StreamEvent::BlockEnd { index: 0 }).unwrap();
    draft
        .apply(StreamEvent::Finish {
            stop_reason: StopReason::AwaitingToolResults,
            usage: None,
        })
        .unwrap();
    let step = draft.finish(None).unwrap();
    let call = step.tool_calls().next().unwrap();
    assert_eq!(call.arguments, serde_json::json!({}));
}

#[test]
fn given_folded_step_when_round_tripped_then_identical() {
    let mut draft = draft();
    draft
        .apply(StreamEvent::BlockStart {
            index: 0,
            kind: BlockKind::Text,
        })
        .unwrap();
    draft
        .apply(StreamEvent::TextDelta {
            index: 0,
            text: "hello".to_owned(),
        })
        .unwrap();
    draft.apply(StreamEvent::BlockEnd { index: 0 }).unwrap();
    draft
        .apply(StreamEvent::Finish {
            stop_reason: StopReason::EndTurn,
            usage: None,
        })
        .unwrap();
    let step = draft.finish(None).unwrap();
    let json = serde_json::to_value(&step).unwrap();
    assert_eq!(serde_json::from_value::<Step>(json).unwrap(), step);
}
