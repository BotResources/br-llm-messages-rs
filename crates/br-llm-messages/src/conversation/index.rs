use std::collections::HashMap;

use uuid::Uuid;

use crate::id::ConversationId;
use crate::turn::{Class, Declared, Place};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Slot {
    pub(crate) turn: usize,
    pub(crate) place: Place,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Index(HashMap<Uuid, Slot>);

impl Index {
    pub(crate) fn seeded(conversation: ConversationId) -> Self {
        let place = Place {
            class: Class::Conversation,
            entry: 0,
            root: 0,
            child: None,
        };
        Index(HashMap::from([(
            conversation.as_uuid(),
            Slot { turn: 0, place },
        )]))
    }

    pub(crate) fn get(&self, id: Uuid) -> Option<&Slot> {
        self.0.get(&id)
    }

    pub(crate) fn clash(&self, declared: &Declared) -> Option<(Uuid, Place)> {
        declared
            .iter()
            .find(|(id, _)| self.0.contains_key(id))
            .map(|(id, place)| (*id, *place))
    }

    pub(crate) fn insert(&mut self, turn: usize, declared: Declared) {
        for (id, place) in declared {
            self.0.insert(id, Slot { turn, place });
        }
    }
}
