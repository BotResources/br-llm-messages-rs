use super::*;
use crate::block::AssistantBlock;
use crate::value::Text;

/// A small stream builder: each helper opens, fills and closes one block at
/// the next index, `finish` closes the stream with its stop reason.
struct Stream {
    draft: StepDraft,
    next: usize,
}

impl Stream {
    fn new() -> Self {
        Self {
            draft: StepDraft::new(),
            next: 0,
        }
    }

    fn open(&mut self, kind: BlockKind) -> usize {
        let index = self.next;
        self.next += 1;
        self.draft
            .apply(StreamEvent::BlockStart { index, kind })
            .unwrap();
        index
    }

    fn close(&mut self, index: usize) {
        self.draft.apply(StreamEvent::BlockEnd { index }).unwrap();
    }

    fn text(mut self, deltas: &[&str]) -> Self {
        let index = self.open(BlockKind::Text);
        for delta in deltas {
            self.draft
                .apply(StreamEvent::TextDelta {
                    index,
                    text: (*delta).to_owned(),
                })
                .unwrap();
        }
        self.close(index);
        self
    }

    fn thinking(mut self, body: &str) -> Self {
        let index = self.open(BlockKind::Thinking);
        self.draft
            .apply(StreamEvent::ThinkingDelta {
                index,
                text: body.to_owned(),
            })
            .unwrap();
        self.close(index);
        self
    }

    fn structured(mut self, fragment: &str) -> Self {
        let index = self.open(BlockKind::Structured);
        self.draft
            .apply(StreamEvent::StructuredDelta {
                index,
                json_fragment: fragment.to_owned(),
            })
            .unwrap();
        self.close(index);
        self
    }

    fn tool_call(mut self, id: &str) -> Self {
        let index = self.next;
        self.next += 1;
        self.draft
            .apply(StreamEvent::ToolCallStart {
                index,
                id: ToolCallId::new(id).unwrap(),
                name: ToolName::new("search").unwrap(),
            })
            .unwrap();
        self.close(index);
        self
    }

    fn finish(mut self, stop_reason: StopReason) -> Result<Step, MessageError> {
        self.draft
            .apply(StreamEvent::Finish {
                stop_reason,
                usage: None,
            })
            .unwrap();
        self.draft.finish(None)
    }
}

fn text(body: &str) -> AssistantBlock {
    AssistantBlock::Text {
        text: Text::new(body).unwrap(),
    }
}

#[test]
fn given_only_an_empty_text_block_when_finish_then_empty_step() {
    let outcome = Stream::new().text(&[]).finish(StopReason::EndTurn);
    assert_eq!(outcome, Err(MessageError::EmptyStep));
}

#[test]
fn given_only_whitespace_text_blocks_when_finish_then_empty_step() {
    let outcome = Stream::new()
        .text(&["  ", "\n"])
        .text(&["\t"])
        .finish(StopReason::Refusal);
    assert_eq!(outcome, Err(MessageError::EmptyStep));
}

#[test]
fn given_blank_text_blocks_around_a_text_when_finish_then_only_the_text_kept() {
    let step = Stream::new()
        .text(&[" "])
        .text(&["  the ", "answer"])
        .text(&["\n\n"])
        .finish(StopReason::EndTurn)
        .unwrap();
    assert_eq!(step.content(), &[text("  the answer")]);
}

#[test]
fn given_blank_text_between_thinking_and_tool_call_when_finish_then_dropped() {
    let step = Stream::new()
        .thinking("plan")
        .text(&[" "])
        .tool_call("call_1")
        .finish(StopReason::AwaitingToolResults)
        .unwrap();
    assert_eq!(step.content().len(), 2);
    assert!(step.content()[0].as_thinking().is_some());
    assert!(step.content()[1].is_tool_call());
}

#[test]
fn given_blank_text_between_tool_calls_when_finish_then_dropped_and_calls_stay_the_tail() {
    let step = Stream::new()
        .tool_call("call_1")
        .text(&["\n"])
        .tool_call("call_2")
        .finish(StopReason::AwaitingToolResults)
        .unwrap();
    assert_eq!(step.tool_calls().count(), 2);
    assert_eq!(step.content().len(), 2);
}

#[test]
fn given_blank_text_after_a_tool_call_when_finish_then_dropped() {
    let step = Stream::new()
        .tool_call("call_1")
        .text(&["  "])
        .finish(StopReason::AwaitingToolResults)
        .unwrap();
    assert_eq!(step.content().len(), 1);
}

#[test]
fn given_only_blank_text_and_awaiting_stop_when_finish_then_empty_step() {
    let outcome = Stream::new()
        .text(&[" "])
        .finish(StopReason::AwaitingToolResults);
    assert_eq!(outcome, Err(MessageError::EmptyStep));
}

#[test]
fn given_blank_text_and_a_cut_structured_value_when_finish_then_invalid_json_not_empty_step() {
    let outcome = Stream::new()
        .text(&[" "])
        .structured("{\"cut\":")
        .finish(StopReason::MaxTokens);
    assert!(matches!(
        outcome,
        Err(MessageError::InvalidJson {
            field: "structured",
            ..
        })
    ));
}

#[test]
fn given_text_after_a_tool_call_when_finish_then_still_not_at_tail() {
    let outcome = Stream::new()
        .tool_call("call_1")
        .text(&["late"])
        .finish(StopReason::AwaitingToolResults);
    assert_eq!(outcome, Err(MessageError::ToolCallNotAtTail));
}
