use uuid::Uuid;

use super::Conversation;

impl Conversation {
    pub(crate) fn holds_identity(&self, id: Uuid) -> bool {
        self.index.get(id).is_some()
    }
}
