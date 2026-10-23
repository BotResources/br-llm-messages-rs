mod common;

use br_llm_messages::{
    AgentId, BlockId, CitationId, ConversationId, EntryId, ErrorCode, HumanId, RequestId, TaskId,
    TurnId, Uuid,
};
use common::{code_of, refused, uuid_with, v7};
use uuid::{Variant, Version};

macro_rules! for_each_id {
    ($body:ident) => {
        $body!(ConversationId);
        $body!(TurnId);
        $body!(EntryId);
        $body!(BlockId);
        $body!(CitationId);
        $body!(HumanId);
        $body!(AgentId);
        $body!(RequestId);
        $body!(TaskId);
    };
}

#[test]
fn given_a_v7_rfc4122_uuid_when_wrapped_then_every_id_type_keeps_it() {
    macro_rules! check {
        ($id:ident) => {
            let wrapped = $id::from_uuid(v7(5)).unwrap();
            assert_eq!(wrapped.as_uuid(), v7(5));
            assert_eq!($id::try_from(v7(5)).unwrap(), wrapped);
        };
    }
    for_each_id!(check);
}

#[test]
fn given_nil_other_versions_or_other_variants_when_wrapped_then_invalid_identity() {
    let rejected = [
        Uuid::nil(),
        Uuid::max(),
        uuid_with(1, Version::Random, Variant::RFC4122),
        uuid_with(1, Version::Mac, Variant::RFC4122),
        uuid_with(1, Version::SortMac, Variant::RFC4122),
        uuid_with(1, Version::Sha1, Variant::RFC4122),
        uuid_with(1, Version::SortRand, Variant::NCS),
        uuid_with(1, Version::SortRand, Variant::Microsoft),
        uuid_with(1, Version::SortRand, Variant::Future),
    ];
    macro_rules! check {
        ($id:ident) => {
            for uuid in rejected {
                let error = refused($id::from_uuid(uuid));
                assert_eq!(error.code(), ErrorCode::InvalidIdentity);
                assert_eq!(
                    $id::try_from(uuid).unwrap_err().code(),
                    ErrorCode::InvalidIdentity
                );
            }
        };
    }
    for_each_id!(check);
}

#[test]
fn given_malformed_or_non_canonical_text_when_parsed_then_invalid_identity() {
    let canonical = v7(9).hyphenated().to_string();
    let rejected = [
        String::new(),
        "not-a-uuid".to_owned(),
        canonical.to_uppercase(),
        format!("{{{canonical}}}"),
        v7(9).urn().to_string(),
        v7(9).simple().to_string(),
        format!(" {canonical}"),
        format!("{canonical} "),
        canonical.replacen('-', "_", 1),
        canonical[..35].to_owned(),
    ];
    macro_rules! check {
        ($id:ident) => {
            for text in &rejected {
                assert_eq!(
                    refused($id::parse(text)).code(),
                    ErrorCode::InvalidIdentity,
                    "{text}"
                );
                assert_eq!(
                    refused(text.parse::<$id>()).code(),
                    ErrorCode::InvalidIdentity
                );
            }
            assert_eq!($id::parse(&canonical).unwrap().as_uuid(), v7(9));
        };
    }
    for_each_id!(check);
}

#[test]
fn given_a_text_holding_a_v4_uuid_when_parsed_then_invalid_identity() {
    let v4 = uuid_with(3, Version::Random, Variant::RFC4122)
        .hyphenated()
        .to_string();

    assert_eq!(
        refused(TurnId::parse(&v4)).code(),
        ErrorCode::InvalidIdentity
    );
}

#[test]
fn given_an_id_when_serialized_then_it_is_a_lowercase_hyphenated_string() {
    macro_rules! check {
        ($id:ident) => {
            let wrapped = $id::from_uuid(v7(0xAB)).unwrap();
            let json = serde_json::to_string(&wrapped).unwrap();
            assert_eq!(json, format!("\"{}\"", v7(0xAB).hyphenated()));
            assert_eq!(json, json.to_lowercase());
            assert_eq!(wrapped.to_string(), v7(0xAB).hyphenated().to_string());
            assert_eq!(serde_json::from_str::<$id>(&json).unwrap(), wrapped);
        };
    }
    for_each_id!(check);
}

#[test]
fn given_json_holding_an_invalid_id_when_loaded_then_invalid_identity() {
    let nil = format!("\"{}\"", Uuid::nil().hyphenated());
    let v4 = format!(
        "\"{}\"",
        uuid_with(3, Version::Random, Variant::RFC4122).hyphenated()
    );
    let upper = format!("\"{}\"", v7(4).hyphenated().to_string().to_uppercase());
    macro_rules! check {
        ($id:ident) => {
            for json in [&nil, &v4, &upper, &"\"\"".to_owned()] {
                assert_eq!(code_of::<$id>(json), ErrorCode::InvalidIdentity, "{json}");
            }
        };
    }
    for_each_id!(check);
}

#[test]
fn given_json_that_is_not_a_string_when_loaded_as_an_id_then_it_is_refused() {
    assert_eq!(code_of::<EntryId>("42"), ErrorCode::InvalidContent);
    assert_eq!(code_of::<EntryId>("null"), ErrorCode::InvalidContent);
    assert_eq!(code_of::<EntryId>("\"abc"), ErrorCode::MalformedJson);
}

#[test]
fn given_externally_owned_ids_when_wrapped_then_the_library_only_checks_them() {
    let foreign = v7(77);

    assert_eq!(HumanId::from_uuid(foreign).unwrap().as_uuid(), foreign);
    assert_eq!(AgentId::from_uuid(foreign).unwrap().as_uuid(), foreign);
    assert_eq!(RequestId::from_uuid(foreign).unwrap().as_uuid(), foreign);
    assert_eq!(TaskId::from_uuid(foreign).unwrap().as_uuid(), foreign);
    let v4 = uuid_with(1, Version::Random, Variant::RFC4122);
    assert!(AgentId::from_uuid(v4).is_err());
    assert!(HumanId::from_uuid(v4).is_err());
}

#[test]
fn given_ids_when_ordered_and_hashed_then_they_follow_the_uuid_value() {
    let low = TurnId::from_uuid(v7(1)).unwrap();
    let high = TurnId::from_uuid(v7(2)).unwrap();

    assert!(low < high);
    let mut set = std::collections::HashSet::new();
    set.insert(low);
    assert!(set.contains(&TurnId::from_uuid(v7(1)).unwrap()));
    assert!(!set.contains(&high));
}
