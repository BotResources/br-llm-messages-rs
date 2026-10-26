mod common;
mod projected_thread_support;

use std::sync::atomic::{AtomicUsize, Ordering};

use br_llm_messages::{
    AITurn, ClosureReason, ErrorCode, InputTurn, Perspective, ProjectedThread, Turn,
};
use common::turns::{close, human, human_message, input_turn, tid};
use common::{aid, at, author, bid, cvid, oj, refused};
use projected_thread_support::*;
use serde_json::{Value, json};

#[test]
fn given_a_selection_when_projected_then_it_is_called_once_per_foreign_incoming_entry_and_nowhere_else()
 {
    let turns = vec![
        input_turn(1, 2).into(),
        ai(10, 1, vec![human(14), said(15, 16, "mine")]),
        ai(
            20,
            2,
            vec![
                said(21, 22, "a"),
                human(23),
                said(24, 25, "b"),
                human(26),
                close(27, ClosureReason::completed()),
            ],
        ),
    ];
    let calls = std::sync::Arc::new(AtomicUsize::new(0));
    let counter = calls.clone();
    let perspective = Perspective::of(aid(1)).keeping(move |_| {
        counter.fetch_add(1, Ordering::SeqCst);
        true
    });

    let thread = perspective.project_turns(cvid(900), &turns).unwrap();

    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert_eq!(thread.len(), 3);
}

#[test]
fn given_no_selection_when_projected_then_every_foreign_incoming_entry_is_kept() {
    let turns = vec![ai(20, 2, vec![human(23), human(26)])];

    let thread = Perspective::of(aid(1))
        .project_turns(cvid(900), &turns)
        .unwrap();

    let Turn::InputTurn(input) = &thread.turns()[0] else {
        panic!("input turn expected");
    };
    assert_eq!(input.entries().len(), 2);
}

#[test]
fn given_observed_times_when_projected_then_translated_turns_and_outputs_carry_none_and_kept_entries_keep_theirs()
 {
    let kept = common::turns::human_message(23).with_observed_at(at(4));
    let timed = AITurn::from_parts(
        tid(20),
        aid(2),
        author("Claude"),
        vec![kept.clone().into(), said(21, 22, "a")],
    )
    .unwrap()
    .with_observed_at(at(5));
    let stored = InputTurn::single(tid(1), human_message(2))
        .unwrap()
        .with_observed_at(at(6));
    let turns: Vec<Turn> = vec![stored.into(), timed.into()];

    let thread = Perspective::of(aid(1))
        .project_turns(cvid(900), &turns)
        .unwrap();

    let Turn::InputTurn(stored) = &thread.turns()[0] else {
        panic!("input turn expected");
    };
    assert_eq!(stored.observed_at(), Some(&at(6)));
    let Turn::InputTurn(translated) = &thread.turns()[1] else {
        panic!("input turn expected");
    };
    assert_eq!(translated.observed_at(), None);
    assert_eq!(translated.entries()[0].observed_at(), Some(&at(4)));
    assert_eq!(translated.entries()[1].observed_at(), None);
    let reloaded = ProjectedThread::from_json(&thread.to_json().unwrap()).unwrap();
    assert_eq!(reloaded, thread);
}

#[test]
fn given_an_input_turn_with_or_without_observed_at_when_a_thread_is_loaded_then_both_are_accepted()
{
    let mut with = document();
    with["turns"][2]["observed_at"] = json!("2026-10-23T08:00:00Z");

    let loaded = load(with).unwrap();

    let Turn::InputTurn(input) = &loaded.turns()[2] else {
        panic!("input turn expected");
    };
    assert!(input.observed_at().is_some());
    assert!(load(document()).is_ok());
}

#[test]
fn given_an_output_or_a_partial_with_observed_at_when_loaded_then_disallowed_kind_at_the_entry() {
    for (turn, expected) in [
        (2, "turns[2].entries[0].observed_at"),
        (3, "turns[3].entries[0].observed_at"),
    ] {
        let mut document = document();
        document["turns"][turn]["entries"][0]["observed_at"] = json!("2026-10-23T08:00:00Z");

        let error = refused(load(document));

        assert_eq!(error.code(), ErrorCode::DisallowedKind);
        assert_eq!(error.location(), Some(expected));
    }
}

#[test]
fn given_a_block_in_raw_fragment_state_when_an_output_is_loaded_then_invalid_content() {
    let raw = [
        json!({"type": "structured_content", "id": bid(700).to_string(), "raw": "{\"a\":"}),
        json!({"type": "structured_content", "id": bid(700).to_string(), "fragments": ["{"]}),
        json!({"type": "structured_content", "id": bid(700).to_string()}),
        json!({"type": "text", "id": bid(700).to_string()}),
    ];
    for turn in [2, 3] {
        for block in &raw {
            let mut document = document();
            document["turns"][turn]["entries"][0]["content"] = json!([block]);

            let error = refused(load(document));

            assert_eq!(error.code(), ErrorCode::InvalidContent, "{block}");
        }
    }
}

#[test]
fn given_an_output_or_a_partial_without_block_or_with_blank_text_when_loaded_then_invalid_content()
{
    for turn in [2, 3] {
        for content in [
            json!([]),
            json!([{"type": "text", "id": bid(700).to_string(), "text": " \n"}]),
        ] {
            let mut document = document();
            document["turns"][turn]["entries"][0]["content"] = content;

            let error = refused(load(document));

            assert_eq!(error.code(), ErrorCode::InvalidContent);
        }
    }
}

#[test]
fn given_a_1x_input_turn_with_an_entry_when_loaded_standalone_then_invalid_content() {
    let old = json!({"type": "input_turn", "id": tid(1).to_string(),
        "entry": {"type": "human_message", "id": common::eid(2).to_string(),
            "human": common::hid(1).to_string(),
            "content": [{"type": "text", "id": bid(3).to_string(), "text": "hi"}]}});

    assert_eq!(
        refused(InputTurn::from_value(oj(old.clone()))).code(),
        ErrorCode::InvalidContent
    );
    assert_eq!(
        refused(Turn::from_value(oj(old))).code(),
        ErrorCode::InvalidContent
    );
}

#[test]
fn given_the_schemas_when_read_then_only_the_conversation_and_the_new_thread_changed() {
    assert_eq!(br_llm_messages::CONVERSATION_SCHEMA, "br-llm-messages/5");
    assert_eq!(
        br_llm_messages::PROJECTED_THREAD_SCHEMA,
        "br-llm-messages-projected-thread/1"
    );
    assert_eq!(br_llm_messages::RECORD_SCHEMA, "br-llm-messages-records/1");
    assert_eq!(br_llm_messages::DRAFT_SCHEMA, "br-llm-messages-draft/1");
    assert_eq!(br_llm_messages::DELTA_SCHEMA, "br-llm-messages-delta/1");
}

#[test]
fn given_a_thread_when_rebuilt_by_a_runner_from_its_json_then_it_is_equal_and_revalidated() {
    let thread = thread();
    let text = thread.to_json().unwrap();

    let rebuilt = ProjectedThread::from_json(&text).unwrap();

    assert_eq!(rebuilt, thread);
    let mut broken: Value = serde_json::from_str(&text).unwrap();
    broken["turns"][1]["entries"][0]["extra"] = json!(1);
    assert_eq!(refused(load(broken)).code(), ErrorCode::InvalidContent);
}
