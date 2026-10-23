mod common;
use common::Raw as _;

use br_llm_messages::{
    AITurn, Conversation, ErrorCode, ExecutionStatus, ResponseEnding, TurnEntry,
};
use common::turns::{
    calling, close, delivery, finished, hosted_call, hosted_record, hosted_result, human,
    input_turn, response, tid,
};
use common::{aid, author, cvid, refused};
use serde_json::{Value, json};

struct Case {
    name: &'static str,
    entries: Vec<TurnEntry>,
    code: ErrorCode,
    index: usize,
}

fn cases() -> Vec<Case> {
    let completed = || close(90, br_llm_messages::ClosureReason::completed());
    vec![
        Case {
            name: "entry after closure",
            entries: vec![human(10), completed(), human(12)],
            code: ErrorCode::ClosedTurn,
            index: 2,
        },
        Case {
            name: "second closure",
            entries: vec![
                completed(),
                close(91, br_llm_messages::ClosureReason::completed()),
            ],
            code: ErrorCode::ClosedTurn,
            index: 1,
        },
        Case {
            name: "response with outstanding calls",
            entries: vec![calling(10, &[(11, "a")]), finished(12)],
            code: ErrorCode::UnresolvedExecution,
            index: 1,
        },
        Case {
            name: "result for an unknown call",
            entries: vec![calling(10, &[(11, "a")]), delivery(20, &[(21, 99, "a")])],
            code: ErrorCode::UnknownReference,
            index: 1,
        },
        Case {
            name: "result for a provider call",
            entries: vec![
                response(10, vec![hosted_call(11)], ResponseEnding::finished()),
                delivery(20, &[(21, 11, "a")]),
            ],
            code: ErrorCode::UnknownReference,
            index: 1,
        },
        Case {
            name: "second result for a call",
            entries: vec![
                calling(10, &[(11, "a")]),
                delivery(20, &[(21, 11, "a")]),
                delivery(30, &[(31, 11, "a")]),
            ],
            code: ErrorCode::DuplicateResult,
            index: 2,
        },
        Case {
            name: "result naming another tool",
            entries: vec![calling(10, &[(11, "a")]), delivery(20, &[(21, 11, "b")])],
            code: ErrorCode::MismatchedResult,
            index: 1,
        },
        Case {
            name: "provider result without call",
            entries: vec![response(
                10,
                vec![hosted_result(11, 99)],
                ResponseEnding::finished(),
            )],
            code: ErrorCode::UnknownReference,
            index: 0,
        },
        Case {
            name: "completed with outstanding calls",
            entries: vec![calling(10, &[(11, "a")]), completed()],
            code: ErrorCode::UnresolvedExecution,
            index: 1,
        },
        Case {
            name: "completed with pending provider execution",
            entries: vec![
                response(
                    10,
                    vec![hosted_record(11, ExecutionStatus::Running)],
                    ResponseEnding::finished(),
                ),
                completed(),
            ],
            code: ErrorCode::UnresolvedExecution,
            index: 1,
        },
        Case {
            name: "completed after provider continuation",
            entries: vec![
                response(10, vec![], ResponseEnding::provider_continuation()),
                completed(),
            ],
            code: ErrorCode::UnresolvedExecution,
            index: 1,
        },
        Case {
            name: "entry id used twice",
            entries: vec![human(10), human(10)],
            code: ErrorCode::ReusedIdentity,
            index: 1,
        },
    ]
}

fn document(turn_values: Vec<Value>) -> Value {
    json!({
        "schema": "br-llm-messages/4",
        "id": cvid(1000).to_string(),
        "turns": turn_values,
    })
}

fn raw_turn(entries: &[TurnEntry]) -> Value {
    json!({
        "type": "ai_turn",
        "id": tid(50).to_string(),
        "agent": aid(1).to_string(),
        "author": "Codex",
        "entries": entries.iter().map(|e| e.to_value().unwrap().raw()).collect::<Vec<_>>(),
    })
}

