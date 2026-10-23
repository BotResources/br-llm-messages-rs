mod common;

use br_llm_messages::{
    Base64Data, ErrorCode, ImageSource, MediaType, ReplayFormat, ReplayMaterial, ReplayPayload,
};
use common::{code_of, refused};
use serde_json::json;

#[test]
fn given_the_four_media_types_when_round_tripped_then_strings_are_stable() {
    for (media, text) in [
        (MediaType::Jpeg, "image/jpeg"),
        (MediaType::Png, "image/png"),
        (MediaType::Gif, "image/gif"),
        (MediaType::Webp, "image/webp"),
    ] {
        assert_eq!(media.as_str(), text);
        assert_eq!(text.parse::<MediaType>().unwrap(), media);
        assert_eq!(
            serde_json::to_string(&media).unwrap(),
            format!("\"{text}\"")
        );
        assert_eq!(
            serde_json::from_str::<MediaType>(&format!("\"{text}\"")).unwrap(),
            media
        );
    }
}

#[test]
fn given_other_media_types_when_parsed_then_variant_mismatch() {
    for other in [
        "image/bmp",
        "image/svg+xml",
        "IMAGE/PNG",
        "image/png ",
        "",
        "application/pdf",
    ] {
        assert_eq!(
            refused(other.parse::<MediaType>()).code(),
            ErrorCode::VariantMismatch
        );
    }
    assert_eq!(
        code_of::<MediaType>("\"image/tiff\""),
        ErrorCode::VariantMismatch
    );
}

#[test]
fn given_standard_padded_base64_when_built_then_accepted_verbatim() {
    for valid in [
        "AAAA",
        "QUJD",
        "QQ==",
        "QUI=",
        "+/+/",
        "AAAAAAAA",
        "iVBORw0KGgo=",
    ] {
        assert_eq!(Base64Data::new(valid).unwrap().as_str(), valid);
    }
}

#[test]
fn given_non_standard_base64_when_built_then_invalid_content() {
    for invalid in [
        "", "AAA", "AAAAA", "AA", "A===", "====", "AA=A", "=AAA", "A-AA", "A_AA", "AA AA", "AAA\n",
        "AAAA\n", "QQ=\n", "AAé=", "AAAA====", "QQ==QQ==",
    ] {
        assert_eq!(
            refused(Base64Data::new(invalid)).code(),
            ErrorCode::InvalidContent,
            "{invalid:?}"
        );
    }
}

#[test]
fn given_missing_padding_or_non_zero_pad_bits_when_built_then_only_the_shape_decides() {
    assert!(Base64Data::new("QQ=").is_err());
    assert!(Base64Data::new("QR==").is_ok());
}

#[test]
fn given_an_inline_image_source_when_round_tripped_then_golden_json_matches() {
    let source = ImageSource::inline(MediaType::Png, Base64Data::new("AAAA").unwrap());

    let value = serde_json::to_value(&source).unwrap();

    assert_eq!(
        value,
        json!({"type":"inline","media_type":"image/png","data":"AAAA"})
    );
    assert_eq!(
        serde_json::from_value::<ImageSource>(value).unwrap(),
        source
    );
}

