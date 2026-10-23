use std::collections::HashSet;

use uuid::Uuid;

use crate::error::{Error, ErrorCode};

#[derive(Default)]
pub(crate) struct IdSink(HashSet<Uuid>);

impl IdSink {
    pub(crate) fn insert(&mut self, uuid: Uuid) -> Result<(), Error> {
        if self.0.insert(uuid) {
            Ok(())
        } else {
            Err(Error::new(ErrorCode::ReusedIdentity).with_id(uuid))
        }
    }
}
