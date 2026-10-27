use uuid::Uuid;

use crate::block::{ContentBlock, ContentBlockKind, PlainBlock};
use crate::entry::{IncomingEntry, ModelResponse, TurnEntry, TurnEntryKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Class {
    Conversation,
    Turn,
    Entry(TurnEntryKind),
    Output,
    Block(ContentBlockKind),
    Citation(Uuid),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Place {
    pub(crate) class: Class,
    pub(crate) entry: usize,
    pub(crate) root: usize,
    pub(crate) child: Option<usize>,
}

pub(crate) type Declared = Vec<(Uuid, Place)>;

pub(crate) enum Body<'a> {
    Content(&'a [PlainBlock]),
    Response(&'a [ContentBlock]),
    Results(&'a [crate::block::RuntimeToolResult]),
    Bare,
}

pub(crate) fn body_of_entry(entry: &TurnEntry) -> Body<'_> {
    match entry {
        TurnEntry::HumanMessage(entry) => Body::Content(entry.content()),
        TurnEntry::AgentRequest(entry) => Body::Content(entry.content()),
        TurnEntry::AgentMail(entry) => Body::Content(entry.content()),
        TurnEntry::ModelResponse(entry) => Body::Response(entry.blocks()),
        TurnEntry::RuntimeToolResults(entry) => Body::Results(entry.results()),
        TurnEntry::RequestAccepted(_)
        | TurnEntry::QueuePosition(_)
        | TurnEntry::TaskStarted(_)
        | TurnEntry::TaskCompleted(_)
        | TurnEntry::TaskFailed(_)
        | TurnEntry::RuntimeNotice(_)
        | TurnEntry::TurnClosure(_) => Body::Bare,
    }
}

pub(crate) fn body_of_incoming(entry: &IncomingEntry) -> Body<'_> {
    match entry {
        IncomingEntry::HumanMessage(entry) => Body::Content(entry.content()),
        IncomingEntry::AgentRequest(entry) => Body::Content(entry.content()),
        IncomingEntry::AgentMail(entry) => Body::Content(entry.content()),
        IncomingEntry::AgentOutput(entry) => Body::Content(entry.content()),
        IncomingEntry::PartialAgentOutput(entry) => Body::Content(entry.content()),
        IncomingEntry::RequestAccepted(_)
        | IncomingEntry::QueuePosition(_)
        | IncomingEntry::TaskStarted(_)
        | IncomingEntry::TaskCompleted(_)
        | IncomingEntry::TaskFailed(_)
        | IncomingEntry::RuntimeNotice(_) => Body::Bare,
    }
}

pub(crate) fn turn_place() -> Place {
    Place {
        class: Class::Turn,
        entry: 0,
        root: 0,
        child: None,
    }
}

pub(crate) fn entry_places(class: Class, id: Uuid, body: Body<'_>, entry: usize) -> Declared {
    let mut out = vec![(
        id,
        Place {
            class,
            entry,
            root: 0,
            child: None,
        },
    )];
    match body {
        Body::Content(blocks) => {
            for (root, block) in blocks.iter().enumerate() {
                plain_places(block, entry, root, None, &mut out);
            }
        }
        Body::Response(blocks) => {
            for (root, block) in blocks.iter().enumerate() {
                content_places(block, entry, root, &mut out);
            }
        }
        Body::Results(results) => {
            for (root, result) in results.iter().enumerate() {
                push_block(
                    result.id().as_uuid(),
                    ContentBlockKind::RuntimeToolResult,
                    entry,
                    root,
                    None,
                    &mut out,
                );
                for (child, block) in result.content().iter().enumerate() {
                    plain_places(block, entry, root, Some(child), &mut out);
                }
            }
        }
        Body::Bare => {}
    }
    out
}

pub(crate) fn places_of_turn_entry(entry: &TurnEntry, index: usize) -> Declared {
    entry_places(
        Class::Entry(entry.kind()),
        entry.id().as_uuid(),
        body_of_entry(entry),
        index,
    )
}

pub(crate) fn places_of_incoming(entry: &IncomingEntry, index: usize) -> Declared {
    entry_places(
        entry.kind().map_or(Class::Output, Class::Entry),
        entry.id().as_uuid(),
        body_of_incoming(entry),
        index,
    )
}

pub(crate) fn places_of_response(response: &ModelResponse) -> Declared {
    entry_places(
        Class::Entry(TurnEntryKind::ModelResponse),
        response.id().as_uuid(),
        Body::Response(response.blocks()),
        0,
    )
}

fn push_block(
    id: Uuid,
    kind: ContentBlockKind,
    entry: usize,
    root: usize,
    child: Option<usize>,
    out: &mut Declared,
) {
    out.push((
        id,
        Place {
            class: Class::Block(kind),
            entry,
            root,
            child,
        },
    ));
}

fn plain_places(
    block: &PlainBlock,
    entry: usize,
    root: usize,
    child: Option<usize>,
    out: &mut Declared,
) {
    push_block(block.id().as_uuid(), block.kind(), entry, root, child, out);
    if let PlainBlock::Text(text) = block {
        for citation in text.citations() {
            out.push((
                citation.id().as_uuid(),
                Place {
                    class: Class::Citation(block.id().as_uuid()),
                    entry,
                    root,
                    child,
                },
            ));
        }
    }
}

fn content_places(block: &ContentBlock, entry: usize, root: usize, out: &mut Declared) {
    push_block(block.id().as_uuid(), block.kind(), entry, root, None, out);
    if let ContentBlock::Text(text) = block {
        for citation in text.citations() {
            out.push((
                citation.id().as_uuid(),
                Place {
                    class: Class::Citation(block.id().as_uuid()),
                    entry,
                    root,
                    child: None,
                },
            ));
        }
    }
    for (child, nested) in block.children().iter().enumerate() {
        plain_places(nested, entry, root, Some(child), out);
    }
}
