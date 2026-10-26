#![allow(
    dead_code,
    reason = "each integration test crate uses a different subset of these helpers"
)]

use br_llm_messages::{Error, ErrorCode, Uuid};
use uuid::{Builder, Variant, Version};

pub fn uuid_with(n: u64, version: Version, variant: Variant) -> Uuid {
    let mut bytes = [0u8; 16];
    bytes[8..].copy_from_slice(&n.to_be_bytes());
    bytes[0] = 0xab;
    Builder::from_bytes(bytes)
        .with_version(version)
        .with_variant(variant)
        .into_uuid()
}

pub fn v7(n: u64) -> Uuid {
    uuid_with(n, Version::SortRand, Variant::RFC4122)
}

pub fn at(n: u32) -> br_llm_messages::ObservedAt {
    let fraction = &"123456789"[..(n % 9 + 1) as usize];
    br_llm_messages::ObservedAt::new(format!("2026-10-23T08:00:{:02}.{fraction}Z", n % 60)).unwrap()
}

pub fn oj(value: serde_json::Value) -> br_llm_messages::OpaqueJson {
    br_llm_messages::OpaqueJson::from_serialize(&value).unwrap()
}

pub trait Raw {
    fn raw(&self) -> serde_json::Value;
}

impl Raw for br_llm_messages::OpaqueJson {
    fn raw(&self) -> serde_json::Value {
        self.deserialize_into().unwrap()
    }
}

pub fn decode<T: serde::de::DeserializeOwned>(json: &str) -> Result<T, Error> {
    br_llm_messages::OpaqueJson::parse(json)?.deserialize_into()
}

pub fn code_of<T: serde::de::DeserializeOwned>(json: &str) -> ErrorCode {
    refused(decode::<T>(json)).code()
}

pub fn refused<T, E: std::fmt::Debug>(result: Result<T, E>) -> E {
    match result {
        Ok(_) => panic!("must be refused"),
        Err(error) => error,
    }
}

pub fn bid(n: u64) -> br_llm_messages::BlockId {
    br_llm_messages::BlockId::from_uuid(v7(n)).unwrap()
}

pub fn cid(n: u64) -> br_llm_messages::CitationId {
    br_llm_messages::CitationId::from_uuid(v7(n)).unwrap()
}

pub fn signature(value: &str) -> br_llm_messages::ReplayMaterial {
    br_llm_messages::ReplayMaterial::new(
        br_llm_messages::ReplayFormat::new("anthropic.signature").unwrap(),
        br_llm_messages::ReplayPayload::text(value),
    )
}

pub fn envelope(value: serde_json::Value) -> br_llm_messages::ReplayMaterial {
    br_llm_messages::ReplayMaterial::new(
        br_llm_messages::ReplayFormat::new("openai.reasoning").unwrap(),
        br_llm_messages::ReplayPayload::json(oj(value)).unwrap(),
    )
}

pub fn text_block(n: u64, text: &str) -> br_llm_messages::TextBlock {
    br_llm_messages::TextBlock::new(bid(n), text).unwrap()
}

pub fn plain_text(n: u64, text: &str) -> br_llm_messages::PlainBlock {
    br_llm_messages::PlainBlock::try_from(text_block(n, text)).unwrap()
}

pub fn png(n: u64) -> br_llm_messages::ImageBlock {
    br_llm_messages::ImageBlock::new(
        bid(n),
        br_llm_messages::ImageSource::inline(
            br_llm_messages::MediaType::Png,
            br_llm_messages::Base64Data::new("QUJD").unwrap(),
        ),
    )
}

pub fn tool(name: &str) -> br_llm_messages::ToolName {
    br_llm_messages::ToolName::new(name).unwrap()
}

pub fn eid(n: u64) -> br_llm_messages::EntryId {
    br_llm_messages::EntryId::from_uuid(v7(n)).unwrap()
}

pub fn hid(n: u64) -> br_llm_messages::HumanId {
    br_llm_messages::HumanId::from_uuid(v7(n)).unwrap()
}

pub fn aid(n: u64) -> br_llm_messages::AgentId {
    br_llm_messages::AgentId::from_uuid(v7(n)).unwrap()
}

pub fn rid(n: u64) -> br_llm_messages::RequestId {
    br_llm_messages::RequestId::from_uuid(v7(n)).unwrap()
}

pub fn kid(n: u64) -> br_llm_messages::TaskId {
    br_llm_messages::TaskId::from_uuid(v7(n)).unwrap()
}

pub fn cvid(n: u64) -> br_llm_messages::ConversationId {
    br_llm_messages::ConversationId::from_uuid(v7(n)).unwrap()
}

pub fn author(label: &str) -> br_llm_messages::Author {
    br_llm_messages::Author::new(label).unwrap()
}

pub fn call(n: u64, name: &str) -> br_llm_messages::RuntimeToolCall {
    br_llm_messages::RuntimeToolCall::new(bid(n), tool(name), oj(serde_json::json!({}))).unwrap()
}

pub fn result(n: u64, call: u64, name: &str) -> br_llm_messages::RuntimeToolResult {
    br_llm_messages::RuntimeToolResult::new(bid(n), bid(call), tool(name), false, vec![]).unwrap()
}

pub mod conversations;
pub mod entries;
pub mod histories;
pub mod order;
pub mod records;
pub mod turns;

pub fn pending_of(
    drafts: &[br_llm_messages::ResponseDraft],
) -> Vec<br_llm_messages::PendingResponse<'_>> {
    drafts
        .iter()
        .map(br_llm_messages::PendingResponse::from)
        .collect()
}
