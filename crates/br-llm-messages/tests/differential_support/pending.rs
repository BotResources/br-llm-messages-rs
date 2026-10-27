use br_llm_messages::{ModelResponse, PendingResponse, ResponseDraft, TurnId};

pub enum Pending {
    Draft(Box<ResponseDraft>),
    Response(TurnId, Box<ModelResponse>),
}

impl Pending {
    pub fn as_pending(&self) -> PendingResponse<'_> {
        match self {
            Pending::Draft(draft) => PendingResponse::from(draft.as_ref()),
            Pending::Response(turn, response) => {
                PendingResponse::response(*turn, response.as_ref())
            }
        }
    }

    pub fn document(&self) -> String {
        match self {
            Pending::Draft(draft) => draft_document(draft),
            Pending::Response(_, response) => serde_json::to_string(response).unwrap(),
        }
    }
}

pub fn draft_document(draft: &ResponseDraft) -> String {
    let slots: Vec<String> = draft
        .view()
        .slots()
        .iter()
        .map(|slot| slot.block().as_uuid().to_string())
        .collect();
    let body = match (draft.response(), draft.to_snapshot()) {
        (Some(response), _) => serde_json::to_string(response).unwrap(),
        (None, Ok(snapshot)) => format!(
            "{}{}",
            serde_json::to_string(snapshot.entry()).unwrap(),
            serde_json::to_string(snapshot.blocks()).unwrap()
        ),
        (None, Err(_)) => String::new(),
    };
    format!("{} {} {}", draft.entry().as_uuid(), slots.join(" "), body)
}
