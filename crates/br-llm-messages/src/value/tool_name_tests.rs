use super::*;

const REFUSED: &[&str] = &[
    "",
    " ",
    "1search",
    "-search",
    "web search",
    "mcp.search",
    "files/read",
    "search!",
    "recherche_é",
    "検索",
    "search\n",
];

fn longest() -> String {
    format!("a{}", "b".repeat(MAX_LEN - 1))
}

#[test]
fn given_names_every_provider_accepts_when_new_then_kept_as_given() {
    for name in [
        "a",
        "_",
        "_x",
        "search",
        "Get-Weather_2",
        "Z9-_",
        &longest(),
    ] {
        let tool_name = ToolName::new(name).unwrap();
        assert_eq!(tool_name.as_str(), name);
    }
}

#[test]
fn given_names_a_provider_refuses_when_new_then_invalid_tool_name() {
    for name in REFUSED {
        assert_eq!(
            ToolName::new(*name),
            Err(MessageError::InvalidToolName),
            "{name:?}"
        );
    }
}

#[test]
fn given_name_of_65_characters_when_new_then_invalid_tool_name() {
    let name = format!("{}c", longest());
    assert_eq!(name.len(), MAX_LEN + 1);
    assert_eq!(ToolName::new(name), Err(MessageError::InvalidToolName));
}

#[test]
fn given_names_a_provider_refuses_when_loaded_then_refused_with_the_rule() {
    for name in REFUSED
        .iter()
        .copied()
        .chain([format!("{}c", longest()).as_str()])
    {
        let json = serde_json::to_string(name).unwrap();
        let error = serde_json::from_str::<ToolName>(&json).unwrap_err();
        assert!(
            error
                .to_string()
                .contains(&MessageError::InvalidToolName.to_string()),
            "{name:?}: {error}"
        );
    }
}

#[test]
fn given_valid_name_when_round_tripped_then_identical() {
    let name = ToolName::new("get_weather-v2").unwrap();
    let json = serde_json::to_string(&name).unwrap();
    assert_eq!(json, "\"get_weather-v2\"");
    assert_eq!(serde_json::from_str::<ToolName>(&json).unwrap(), name);
}
