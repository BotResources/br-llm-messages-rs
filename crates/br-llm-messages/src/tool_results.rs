use crate::block::ToolResult;
use crate::error::MessageError;
use crate::value::ToolCallId;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(try_from = "RawToolResults")]
pub struct ToolResults {
    results: Vec<ToolResult>,
}

#[derive(serde::Deserialize)]
struct RawToolResults {
    results: Vec<ToolResult>,
}

impl TryFrom<RawToolResults> for ToolResults {
    type Error = MessageError;

    fn try_from(raw: RawToolResults) -> Result<Self, Self::Error> {
        let mut collected = ToolResults::empty();
        for result in raw.results {
            collected.push(result)?;
        }
        Ok(collected)
    }
}

impl ToolResults {
    pub fn empty() -> Self {
        Self {
            results: Vec::new(),
        }
    }

    pub fn push(&mut self, result: ToolResult) -> Result<(), MessageError> {
        if self.contains(&result.tool_call_id) {
            return Err(MessageError::DuplicateToolResultId {
                id: result.tool_call_id,
            });
        }
        self.results.push(result);
        Ok(())
    }

    pub fn contains(&self, id: &ToolCallId) -> bool {
        self.results.iter().any(|r| &r.tool_call_id == id)
    }

    pub fn ids(&self) -> impl Iterator<Item = &ToolCallId> {
        self.results.iter().map(|r| &r.tool_call_id)
    }

    pub fn results(&self) -> &[ToolResult] {
        &self.results
    }

    pub fn is_empty(&self) -> bool {
        self.results.is_empty()
    }

    pub fn len(&self) -> usize {
        self.results.len()
    }
}

impl Default for ToolResults {
    fn default() -> Self {
        Self::empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::ToolName;

    fn result(id: &str) -> ToolResult {
        ToolResult::new(
            ToolCallId::new(id).unwrap(),
            ToolName::new("search").unwrap(),
            Vec::new(),
            false,
        )
    }

    #[test]
    fn given_unique_results_when_pushed_then_collected_in_order() {
        let mut results = ToolResults::empty();
        results.push(result("a")).unwrap();
        results.push(result("b")).unwrap();
        let ids: Vec<&str> = results.ids().map(ToolCallId::as_str).collect();
        assert_eq!(ids, vec!["a", "b"]);
    }

    #[test]
    fn given_duplicate_id_when_pushed_then_refused_both_ways() {
        let mut results = ToolResults::empty();
        results.push(result("dup")).unwrap();
        assert!(matches!(
            results.push(result("dup")),
            Err(MessageError::DuplicateToolResultId { .. })
        ));
        let json = serde_json::json!({
            "results": [
                { "tool_call_id": "dup", "tool_name": "search", "content": [], "is_error": false },
                { "tool_call_id": "dup", "tool_name": "search", "content": [], "is_error": false }
            ]
        });
        assert!(serde_json::from_value::<ToolResults>(json).is_err());
    }

    #[test]
    fn given_results_when_round_tripped_then_identical() {
        let mut results = ToolResults::empty();
        results.push(result("a")).unwrap();
        let json = serde_json::to_value(&results).unwrap();
        assert_eq!(
            serde_json::from_value::<ToolResults>(json).unwrap(),
            results
        );
    }
}
