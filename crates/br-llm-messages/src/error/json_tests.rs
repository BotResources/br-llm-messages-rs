use serde::de::Error as _;
use serde::{Deserialize, Deserializer};
use uuid::Uuid;

use super::{Error, ErrorCode};

#[derive(Debug)]
struct Probe;

impl<'de> Deserialize<'de> for Probe {
    fn deserialize<D: Deserializer<'de>>(_: D) -> Result<Self, D::Error> {
        let id = Uuid::parse_str("01890a5d-ac96-7ab2-80e2-4536629c90de").unwrap();
        let error = Error::new(ErrorCode::ReusedIdentity)
            .with_id(id)
            .at("entries[1]")
            .at("turns[2]");
        Err(D::Error::custom(error))
    }
}

#[derive(Debug)]
struct Bare;

impl<'de> Deserialize<'de> for Bare {
    fn deserialize<D: Deserializer<'de>>(_: D) -> Result<Self, D::Error> {
        Err(D::Error::custom(Error::new(ErrorCode::TurnClosed)))
    }
}

#[test]
fn given_custom_serde_error_with_id_and_location_when_mapped_then_all_three_survive() {
    let failure = serde_json::from_str::<Probe>("{\"a\": 1}").unwrap_err();

    let mapped = Error::from_json_error(&failure);

    assert_eq!(mapped.code(), ErrorCode::ReusedIdentity);
    assert_eq!(
        mapped.object_id(),
        Some(Uuid::parse_str("01890a5d-ac96-7ab2-80e2-4536629c90de").unwrap())
    );
    assert_eq!(mapped.location(), Some("turns[2].entries[1]"));
}

#[test]
fn given_custom_serde_error_with_code_only_when_mapped_then_no_id_or_location_appear() {
    let failure = serde_json::from_str::<Bare>("[]").unwrap_err();

    let mapped = Error::from_json_error(&failure);

    assert_eq!(mapped.code(), ErrorCode::TurnClosed);
    assert_eq!(mapped.object_id(), None);
    assert_eq!(mapped.location(), None);
}

#[test]
fn given_every_code_with_and_without_id_and_location_when_displayed_then_the_mapping_recovers_it() {
    let id = Uuid::parse_str("01890a5d-ac96-7ab2-80e2-4536629c90de").unwrap();
    for code in ErrorCode::ALL.iter().copied() {
        let variants = [
            Error::new(code),
            Error::new(code).with_id(id),
            Error::new(code).at("blocks[3].content[0]"),
            Error::new(code).with_id(id).at("blocks[3].content[0]"),
        ];
        for original in variants {
            let failure = <serde_json::Error as serde::de::Error>::custom(original.clone());

            assert_eq!(Error::from_json_error(&failure), original);
        }
    }
}
