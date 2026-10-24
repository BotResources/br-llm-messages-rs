use br_llm_messages::{AITurn, ClosureReason, ErrorCode, ResponseEnding, TurnEntry};

use super::turns::{close, delivery, finished, human, open_turn, pushed, response};
use super::{call, refused};

#[derive(Clone, Copy)]
pub struct Accepts {
    pub incoming: bool,
    pub response: bool,
    pub results: bool,
    pub completed: bool,
    pub abnormal: bool,
}

pub const fn accepts(
    incoming: bool,
    response: bool,
    results: bool,
    completed: bool,
    abnormal: bool,
) -> Accepts {
    Accepts {
        incoming,
        response,
        results,
        completed,
        abnormal,
    }
}

pub const NOTHING: Accepts = accepts(false, false, false, false, false);

fn probes() -> [(bool, TurnEntry); 5] {
    [
        (false, human(900)),
        (false, finished(910)),
        (false, delivery(920, &[(921, 11, "a")])),
        (true, close(930, ClosureReason::completed())),
        (false, close(940, ClosureReason::cancelled())),
    ]
}

pub fn judge_row(name: &str, turn: &AITurn, expected: Accepts) {
    let wanted = [
        expected.incoming,
        expected.response,
        expected.results,
        expected.completed,
        expected.abnormal,
    ];
    for (index, ((completed_closure, entry), accepted)) in
        probes().into_iter().zip(wanted).enumerate()
    {
        let mut attempt = turn.clone();
        let outcome = attempt.push_entry(entry.clone());
        if accepted {
            assert!(outcome.is_ok(), "{name}: probe {index} should be accepted");
            assert_eq!(attempt.entries().len(), turn.entries().len() + 1);
            continue;
        }
        let error = refused(outcome);
        let expected_code = if turn.is_closed() {
            ErrorCode::ClosedTurn
        } else if completed_closure {
            ErrorCode::UnresolvedExecution
        } else {
            ErrorCode::EntryOutOfOrder
        };
        assert_eq!(error.code(), expected_code, "{name}: probe {index}");
        if !completed_closure {
            assert_eq!(error.object_id(), Some(entry.id().as_uuid()), "{name}");
        }
        assert_eq!(&attempt, turn, "{name}: probe {index} must change nothing");
    }
}

pub fn runtime_call_response(ending: ResponseEnding) -> TurnEntry {
    response(10, vec![call(11, "a").into()], ending)
}

pub fn after(entries: Vec<TurnEntry>) -> AITurn {
    let mut turn = open_turn(1);
    pushed(&mut turn, entries);
    turn
}
