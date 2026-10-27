mod common;
use common::Raw as _;

use br_llm_messages::{
    AgentOutput, ErrorCode, IncomingEntry, InputTurn, PartialAgentOutput, ProjectedThread, Turn,
};
use common::turns::{human_message, tid};
use common::{aid, author, cvid, eid, plain_text, refused};

fn output(entry: u64, block: u64) -> IncomingEntry {
    AgentOutput::new(
        eid(entry),
        aid(2),
        author("Codex"),
        vec![plain_text(block, "out")],
    )
    .unwrap()
    .into()
}

fn projected_turn() -> InputTurn {
    InputTurn::new_projected(tid(10), vec![human_message(11).into(), output(12, 13)]).unwrap()
}

#[test]
fn given_a_standalone_input_turn_holding_an_output_when_serialized_then_disallowed_kind_where_the_loader_refuses_it()
 {
    let turn = projected_turn();

    let json = refused(turn.to_json());
    let value = refused(turn.to_value());
    let loaded = refused(InputTurn::from_json(
        &projected_thread_document().to_string(),
    ));

    assert_eq!(json.code(), ErrorCode::DisallowedKind);
    assert_eq!(json.object_id(), Some(eid(12).as_uuid()));
    assert_eq!(json.location(), Some("entries[1]"));
    assert_eq!(value, json);
    assert_eq!(loaded.code(), json.code());
    assert!(serde_json::to_string(&turn).is_err());
}

#[test]
fn given_a_partial_output_when_the_standalone_turn_is_serialized_then_disallowed_kind() {
    let partial: IncomingEntry = PartialAgentOutput::new(
        eid(20),
        aid(2),
        author("Codex"),
        vec![plain_text(21, "half")],
    )
    .unwrap()
    .into();
    let turn = InputTurn::new_projected(tid(10), vec![partial]).unwrap();

    assert_eq!(refused(turn.to_json()).code(), ErrorCode::DisallowedKind);
}

#[test]
fn given_a_turn_holding_an_output_when_serialized_as_a_turn_then_disallowed_kind() {
    let turn: Turn = projected_turn().into();

    assert_eq!(refused(turn.to_json()).code(), ErrorCode::DisallowedKind);
    assert_eq!(refused(turn.to_value()).code(), ErrorCode::DisallowedKind);
}

#[test]
fn given_a_projected_thread_holding_such_a_turn_when_serialized_then_it_round_trips() {
    let thread =
        ProjectedThread::from_turns(cvid(1), aid(1), vec![projected_turn().into()]).unwrap();

    let reloaded = ProjectedThread::from_json(&thread.to_json().unwrap()).unwrap();
    let from_serde: ProjectedThread =
        serde_json::from_str(&serde_json::to_string(&thread).unwrap()).unwrap();

    assert_eq!(reloaded, thread);
    assert_eq!(from_serde, thread);
    assert_eq!(thread.check(), Ok(()));
}

#[test]
fn given_a_standalone_input_turn_without_outputs_when_serialized_then_it_round_trips() {
    let turn = InputTurn::new(tid(10), vec![human_message(11).into()]).unwrap();

    assert_eq!(
        InputTurn::from_json(&turn.to_json().unwrap()).unwrap(),
        turn
    );
}

fn projected_thread_document() -> serde_json::Value {
    let thread =
        ProjectedThread::from_turns(cvid(1), aid(1), vec![projected_turn().into()]).unwrap();
    thread.to_value().unwrap().raw()["turns"][0].clone()
}
