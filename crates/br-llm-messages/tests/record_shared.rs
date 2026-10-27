mod common;
mod projection_support;

use br_llm_messages::{Conversation, Perspective, RecordSet};
use common::conversations::rich;
use common::records::{edited, rich_set, row};
use common::turns::{human, input_turn};
use common::{aid, cvid, v7};
use projection_support::{ai, said};
use serde_json::json;

#[test]
fn given_a_record_set_when_reconstructed_by_reference_twice_then_it_is_not_consumed_and_both_equal_the_consuming_call()
 {
    let set = rich_set();
    let untouched = set.clone();
    let id = rich().id();

    let first = set.to_conversation(id).unwrap();
    let second = set.to_conversation(id).unwrap();

    assert_eq!(set, untouched);
    assert_eq!(first, second);
    assert_eq!(first, rich());
    assert_eq!(first, set.clone().into_conversation(id).unwrap());
    assert_eq!(set.to_turns().unwrap(), set.clone().into_turns().unwrap());
}

fn shared_history() -> Conversation {
    Conversation::from_parts(
        cvid(900),
        vec![
            input_turn(1, 2).into(),
            ai(10, 2, vec![human(13), said(11, 12, "from two")]),
            ai(20, 3, vec![said(21, 22, "from three")]),
            ai(30, 1, vec![said(31, 32, "from one")]),
        ],
    )
    .unwrap()
}

#[test]
fn given_one_record_set_when_projected_by_reference_for_several_agents_then_each_equals_the_in_memory_projection()
 {
    let conversation = shared_history();
    let set = RecordSet::from_conversation(&conversation).unwrap();
    let untouched = set.clone();
    for agent in [aid(1), aid(2), aid(3)] {
        let by_reference = set.to_projected_thread(conversation.id(), agent).unwrap();

        assert_eq!(
            by_reference,
            Perspective::of(agent).project(&conversation),
            "{agent:?}"
        );
        assert_eq!(
            by_reference,
            set.clone()
                .into_projected_thread(conversation.id(), agent)
                .unwrap()
        );
        assert_eq!(
            by_reference,
            set.to_projected_thread_pending(conversation.id(), agent, &[])
                .unwrap()
        );
    }
    assert_eq!(set, untouched);
}

#[test]
fn given_a_sound_record_set_when_checked_as_a_whole_then_it_is_accepted() {
    assert!(rich_set().check().is_ok());
    assert!(RecordSet::new().check().is_ok());
}

#[test]
fn given_sets_that_reconstruction_refuses_when_checked_as_a_whole_then_the_same_refusal_comes_without_building()
 {
    let broken: Vec<(&str, RecordSet)> = vec![
        (
            "reused identity",
            edited(&rich_set(), |document| {
                let first = document["entries"][0]["id"].clone();
                document["entries"][1]["id"] = first;
            }),
        ),
        (
            "dangling turn",
            edited(&rich_set(), |document| {
                document["entries"][0]["turn"] = json!(v7(9_999).hyphenated().to_string());
            }),
        ),
        (
            "dangling entry",
            edited(&rich_set(), |document| {
                document["blocks"][0]["entry"] = json!(v7(9_999).hyphenated().to_string());
            }),
        ),
        (
            "turn rules broken by the order of the rows",
            edited(&rich_set(), |document| {
                document["entries"].as_array_mut().unwrap().reverse();
            }),
        ),
        (
            "incomplete row",
            edited(&rich_set(), |document| {
                let id = rich_set().blocks()[0].id().as_uuid();
                row(document, "blocks", id)["state"] = json!("producing");
            }),
        ),
    ];
    for (label, set) in broken {
        let checked = set.check().unwrap_err();

        let rebuilt = set.to_conversation(rich().id()).unwrap_err();

        assert_eq!(checked.code(), rebuilt.code(), "{label}");
        assert_eq!(checked.location(), rebuilt.location(), "{label}");
    }
}
