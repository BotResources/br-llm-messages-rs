mod execution_record;
mod exposure;
mod ids;
mod image;
mod kind;
mod plain;
mod provider_call;
mod provider_result;
mod runtime_call;
mod runtime_result;
mod structured;
mod text;
mod thinking;

pub use execution_record::ProviderToolExecutionRecord;
pub use exposure::ResultExposure;
pub use image::ImageBlock;
pub use kind::ContentBlockKind;
pub use plain::PlainBlock;
pub use provider_call::ProviderToolCall;
pub use provider_result::ProviderToolResult;
pub use runtime_call::RuntimeToolCall;
pub use runtime_result::RuntimeToolResult;
pub use structured::StructuredBlock;
pub use text::TextBlock;
pub use thinking::{RedactedThinkingBlock, ThinkingBlock};

use crate::opaque_json::OpaqueJson;

use crate::error::Error;
use crate::id::BlockId;
use crate::value::ObservedAt;
pub(crate) use ids::IdSink;

tagged_enum! {
    #[non_exhaustive]
    #[derive(Debug, Clone, PartialEq)]
    pub enum ContentBlock, ContentBlockWire, remote = "ContentBlock", [tag = "type", rename_all = "snake_case"] {
        Text(TextBlock),
        Image(ImageBlock),
        StructuredContent(StructuredBlock),
        Thinking(ThinkingBlock),
        RedactedThinking(RedactedThinkingBlock),
        RuntimeToolCall(RuntimeToolCall),
        RuntimeToolResult(RuntimeToolResult),
        ProviderToolCall(ProviderToolCall),
        ProviderToolResult(ProviderToolResult),
        ProviderToolExecutionRecord(ProviderToolExecutionRecord),
    }
}

macro_rules! per_variant {
    ($($variant:ident($block:ty)),+ $(,)?) => {
        impl ContentBlock {
            pub fn kind(&self) -> ContentBlockKind {
                match self {
                    $(ContentBlock::$variant(_) => ContentBlockKind::$variant,)+
                }
            }

            pub fn id(&self) -> BlockId {
                match self {
                    $(ContentBlock::$variant(block) => block.id(),)+
                }
            }

            pub fn observed_at(&self) -> Option<&ObservedAt> {
                match self {
                    $(ContentBlock::$variant(block) => block.observed_at(),)+
                }
            }

            pub(crate) fn collect_ids(&self, sink: &mut IdSink) -> Result<(), Error> {
                match self {
                    $(ContentBlock::$variant(block) => block.collect_ids(sink),)+
                }
            }
        }

        $(
            impl From<$block> for ContentBlock {
                fn from(block: $block) -> Self {
                    ContentBlock::$variant(block)
                }
            }
        )+
    };
}

per_variant!(
    Text(TextBlock),
    Image(ImageBlock),
    StructuredContent(StructuredBlock),
    Thinking(ThinkingBlock),
    RedactedThinking(RedactedThinkingBlock),
    RuntimeToolCall(RuntimeToolCall),
    RuntimeToolResult(RuntimeToolResult),
    ProviderToolCall(ProviderToolCall),
    ProviderToolResult(ProviderToolResult),
    ProviderToolExecutionRecord(ProviderToolExecutionRecord),
);

impl From<PlainBlock> for ContentBlock {
    fn from(block: PlainBlock) -> Self {
        match block {
            PlainBlock::Text(text) => ContentBlock::Text(text),
            PlainBlock::Image(image) => ContentBlock::Image(image),
            PlainBlock::StructuredContent(structured) => {
                ContentBlock::StructuredContent(structured)
            }
        }
    }
}

impl ContentBlock {
    pub fn children(&self) -> &[PlainBlock] {
        match self {
            ContentBlock::RuntimeToolResult(block) => block.content(),
            ContentBlock::ProviderToolResult(block) => block.content(),
            ContentBlock::ProviderToolExecutionRecord(block) => block.content(),
            _ => &[],
        }
    }

    pub fn to_json(&self) -> Result<String, Error> {
        crate::json::encode(self)
    }

    pub fn from_json(text: &str) -> Result<Self, Error> {
        crate::json::decode(text)
    }

    pub fn to_value(&self) -> Result<OpaqueJson, Error> {
        crate::json::encode_value(self).map(OpaqueJson::from_inner)
    }

    pub fn from_value(value: OpaqueJson) -> Result<Self, Error> {
        let value = value.into_inner();
        crate::json::decode_value(value)
    }
}
