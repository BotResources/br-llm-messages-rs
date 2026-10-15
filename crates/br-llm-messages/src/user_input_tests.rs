use super::*;
use crate::value::Text;

fn id(value: &str) -> InputId {
    InputId::new(value).unwrap()
}

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
        UserInput::new(id("i1"), UserSource::Human, None, Vec::new()),
        Err(MessageError::EmptyUserContent)
    ));
    let json = serde_json::json!({ "id": "i1", "source": { "type": "human" }, "content": [] });
    assert!(serde_json::from_value::<UserInput>(json).is_err());
}

#[test]
fn given_input_with_author_when_round_tripped_then_identical() {
    let input = UserInput::new(
        id("i1"),
        UserSource::runtime("task notification").unwrap(),
        Some(Author::new("id-scheduler", "scheduler").unwrap()),
        vec![text("wake up")],
    )
    .unwrap();
    let json = serde_json::to_value(&input).unwrap();
    assert_eq!(serde_json::from_value::<UserInput>(json).unwrap(), input);
}

#[test]
fn given_authorless_human_input_when_serialized_then_id_first_and_author_omitted() {
    let input = UserInput::new(id("i1"), UserSource::Human, None, vec![text("hi")]).unwrap();
    let json = serde_json::to_string(&input).unwrap();
    assert_eq!(
        json,
        r#"{"id":"i1","source":{"type":"human"},"content":[{"type":"text","text":"hi"}]}"#
    );
}

#[test]
fn given_id_chosen_by_the_caller_when_built_then_kept_verbatim_both_ways() {
    let input = UserInput::new(id(" in 1 "), UserSource::Human, None, vec![text("hi")]).unwrap();
    assert_eq!(input.id(), &id(" in 1 "));
    let json = serde_json::to_value(&input).unwrap();
    assert_eq!(json["id"], serde_json::json!(" in 1 "));
    let loaded = serde_json::from_value::<UserInput>(json).unwrap();
    assert_eq!(loaded.id().as_str(), " in 1 ");
}

#[test]
fn given_one_space_input_id_when_built_and_loaded_then_accepted() {
    assert!(UserInput::new(id(" "), UserSource::Human, None, vec![text("hi")]).is_ok());
    let json = serde_json::json!({
        "id": " ",
        "source": { "type": "human" },
        "content": [ { "type": "text", "text": "hi" } ]
    });
    assert!(serde_json::from_value::<UserInput>(json).is_ok());
}

#[test]
fn given_user_input_with_empty_id_when_loaded_then_refused_with_the_rule() {
    assert_eq!(
        InputId::new(""),
        Err(MessageError::Blank { field: "input_id" })
    );
    let json = serde_json::json!({
        "id": "",
        "source": { "type": "human" },
        "content": [ { "type": "text", "text": "hi" } ]
    });
    let error = serde_json::from_value::<UserInput>(json).unwrap_err();
    let rule = MessageError::Blank { field: "input_id" };
    assert!(error.to_string().contains(&rule.to_string()), "{error}");
}

#[test]
fn given_user_input_without_id_when_loaded_then_refused() {
    let json = serde_json::json!({
        "source": { "type": "human" },
        "content": [ { "type": "text", "text": "hi" } ]
    });
    let error = serde_json::from_value::<UserInput>(json).unwrap_err();
    assert!(error.to_string().contains("missing field `id`"), "{error}");
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
        "id": "i1",
        "source": { "type": "runtime", "kind": "" },
        "content": [ { "type": "text", "text": "wake up" } ]
    });
    let error = serde_json::from_value::<UserInput>(json).unwrap_err();
    let rule = MessageError::Blank {
        field: "runtime_kind",
    };
    assert!(error.to_string().contains(&rule.to_string()), "{error}");
}
