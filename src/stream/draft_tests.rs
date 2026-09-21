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
fn given_delta_with_no_open_block_when_applied_then_no_open_block() {
    let mut draft = draft();
    assert!(matches!(
        draft.apply(StreamEvent::TextDelta {
            index: 0,
            text: "hi".to_owned()
        }),
        Err(MessageError::NoOpenBlock { index: 0 })
    ));
}

#[test]
fn given_open_block_when_start_again_then_block_still_open() {
    let mut draft = draft();
    draft
        .apply(StreamEvent::BlockStart {
            index: 0,
            kind: BlockKind::Text,
        })
        .unwrap();
    assert!(matches!(
        draft.apply(StreamEvent::BlockStart {
            index: 1,
            kind: BlockKind::Text
        }),
        Err(MessageError::BlockStillOpen { index: 0 })
    ));
}

#[test]
fn given_wrong_kind_delta_when_applied_then_kind_mismatch() {
    let mut draft = draft();
    draft
        .apply(StreamEvent::BlockStart {
            index: 0,
            kind: BlockKind::Thinking,
        })
        .unwrap();
    assert!(matches!(
        draft.apply(StreamEvent::TextDelta {
            index: 0,
            text: "hi".to_owned()
        }),
        Err(MessageError::BlockKindMismatch { index: 0 })
    ));
}

#[test]
fn given_unexpected_index_when_start_then_refused() {
    let mut draft = draft();
    assert!(matches!(
        draft.apply(StreamEvent::BlockStart {
            index: 5,
            kind: BlockKind::Text
        }),
        Err(MessageError::UnexpectedBlockIndex {
            expected: 0,
            given: 5
        })
    ));
}

#[test]
fn given_block_start_tool_call_kind_when_applied_then_needs_start() {
    let mut draft = draft();
    assert!(matches!(
        draft.apply(StreamEvent::BlockStart {
            index: 0,
            kind: BlockKind::ToolCall
        }),
        Err(MessageError::ToolCallNeedsStart)
    ));
}

#[test]
fn given_open_block_when_finish_then_block_still_open() {
    let mut draft = draft();
    draft
        .apply(StreamEvent::BlockStart {
            index: 0,
            kind: BlockKind::Text,
        })
        .unwrap();
    assert!(matches!(
        draft.apply(StreamEvent::Finish {
            stop_reason: StopReason::EndTurn,
            usage: None
        }),
        Err(MessageError::BlockStillOpen { index: 0 })
    ));
}

#[test]
fn given_finished_draft_when_more_events_then_event_after_finish() {
    let mut draft = draft();
    draft
        .apply(StreamEvent::Finish {
            stop_reason: StopReason::EndTurn,
            usage: None,
        })
        .unwrap();
    assert!(matches!(
        draft.apply(StreamEvent::Finish {
            stop_reason: StopReason::EndTurn,
            usage: None
        }),
        Err(MessageError::EventAfterFinish)
    ));
}

#[test]
fn given_draft_without_finish_event_when_finish_then_missing_finish() {
    let draft = draft();
    assert!(matches!(
        draft.finish(None),
        Err(MessageError::MissingFinish)
    ));
}

#[test]
fn given_invalid_structured_fragment_when_finish_then_invalid_json() {
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
            json_fragment: "{not json".to_owned(),
        })
        .unwrap();
    draft.apply(StreamEvent::BlockEnd { index: 0 }).unwrap();
    draft
        .apply(StreamEvent::Finish {
            stop_reason: StopReason::EndTurn,
            usage: None,
        })
        .unwrap();
    assert!(matches!(
        draft.finish(None),
        Err(MessageError::InvalidJson {
            field: "structured",
            ..
        })
    ));
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
