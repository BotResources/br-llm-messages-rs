use crate::value::{Text, ToolCallId, ToolName};

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[non_exhaustive]
pub enum ToolResultBlock {
    Text { text: Text },
}

impl ToolResultBlock {
    pub fn text(text: Text) -> Self {
        ToolResultBlock::Text { text }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ToolResult {
    pub tool_call_id: ToolCallId,
    pub tool_name: ToolName,
    #[serde(default)]
    pub content: Vec<ToolResultBlock>,
    pub is_error: bool,
}

impl ToolResult {
    pub fn new(
        tool_call_id: ToolCallId,
        tool_name: ToolName,
        content: Vec<ToolResultBlock>,
        is_error: bool,
    ) -> Self {
        Self {
            tool_call_id,
            tool_name,
            content,
            is_error,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn given_result_block_when_serialized_then_type_tagged() {
        let block = ToolResultBlock::text(Text::new("42").unwrap());
        let json = serde_json::to_value(&block).unwrap();
        assert_eq!(json, serde_json::json!({ "type": "text", "text": "42" }));
        assert_eq!(
            serde_json::from_value::<ToolResultBlock>(json).unwrap(),
            block
        );
    }

    #[test]
    fn given_empty_content_when_built_then_accepted_and_round_trips() {
        let result = ToolResult::new(
            ToolCallId::new("call_1").unwrap(),
            ToolName::new("noop").unwrap(),
            Vec::new(),
            false,
        );
        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(serde_json::from_value::<ToolResult>(json).unwrap(), result);
    }

    #[test]
    fn given_error_result_when_round_tripped_then_identical() {
        let result = ToolResult::new(
            ToolCallId::new("call_2").unwrap(),
            ToolName::new("search").unwrap(),
            vec![ToolResultBlock::text(Text::new("boom").unwrap())],
            true,
        );
        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(serde_json::from_value::<ToolResult>(json).unwrap(), result);
    }
}