fn built(entries: &[TurnEntry]) -> Result<AITurn, br_llm_messages::Error> {
    AITurn::from_parts(tid(50), aid(1), author("Codex"), entries.to_vec())
}

#[test]
fn given_json_breaking_a_build_rule_when_loaded_then_the_same_code_and_a_precise_location() {
    for case in cases() {
        let direct = refused(built(&case.entries));
        let input = serde_json::to_value(input_turn(1, 2)).unwrap();
        let loaded = refused(Conversation::from_value(common::oj(document(vec![
            input,
            raw_turn(&case.entries),
        ]))));

        assert_eq!(direct.code(), case.code, "{}", case.name);
        assert_eq!(loaded.code(), case.code, "{}", case.name);
        assert_eq!(loaded.object_id(), direct.object_id(), "{}", case.name);
        let expected = format!("turns[1].{}", direct.location().unwrap());
        assert_eq!(loaded.location(), Some(expected.as_str()), "{}", case.name);
        assert!(
            direct
                .location()
                .unwrap()
                .starts_with(&format!("entries[{}]", case.index)),
            "{}",
            case.name
        );
    }
}

#[test]
fn given_a_clash_between_turns_when_loaded_then_reused_identity_locates_the_entry() {
    let first = serde_json::to_value(input_turn(1, 2)).unwrap();
    let second = raw_turn(&[human(10), human(12)]);
    let third = raw_turn(&[human(20), human(2)]);
    let mut third = third;
    third["id"] = json!(tid(60).to_string());

    let direct = refused(Conversation::from_parts(
        cvid(1000),
        vec![
            input_turn(1, 2).into(),
            built(&[human(10), human(12)]).unwrap().into(),
            AITurn::from_parts(tid(60), aid(1), author("Codex"), vec![human(20), human(2)])
                .unwrap()
                .into(),
        ],
    ));
    let loaded = refused(Conversation::from_value(common::oj(document(vec![
        first, second, third,
    ]))));

    assert_eq!(direct.code(), ErrorCode::ReusedIdentity);
    assert_eq!(loaded.code(), ErrorCode::ReusedIdentity);
    assert_eq!(loaded.location(), Some("turns[2].entries[1]"));
    assert_eq!(loaded.object_id(), Some(common::eid(2).as_uuid()));
}

#[test]
fn given_a_turn_id_reused_by_a_later_turn_when_loaded_then_reused_identity() {
    let first = serde_json::to_value(input_turn(1, 2)).unwrap();
    let mut second = raw_turn(&[human(10)]);
    second["id"] = first["id"].clone();

    let loaded = refused(Conversation::from_value(common::oj(document(vec![
        first, second,
    ]))));

    assert_eq!(loaded.code(), ErrorCode::ReusedIdentity);
    assert_eq!(loaded.location(), Some("turns[1]"));
}

#[test]
fn given_a_shape_error_in_an_entry_when_loaded_then_the_location_still_names_it() {
    let mut bad = human(10).to_value().unwrap().raw();
    bad["content"] = json!([]);
    let mut turn = raw_turn(&[finished(5)]);
    turn["entries"] = json!([finished(5).to_value().unwrap().raw(), bad]);

    let error = refused(Conversation::from_value(common::oj(document(vec![turn]))));

    assert_eq!(error.code(), ErrorCode::InvalidContent);
    assert!(error.location().unwrap().starts_with("turns[0].entries[1]"));
}

#[test]
fn given_valid_entries_replayed_through_json_when_loaded_then_the_turn_equals_the_built_one() {
    let entries = vec![
        calling(10, &[(11, "a"), (12, "b")]),
        delivery(20, &[(21, 12, "b")]),
        delivery(30, &[(31, 11, "a")]),
        finished(40),
        close(55, br_llm_messages::ClosureReason::completed()),
    ];
    let direct = built(&entries).unwrap();

    let loaded = Conversation::from_value(common::oj(document(vec![raw_turn(&entries)]))).unwrap();

    assert_eq!(loaded.turns()[0].as_ai().unwrap(), &direct);
}
