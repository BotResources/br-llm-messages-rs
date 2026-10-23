use super::position;
use crate::error::ErrorCode;

#[test]
fn given_an_index_within_u32_when_converted_then_the_position_is_kept() {
    assert_eq!(position(7).unwrap(), 7);
    assert_eq!(position(u32::MAX as usize).unwrap(), u32::MAX);
}

#[cfg(target_pointer_width = "64")]
#[test]
fn given_an_index_beyond_u32_when_converted_then_invalid_position_is_returned() {
    let error = position(u32::MAX as usize + 1).unwrap_err();

    assert_eq!(error.code(), ErrorCode::InvalidPosition);
}
