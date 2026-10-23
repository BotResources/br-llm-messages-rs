use std::collections::HashMap;

use super::check_containment;
use crate::block::ContentBlockKind;
use crate::error::ErrorCode;
use crate::id::{BlockId, EntryId};
use crate::opaque_json::OpaqueJson;
use crate::record::{BlockRecord, RepresentationState};
use uuid::Uuid;

fn block_id(tail: u8) -> BlockId {
    let mut bytes = [0u8; 16];
    bytes[0] = 1;
    bytes[6] = 0x70;
    bytes[8] = 0x80;
    bytes[15] = tail;
    BlockId::from_uuid(Uuid::from_bytes(bytes)).unwrap()
}

#[test]
fn given_a_parent_missing_from_the_index_when_containment_is_checked_then_unknown_reference_is_returned()
 {
    let entry = EntryId::from_uuid(block_id(9).as_uuid()).unwrap();
    let orphan = BlockRecord::new(
        block_id(1),
        entry,
        ContentBlockKind::Text,
        0,
        RepresentationState::Complete,
        OpaqueJson::empty_object(),
    )
    .with_parent(block_id(2));
    let blocks = [orphan];
    let rows: HashMap<BlockId, &BlockRecord> = HashMap::new();

    let error = check_containment(&blocks, &rows).unwrap_err();

    assert_eq!(error.code(), ErrorCode::UnknownReference);
    assert_eq!(error.object_id(), Some(block_id(1).as_uuid()));
    assert_eq!(error.location(), Some("parent"));
}
