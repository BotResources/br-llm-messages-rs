use super::*;

fn draft() -> StepDraft {
    StepDraft::new()
}

#[test]
fn given_redacted_thinking_stream_without_data_when_finished_then_blank() {
    let mut draft = draft();
    draft
        .apply(StreamEvent::BlockStart {
            index: 0,
            kind: BlockKind::RedactedThinking,
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
        Err(MessageError::Blank {
            field: "redacted_thinking_data"
        })
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
fn given_invalid_tool_arguments_fragment_when_finish_then_invalid_json() {
    let mut draft = draft();
    draft
        .apply(StreamEvent::ToolCallStart {
            index: 0,
            id: ToolCallId::new("call_1").unwrap(),
            name: ToolName::new("search").unwrap(),
        })
        .unwrap();
    draft
        .apply(StreamEvent::ToolCallArgumentsDelta {
            index: 0,
            json_fragment: "{not json".to_owned(),
        })
        .unwrap();
    draft.apply(StreamEvent::BlockEnd { index: 0 }).unwrap();
    draft
        .apply(StreamEvent::Finish {
            stop_reason: StopReason::AwaitingToolResults,
            usage: None,
        })
        .unwrap();
    assert!(matches!(
        draft.finish(None),
        Err(MessageError::InvalidJson {
            field: "tool_arguments",
            ..
        })
    ));
}

#[test]
fn given_finish_with_no_blocks_when_finish_then_empty_step() {
    let mut draft = draft();
    draft
        .apply(StreamEvent::Finish {
            stop_reason: StopReason::EndTurn,
            usage: None,
        })
        .unwrap();
    assert!(matches!(draft.finish(None), Err(MessageError::EmptyStep)));
}

#[test]
fn given_empty_structured_block_when_finish_then_invalid_json() {
    let mut draft = draft();
    draft
        .apply(StreamEvent::BlockStart {
            index: 0,
            kind: BlockKind::Structured,
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
fn given_empty_text_block_when_finish_then_blank_text() {
    let mut draft = draft();
    draft
        .apply(StreamEvent::BlockStart {
            index: 0,
            kind: BlockKind::Text,
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
        Err(MessageError::Blank { field: "text" })
    ));
}

#[test]
fn given_tool_call_with_non_awaiting_finish_when_finish_then_tool_calls_without_awaiting() {
    let mut draft = draft();
    draft
        .apply(StreamEvent::ToolCallStart {
            index: 0,
            id: ToolCallId::new("call_1").unwrap(),
            name: ToolName::new("search").unwrap(),
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
        Err(MessageError::ToolCallsWithoutAwaiting)
    ));
}

#[test]
fn given_tool_call_before_text_block_when_finish_then_tool_call_not_at_tail() {
    let mut draft = draft();
    draft
        .apply(StreamEvent::ToolCallStart {
            index: 0,
            id: ToolCallId::new("call_1").unwrap(),
            name: ToolName::new("search").unwrap(),
        })
        .unwrap();
    draft.apply(StreamEvent::BlockEnd { index: 0 }).unwrap();
    draft
        .apply(StreamEvent::BlockStart {
            index: 1,
            kind: BlockKind::Text,
        })
        .unwrap();
    draft
        .apply(StreamEvent::TextDelta {
            index: 1,
            text: "trailing".to_owned(),
        })
        .unwrap();
    draft.apply(StreamEvent::BlockEnd { index: 1 }).unwrap();
    draft
        .apply(StreamEvent::Finish {
            stop_reason: StopReason::AwaitingToolResults,
            usage: None,
        })
        .unwrap();
    assert!(matches!(
        draft.finish(None),
        Err(MessageError::ToolCallNotAtTail)
    ));
}
