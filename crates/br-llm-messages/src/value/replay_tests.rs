use super::*;

fn format(name: &str) -> ReplayFormat {
    ReplayFormat::new(name).unwrap()
}

fn text_material(value: &str) -> ReplayMaterial {
    ReplayMaterial::new(format("anthropic.signature"), ReplayPayload::text(value))
}

#[test]
fn given_text_material_when_a_fragment_of_the_same_format_is_appended_then_it_is_concatenated() {
    let mut material = text_material("ab");

    material
        .append(&format("anthropic.signature"), "cd")
        .unwrap();

    assert_eq!(material, text_material("abcd"));
}

#[test]
fn given_text_material_when_an_empty_fragment_is_appended_then_nothing_changes() {
    let mut material = text_material("ab");

    material.append(&format("anthropic.signature"), "").unwrap();

    assert_eq!(material, text_material("ab"));
}

#[test]
fn given_text_material_when_another_format_is_appended_then_stream_target_error_at_format() {
    let mut material = text_material("ab");

    let error = material
        .append(&format("openai.reasoning"), "cd")
        .unwrap_err();

    assert_eq!(error.code(), ErrorCode::StreamTargetError);
    assert_eq!(error.location(), Some("format"));
    assert_eq!(material, text_material("ab"));
}

#[test]
fn given_json_material_when_a_fragment_is_appended_then_stream_target_error_at_payload() {
    let json = ReplayPayload::json(OpaqueJson::parse("{\"a\":1}").unwrap()).unwrap();
    let mut material = ReplayMaterial::new(format("anthropic.signature"), json);
    let before = material.clone();

    let error = material
        .append(&format("anthropic.signature"), "x")
        .unwrap_err();

    assert_eq!(error.code(), ErrorCode::StreamTargetError);
    assert_eq!(error.location(), Some("payload"));
    assert_eq!(material, before);
}
