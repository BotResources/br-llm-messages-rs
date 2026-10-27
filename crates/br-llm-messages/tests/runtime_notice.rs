mod common;

use br_llm_messages::{
    AITurn, ActorRef, Conversation, EntryRecord, ErrorCode, IncomingEntry, InputTurn, Perspective,
    RecordSet, RepresentationState, RuntimeNotice, Turn, TurnEntry, TurnEntryKind,
};
use common::Raw as _;
use common::turns::{human, response, tid};
use common::{aid, author, cvid, eid, oj, refused};
use serde_json::{Value, json};

fn notice(n: u64) -> RuntimeNotice {
    RuntimeNotice::new(eid(n), "the queue restarts at noon").unwrap()
}

fn history() -> Conversation {
    let mut turn = AITurn::open(tid(102), aid(2), author("Claude"));
    turn.push_entry(notice(11).into()).unwrap();
    turn.push_entry(response(
        12,
        vec![common::text_block(13, "ok").into()],
        br_llm_messages::ResponseEnding::finished(),
    ))
    .unwrap();
    turn.push_entry(notice(14).into()).unwrap();
    Conversation::from_parts(
        cvid(1),
        vec![
            InputTurn::new(
                tid(101),
                vec![human(10).try_into().unwrap(), notice(15).into()],
            )
            .unwrap()
            .into(),
            turn.into(),
        ],
    )
    .unwrap()
}

#[test]
fn given_a_runtime_notice_when_built_then_its_text_is_kept_and_a_blank_one_is_refused() {
    assert_eq!(notice(1).text(), "the queue restarts at noon");
    assert_eq!(notice(1).id(), eid(1));
    for blank in ["", "  \n\t"] {
        let error = refused(RuntimeNotice::new(eid(1), blank));

        assert_eq!(error.code(), ErrorCode::InvalidContent);
        assert_eq!(error.location(), Some("text"));
        assert_eq!(error.object_id(), Some(eid(1).as_uuid()));
    }
}

#[test]
fn given_a_runtime_notice_json_with_an_author_when_loaded_then_author_forbidden() {
    let value = json!({"type": "runtime_notice", "id": eid(1).to_string(),
        "text": "x", "author": "Claude"});

    let error = refused(TurnEntry::from_value(oj(value.clone())));

    assert_eq!(error.code(), ErrorCode::AuthorForbidden);
    assert_eq!(
        refused(IncomingEntry::from_value(oj(value))).code(),
        ErrorCode::AuthorForbidden
    );
}

#[test]
fn given_a_runtime_notice_json_with_blocks_or_without_text_when_loaded_then_invalid_content() {
    for value in [
        json!({"type": "runtime_notice", "id": eid(1).to_string(), "text": "x", "content": []}),
        json!({"type": "runtime_notice", "id": eid(1).to_string()}),
        json!({"type": "runtime_notice", "id": eid(1).to_string(), "text": "  "}),
        json!({"type": "runtime_notice", "id": eid(1).to_string(), "text": "x", "task": "y"}),
    ] {
        let error = refused(TurnEntry::from_value(oj(value.clone())));

        assert_eq!(error.code(), ErrorCode::InvalidContent, "{value}");
    }
}

#[test]
fn given_a_runtime_notice_when_serialized_then_the_golden_shape_loads_back_equal() {
    let entry: TurnEntry = notice(1).with_observed_at(common::at(1)).into();

    let value = entry.to_value().unwrap();

    assert_eq!(
        serde_json::to_value(&value).unwrap(),
        json!({"type": "runtime_notice", "id": eid(1).to_string(),
            "text": "the queue restarts at noon", "observed_at": common::at(1).as_str()})
    );
    assert_eq!(TurnEntry::from_value(value).unwrap(), entry);
    let incoming = IncomingEntry::from(notice(1));
    assert_eq!(incoming.kind(), Some(TurnEntryKind::RuntimeNotice));
    assert_eq!(
        IncomingEntry::from_json(&incoming.to_json().unwrap()).unwrap(),
        incoming
    );
}

