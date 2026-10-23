mod typed;

pub use typed::{
    AgentId, BlockId, CitationId, ConversationId, EntryId, HumanId, RequestId, TaskId, TurnId,
};

use uuid::{Uuid, Variant};

use crate::error::{Error, ErrorCode};

const CANONICAL_LEN: usize = 36;
const UUID_VERSION_7: usize = 7;

pub(crate) fn check(uuid: Uuid) -> Result<Uuid, Error> {
    if uuid.get_version_num() == UUID_VERSION_7 && uuid.get_variant() == Variant::RFC4122 {
        Ok(uuid)
    } else {
        Err(Error::new(ErrorCode::InvalidIdentity).with_id(uuid))
    }
}

pub(crate) fn parse(text: &str) -> Result<Uuid, Error> {
    let invalid = || Error::new(ErrorCode::InvalidIdentity);
    if text.len() != CANONICAL_LEN {
        return Err(invalid());
    }
    let uuid = Uuid::try_parse(text).map_err(|_| invalid())?;
    let mut buffer = Uuid::encode_buffer();
    if uuid.hyphenated().encode_lower(&mut buffer) != text {
        return Err(invalid());
    }
    check(uuid)
}
