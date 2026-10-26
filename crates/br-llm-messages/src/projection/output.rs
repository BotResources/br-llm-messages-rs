use super::pending::Owner;
use super::perspective::blocks_of;
use crate::block::PlainBlock;
use crate::entry::{AgentOutput, IncomingEntry, ModelResponse, PartialAgentOutput};
use crate::error::Error;
use crate::id::EntryId;

pub(crate) fn output_entry(
    entry: EntryId,
    owner: &Owner,
    complete: bool,
    blocks: Vec<PlainBlock>,
) -> Result<Option<IncomingEntry>, Error> {
    if blocks.is_empty() {
        return Ok(None);
    }
    let (agent, author) = owner;
    let author = author.clone();
    if complete {
        AgentOutput::from_content(entry, *agent, author, blocks).map(IncomingEntry::AgentOutput)
    } else {
        PartialAgentOutput::from_content(entry, *agent, author, blocks)
            .map(IncomingEntry::PartialAgentOutput)
    }
    .map(Some)
}

pub(crate) fn output_of(owner: &Owner, response: &ModelResponse) -> Option<IncomingEntry> {
    output_entry(response.id(), owner, true, blocks_of(response.blocks()))
        .ok()
        .flatten()
}
