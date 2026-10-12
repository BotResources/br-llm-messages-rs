use crate::block::UserBlock;
use crate::error::MessageError;
use crate::value::Author;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", try_from = "RawUserSource")]
pub enum UserSource {
    Human,
    Runtime { kind: String },
}

#[derive(serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum RawUserSource {
    Human,
    Runtime { kind: String },
}

impl TryFrom<RawUserSource> for UserSource {
    type Error = MessageError;

    fn try_from(raw: RawUserSource) -> Result<Self, Self::Error> {
        match raw {
            RawUserSource::Human => Ok(UserSource::Human),
            RawUserSource::Runtime { kind } => UserSource::runtime(kind),
        }
    }
}

impl UserSource {
    pub fn runtime(kind: impl Into<String>) -> Result<Self, MessageError> {
        let kind = kind.into();
        if kind.is_empty() {
            return Err(MessageError::Blank {
                field: "runtime_kind",
            });
        }
        Ok(UserSource::Runtime { kind })
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

impl std::fmt::Display for UserInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let source = match &self.source {
            UserSource::Human => "human".to_owned(),
            UserSource::Runtime { kind } => format!("runtime:{kind}"),
        };
        match &self.author {
            Some(author) => write!(f, "input[{source}] {author}"),
            None => write!(f, "input[{source}]"),
        }
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
            Some(Author::new("scheduler").unwrap()),
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
}