#[test]
fn given_a_malformed_image_source_when_loaded_then_the_precise_code_surfaces() {
    assert_eq!(
        code_of::<ImageSource>(r#"{"type":"inline","media_type":"image/png","data":"A A="}"#),
        ErrorCode::InvalidContent
    );
    assert_eq!(
        code_of::<ImageSource>(r#"{"type":"inline","media_type":"image/bmp","data":"AAAA"}"#),
        ErrorCode::VariantMismatch
    );
    assert_eq!(
        code_of::<ImageSource>(r#"{"type":"url","media_type":"image/png","data":"AAAA"}"#),
        ErrorCode::VariantMismatch
    );
    assert_eq!(
        code_of::<ImageSource>(r#"{"type":"inline","media_type":"image/png"}"#),
        ErrorCode::InvalidContent
    );
    assert_eq!(
        code_of::<ImageSource>(
            r#"{"type":"inline","media_type":"image/png","data":"AAAA","extra":1}"#
        ),
        ErrorCode::InvalidContent
    );
}

#[test]
fn given_replay_format_when_checked_then_any_non_empty_string_up_to_256_scalars_passes_unchanged() {
    let longest = "é".repeat(256);
    for valid in [
        "a",
        "Anthropic Signature/v2",
        "OpenAI:reasoning#1",
        " spaced ",
        "1a",
        "\u{1F600}\u{0301}",
        longest.as_str(),
    ] {
        let format = ReplayFormat::new(valid).unwrap();

        assert_eq!(format.as_str(), valid);
        assert_eq!(format, ReplayFormat::new(valid).unwrap());
    }
    assert_ne!(
        ReplayFormat::new("a").unwrap(),
        ReplayFormat::new("A").unwrap()
    );
}

#[test]
fn given_replay_format_when_empty_or_longer_than_256_scalars_then_invalid_content() {
    let too_long = "é".repeat(257);
    for invalid in ["", too_long.as_str()] {
        assert_eq!(
            refused(ReplayFormat::new(invalid)).code(),
            ErrorCode::InvalidContent
        );
    }
    assert_eq!(code_of::<ReplayFormat>("\"\""), ErrorCode::InvalidContent);
    assert_eq!(
        code_of::<ReplayFormat>(&format!("\"{too_long}\"")),
        ErrorCode::InvalidContent
    );
    assert_eq!(
        serde_json::from_str::<ReplayFormat>("\"Bad Format\"")
            .unwrap()
            .as_str(),
        "Bad Format"
    );
}

#[test]
fn given_text_and_json_payloads_when_round_tripped_then_preserved_exactly() {
    let format = ReplayFormat::new("anthropic.signature").unwrap();
    let text = ReplayMaterial::new(format.clone(), ReplayPayload::text(" sig\u{0301}\n=+/ "));
    let object = json!({"b":[1,2.5,null,{"z":"y"}],"a":"é","empty":"","neg":-3});
    let structured = ReplayMaterial::new(
        ReplayFormat::new("openai.reasoning").unwrap(),
        ReplayPayload::json(common::oj(object.clone())).unwrap(),
    );

    for material in [text, structured] {
        let wire = serde_json::to_string(&material).unwrap();
        assert_eq!(
            serde_json::from_str::<ReplayMaterial>(&wire).unwrap(),
            material
        );
    }
    let wire = serde_json::to_value(ReplayMaterial::new(format, ReplayPayload::text("s"))).unwrap();
    assert_eq!(
        wire,
        json!({"format":"anthropic.signature","payload":{"type":"text","value":"s"}})
    );
}

#[test]
fn given_replay_material_the_format_is_never_inferred_from_the_payload() {
    let material = ReplayMaterial::new(
        ReplayFormat::new("anthropic.signature").unwrap(),
        ReplayPayload::json(common::oj(
            json!({"type":"reasoning","encrypted_content":"x"}),
        ))
        .unwrap(),
    );

    assert_eq!(material.format().as_str(), "anthropic.signature");
    assert!(matches!(material.payload(), ReplayPayload::Json { .. }));
}

#[test]
fn given_malformed_replay_material_when_loaded_then_refused_with_the_precise_code() {
    assert_eq!(
        code_of::<ReplayMaterial>(r#"{"format":"a","payload":{"type":"text","value":"s"},"x":1}"#),
        ErrorCode::InvalidContent
    );
    assert_eq!(
        code_of::<ReplayMaterial>(r#"{"format":"a","payload":{"type":"text","value":"s","x":1}}"#),
        ErrorCode::InvalidContent
    );
    assert_eq!(
        code_of::<ReplayMaterial>(r#"{"format":"a","payload":{"type":"text"}}"#),
        ErrorCode::InvalidContent
    );
    assert_eq!(
        code_of::<ReplayMaterial>(r#"{"format":"a","payload":{"type":"binary","value":"s"}}"#),
        ErrorCode::VariantMismatch
    );
    assert_eq!(
        code_of::<ReplayMaterial>(r#"{"payload":{"type":"text","value":"s"}}"#),
        ErrorCode::InvalidContent
    );
    assert_eq!(
        code_of::<ReplayMaterial>(r#"{"format":"","payload":{"type":"text","value":"s"}}"#),
        ErrorCode::InvalidContent
    );
    assert_eq!(
        code_of::<ReplayMaterial>(r#"{"format":"a","payload":{"type":"text","value":5}}"#),
        ErrorCode::InvalidContent
    );
}
