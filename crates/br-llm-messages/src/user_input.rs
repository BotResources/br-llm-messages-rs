use crate::block::UserBlock;
use crate::error::MessageError;
use crate::value::{Author, RuntimeKind};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum UserSource {
    Human,
    Runtime { kind: RuntimeKind },
}

impl UserSource {
    pub fn runtime(kind: impl Into<String>) -> Result<Self, MessageError> {
        Ok(UserSource::Runtime {
            kind: RuntimeKind::new(kind)?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(try_from = "RawUserInput")]
pub struct UserInput {
    source: UserSource,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    author: Option<Author>,
    content: Vec<UserBlock>,
}

#[derive(serde::Deserialize)]
struct RawUserInput {
    source: UserSource,
    #[serde(default)]
    author: Option<Author>,
    content: Vec<UserBlock>,
}

impl TryFrom<RawUserInput> for UserInput {
    type Error = MessageError;

    fn try_from(raw: RawUserInput) -> Result<Self, Self::Error> {
        UserInput::new(raw.source, raw.author, raw.content)
    }
}

impl UserInput {
    pub fn new(
        source: UserSource,
        author: Option<Author>,
        content: Vec<UserBlock>,
    ) -> Result<Self, MessageError> {
        if content.is_empty() {
            return Err(MessageError::EmptyUserContent);
        }
        Ok(Self {
            source,
            author,
            content,
        })
    }

    pub fn source(&self) -> &UserSource {
        &self.source
    }

    pub fn author(&self) -> Option<&Author> {
        self.author.as_ref()
    }

    pub fn content(&self) -> &[UserBlock] {
        &self.content
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::Text;

    fn text(body: &str) -> UserBlock {
        UserBlock::text(Text::new(body).unwrap())
    }

    #[test]
    fn given_empty_runtime_kind_when_new_then_blank_both_ways() {
        assert!(matches!(
            UserSource::runtime(""),
            Err(MessageError::Blank {
                field: "runtime_kind"
            })
        ));
        let json = serde_json::json!({ "type": "runtime", "kind": "" });
        assert!(serde_json::from_value::<UserSource>(json).is_err());
    }

    #[test]
    fn given_human_source_when_serialized_then_type_tagged() {
        let json = serde_json::to_value(UserSource::Human).unwrap();
        assert_eq!(json, serde_json::json!({ "type": "human" }));
    }

    #[test]
    fn given_empty_content_when_new_then_refused_both_ways() {
        assert!(matches!(
            UserInput::new(UserSource::Human, None, Vec::new()),
            Err(MessageError::EmptyUserContent)
        ));
        let json = serde_json::json!({ "source": { "type": "human" }, "content": [] });
        assert!(serde_json::from_value::<UserInput>(json).is_err());
    }

    #[test]
    fn given_input_with_author_when_round_tripped_then_identical() {
        let input = UserInput::new(
            UserSource::runtime("task notification").unwrap(),
            Some(Author::new("id-scheduler", "scheduler").unwrap()),
            vec![text("wake up")],
        )
        .unwrap();
        let json = serde_json::to_value(&input).unwrap();
        assert_eq!(serde_json::from_value::<UserInput>(json).unwrap(), input);
    }

    #[test]
    fn given_authorless_human_input_when_serialized_then_author_omitted() {
        let input = UserInput::new(UserSource::Human, None, vec![text("hi")]).unwrap();
        let json = serde_json::to_value(&input).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "source": { "type": "human" },
                "content": [ { "type": "text", "text": "hi" } ]
            })
        );
    }

    #[test]
    fn given_runtime_kind_value_when_source_built_by_hand_then_checked_kind_carried() {
        let source = UserSource::Runtime {
            kind: RuntimeKind::new("task notification").unwrap(),
        };
        assert_eq!(source, UserSource::runtime("task notification").unwrap());
        let json = serde_json::to_value(&source).unwrap();
        assert_eq!(
            json,
            serde_json::json!({ "type": "runtime", "kind": "task notification" })
        );
        assert_eq!(serde_json::from_value::<UserSource>(json).unwrap(), source);
    }

    #[test]
    fn given_one_space_runtime_kind_when_built_and_loaded_then_accepted() {
        assert!(UserSource::runtime(" ").is_ok());
        let json = serde_json::json!({ "type": "runtime", "kind": " " });
        assert!(serde_json::from_value::<UserSource>(json).is_ok());
    }

    #[test]
    fn given_user_input_with_empty_runtime_kind_when_loaded_then_refused_with_the_rule() {
        let json = serde_json::json!({
            "source": { "type": "runtime", "kind": "" },
            "content": [ { "type": "text", "text": "wake up" } ]
        });
        let error = serde_json::from_value::<UserInput>(json).unwrap_err();
        let rule = MessageError::Blank {
            field: "runtime_kind",
        };
        assert!(error.to_string().contains(&rule.to_string()), "{error}");
    }
}