#[test]
fn given_the_runtime_notice_kind_when_named_then_it_is_a_recorded_incoming_kind() {
    let kind: TurnEntryKind = "runtime_notice".parse().unwrap();

    assert_eq!(kind, TurnEntryKind::RuntimeNotice);
    assert!(kind.is_incoming());
    assert!(TurnEntry::from(notice(1)).is_incoming());
    assert_eq!(
        serde_json::from_value::<TurnEntryKind>(json!("runtime_notice")).unwrap(),
        kind
    );
}

#[test]
fn given_runtime_notices_in_an_input_turn_and_in_an_ai_turn_when_a_conversation_round_trips_then_it_is_equal()
 {
    let conversation = history();

    let reloaded = Conversation::from_json(&conversation.to_json().unwrap()).unwrap();

    assert_eq!(reloaded, conversation);
}

#[test]
fn given_runtime_notices_when_decomposed_then_rows_carry_no_sender_author_or_block_and_rebuild_equal()
 {
    let conversation = history();

    let set = RecordSet::from_conversation(&conversation).unwrap();

    let rows: Vec<&EntryRecord> = set
        .entries()
        .iter()
        .filter(|row| row.kind() == TurnEntryKind::RuntimeNotice)
        .collect();
    assert_eq!(rows.len(), 3);
    for row in rows {
        assert_eq!(row.state(), RepresentationState::Complete);
        assert!(row.sender().is_none() && row.author().is_none());
        assert_eq!(row.payload().raw()["text"], "the queue restarts at noon");
        assert!(set.blocks().iter().all(|block| block.entry() != row.id()));
    }
    let reloaded = RecordSet::from_json(&set.to_json().unwrap()).unwrap();
    assert_eq!(reloaded.to_conversation(cvid(1)).unwrap(), conversation);
}

#[test]
fn given_a_runtime_notice_row_with_a_sender_an_author_or_a_bad_payload_when_checked_locally_then_it_is_refused()
 {
    let row = |payload: Value| {
        EntryRecord::new(
            eid(1),
            tid(1),
            TurnEntryKind::RuntimeNotice,
            RepresentationState::Complete,
            oj(payload),
        )
    };
    assert!(row(json!({"text": "x"})).check_local().is_ok());

    let sender = row(json!({"text": "x"})).with_sender(ActorRef::agent(aid(1)));
    let authored = row(json!({"text": "x"})).with_author(author("Claude"));

    assert_eq!(
        sender.check_local().unwrap_err().code(),
        ErrorCode::DisallowedKind
    );
    assert_eq!(
        authored.check_local().unwrap_err().code(),
        ErrorCode::AuthorForbidden
    );
    for payload in [
        json!({}),
        json!({"text": " "}),
        json!({"text": "x", "extra": 1}),
    ] {
        assert_eq!(
            row(payload).check_local().unwrap_err().code(),
            ErrorCode::InvalidContent
        );
    }
}

#[test]
fn given_a_runtime_notice_in_another_agents_turn_when_projected_by_default_then_it_is_kept_and_the_door_agrees()
 {
    let conversation = history();
    let expected = Perspective::of(aid(1)).project(&conversation);

    let kept: Vec<TurnEntryKind> = expected
        .turns()
        .iter()
        .filter_map(Turn::as_input)
        .flat_map(|turn| turn.entries().iter().filter_map(IncomingEntry::kind))
        .filter(|kind| *kind == TurnEntryKind::RuntimeNotice)
        .collect();

    assert_eq!(kept.len(), 3);
    let from_rows = RecordSet::from_conversation(&conversation)
        .unwrap()
        .to_projected_thread(cvid(1), aid(1))
        .unwrap();
    assert_eq!(from_rows, expected);
}

#[test]
fn given_a_runtime_notice_in_the_target_agents_own_turn_when_projected_then_the_turn_is_unchanged()
{
    let conversation = history();

    let thread = Perspective::of(aid(2)).project(&conversation);

    assert_eq!(thread.turns()[1], conversation.turns()[1]);
}
