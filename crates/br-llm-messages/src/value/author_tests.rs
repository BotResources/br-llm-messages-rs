use super::*;
use crate::error::MessageError;

#[test]
fn given_empty_id_when_new_then_blank_author_id() {
    assert!(matches!(
        Author::new("", "Alice"),
        Err(MessageError::Blank { field: "author_id" })
    ));
}

#[test]
fn given_empty_name_when_new_then_blank_author_name() {
    assert!(matches!(
        Author::new("a1", ""),
        Err(MessageError::Blank {
            field: "author_name"
        })
    ));
}

#[test]
fn given_single_space_id_and_name_when_new_then_accepted_verbatim() {
    let author = Author::new(" ", " ").unwrap();
    assert_eq!(author.id().as_str(), " ");
    assert_eq!(author.name().as_str(), " ");
}

#[test]
fn given_id_and_name_when_new_then_both_kept() {
    let author = Author::new("a1", "Alice").unwrap();
    assert_eq!(author.id().as_str(), "a1");
    assert_eq!(author.name().as_str(), "Alice");
}

#[test]
fn given_same_id_and_different_names_when_compared_then_same_author() {
    let before = Author::new("a1", "Alice").unwrap();
    let renamed = Author::new("a1", "Alice Martin").unwrap();
    assert_eq!(before, renamed);
}

#[test]
fn given_same_id_and_different_names_when_hashed_then_same_hash() {
    use std::collections::HashSet;
    let mut set = HashSet::new();
    set.insert(Author::new("a1", "Alice").unwrap());
    set.insert(Author::new("a1", "Alice Martin").unwrap());
    assert_eq!(set.len(), 1);
}

#[test]
fn given_same_name_and_different_ids_when_compared_then_different_authors() {
    let one = Author::new("a1", "Alice").unwrap();
    let other = Author::new("a2", "Alice").unwrap();
    assert_ne!(one, other);
}

#[test]
fn given_author_when_serialized_then_id_and_name_object() {
    let author = Author::new("a1", "Alice").unwrap();
    let json = serde_json::to_value(&author).unwrap();
    assert_eq!(json, serde_json::json!({ "id": "a1", "name": "Alice" }));
}

#[test]
fn given_author_when_round_tripped_then_id_and_name_kept() {
    let author = Author::new("a1", "Alice").unwrap();
    let json = serde_json::to_string(&author).unwrap();
    let loaded: Author = serde_json::from_str(&json).unwrap();
    assert_eq!(loaded.id().as_str(), "a1");
    assert_eq!(loaded.name().as_str(), "Alice");
}

#[test]
fn given_empty_id_or_name_in_json_when_deserialized_then_refused_with_rule_message() {
    let empty_id = serde_json::from_value::<Author>(serde_json::json!({ "id": "", "name": "A" }))
        .unwrap_err()
        .to_string();
    assert!(empty_id.contains("author_id must not be blank"));
    let empty_name = serde_json::from_value::<Author>(serde_json::json!({ "id": "a", "name": "" }))
        .unwrap_err()
        .to_string();
    assert!(empty_name.contains("author_name must not be blank"));
}

#[test]
fn given_missing_name_or_bare_string_when_deserialized_then_refused() {
    assert!(serde_json::from_value::<Author>(serde_json::json!({ "id": "a1" })).is_err());
    assert!(serde_json::from_value::<Author>(serde_json::json!("agent-a")).is_err());
}

#[test]
fn given_unknown_field_when_deserialized_then_accepted_and_dropped() {
    let author: Author =
        serde_json::from_value(serde_json::json!({ "id": "a1", "name": "A", "extra": 1 })).unwrap();
    assert_eq!(
        serde_json::to_value(&author).unwrap(),
        serde_json::json!({ "id": "a1", "name": "A" })
    );
}

#[test]
fn given_author_when_displayed_then_name_shown() {
    let author = Author::new("a1", "Alice").unwrap();
    assert_eq!(author.to_string(), "Alice");
}
