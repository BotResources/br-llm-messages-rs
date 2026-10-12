use super::*;

fn draft() -> StepDraft {
    StepDraft::new()
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
fn given_foreign_delta_on_open_block_when_applied_then_kind_mismatch() {
    let cases: Vec<(BlockKind, StreamEvent)> = vec![
        (
            BlockKind::Text,
            StreamEvent::SignatureDelta {
                index: 0,
                signature: "sig".to_owned(),
            },
        ),
        (
            BlockKind::Text,
            StreamEvent::ThinkingDelta {
                index: 0,
                text: "t".to_owned(),
            },
        ),
        (
            BlockKind::Text,
            StreamEvent::RedactedThinkingData {
                index: 0,
                data: "enc==".to_owned(),
            },
        ),
        (
            BlockKind::Structured,
            StreamEvent::ToolCallArgumentsDelta {
                index: 0,
                json_fragment: "{}".to_owned(),
            },
        ),
        (
            BlockKind::RedactedThinking,
            StreamEvent::TextDelta {
                index: 0,
                text: "x".to_owned(),
            },
        ),
    ];
    for (kind, delta) in cases {
        let mut draft = draft();
        draft
            .apply(StreamEvent::BlockStart { index: 0, kind })
            .unwrap();
        assert!(matches!(
            draft.apply(delta),
            Err(MessageError::BlockKindMismatch { index: 0 })
        ));
    }
}

#[test]
fn given_structured_delta_on_tool_call_block_when_applied_then_kind_mismatch() {
    let mut draft = draft();
    draft
        .apply(StreamEvent::ToolCallStart {
            index: 0,
            id: ToolCallId::new("call_1").unwrap(),
            name: ToolName::new("search").unwrap(),
        })
        .unwrap();
    assert!(matches!(
        draft.apply(StreamEvent::StructuredDelta {
            index: 0,
            json_fragment: "{}".to_owned()
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
fn given_block_end_with_no_open_block_when_applied_then_no_open_block() {
    let mut draft = draft();
    assert!(matches!(
        draft.apply(StreamEvent::BlockEnd { index: 0 }),
        Err(MessageError::NoOpenBlock { index: 0 })
    ));
}

#[test]
fn given_block_end_with_wrong_index_when_applied_then_no_open_block() {
    let mut draft = draft();
    draft
        .apply(StreamEvent::BlockStart {
            index: 0,
            kind: BlockKind::Text,
        })
        .unwrap();
    assert!(matches!(
        draft.apply(StreamEvent::BlockEnd { index: 1 }),
        Err(MessageError::NoOpenBlock { index: 1 })
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
