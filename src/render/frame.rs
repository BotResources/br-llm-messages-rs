use crate::block::{AssistantBlock, UserBlock};
use crate::error::MessageError;
use crate::turn::{Turn, TurnItem};
use crate::user_input::{UserInput, UserSource};
use crate::value::Text;

use super::wire::{WireMessage, WireUserBlock};

pub(super) fn render_user_frame(input: &UserInput) -> Result<WireMessage, MessageError> {
    let (role, kind) = match input.source() {
        UserSource::Human => ("user", None),
        UserSource::Runtime { kind } => ("runtime", Some(kind.as_str())),
    };
    let author = input.author().map(|author| author.as_str());

    let mut texts = Vec::new();
    let mut images = Vec::new();
    for block in input.content() {
        match block {
            UserBlock::Text { text } => texts.push(text.as_str()),
            UserBlock::Image(image) => images.push(image.clone()),
        }
    }

    let framed = frame(author, role, kind, &texts.join("\n\n"));
    let mut content = vec![WireUserBlock::Text {
        text: Text::new(framed)?,
    }];
    for image in images {
        content.push(WireUserBlock::Image(image));
    }
    Ok(WireMessage::User { content })
}

pub(super) fn render_agent_frame(turn: &Turn) -> Result<WireMessage, MessageError> {
    let author = turn.author().map(|author| author.as_str());
    let mut parts = Vec::new();
    for item in turn.items() {
        if let TurnItem::Step(step) = item {
            for block in step.content() {
                match block {
                    AssistantBlock::Text { text } => parts.push(text.as_str().to_owned()),
                    AssistantBlock::Structured { value } => parts.push(value.to_string()),
                    AssistantBlock::Thinking(_)
                    | AssistantBlock::RedactedThinking(_)
                    | AssistantBlock::ToolCall(_) => {}
                }
            }
        }
    }
    let framed = frame(author, "agent", None, &parts.join("\n\n"));
    Ok(WireMessage::User {
        content: vec![WireUserBlock::Text {
            text: Text::new(framed)?,
        }],
    })
}

fn frame(author: Option<&str>, role: &str, kind: Option<&str>, body: &str) -> String {
    let mut opening = String::from("<message");
    if let Some(author) = author {
        opening.push_str(&format!(" author=\"{}\"", escape_attr(author)));
    }
    opening.push_str(&format!(" role=\"{role}\""));
    if let Some(kind) = kind {
        opening.push_str(&format!(" kind=\"{}\"", escape_attr(kind)));
    }
    opening.push('>');
    format!("{opening}{}</message>", escape_text(body))
}

fn escape_text(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn escape_attr(input: &str) -> String {
    escape_text(input).replace('"', "&quot;")
}
