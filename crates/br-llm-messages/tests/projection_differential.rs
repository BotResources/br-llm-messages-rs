mod common;
mod differential_support;

use br_llm_messages::EntryId;
use differential_support::{HOSTILE, Hostile, agree, scenario};

fn selected(id: EntryId) -> bool {
    !id.as_uuid().as_u128().is_multiple_of(3)
}

fn run(hostile: Hostile) -> (usize, usize) {
    let (mut accepted, mut refused) = (0, 0);
    for seed in 0..60 {
        let case = scenario(seed, hostile);
        for agent in (1..=4).filter(|agent| case.interrupted_owner != Some(*agent)) {
            for select in [None, Some(selected as fn(EntryId) -> bool)] {
                let label = format!(
                    "{hostile:?} seed {seed} agent {agent} select {}",
                    select.is_some()
                );
                if agree(&case, agent, select, &label) {
                    accepted += 1;
                } else {
                    refused += 1;
                }
            }
        }
    }
    (accepted, refused)
}

#[test]
fn given_clean_histories_with_pending_responses_when_projected_from_memory_or_from_reduced_rows_then_both_agree()
 {
    let (accepted, _) = run(Hostile::Clean);

    assert!(accepted > 100, "most clean cases must project");
}

#[test]
fn given_every_hostile_reuse_when_projected_from_memory_or_from_reduced_rows_then_both_agree() {
    for hostile in HOSTILE {
        let (accepted, refused) = run(hostile);

        assert!(accepted + refused > 0, "{hostile:?}");
    }
}

#[test]
fn given_an_identity_reused_by_an_input_or_foreign_turn_when_projected_then_some_cases_are_refused()
{
    for hostile in [
        Hostile::InputEntry,
        Hostile::InputBlock,
        Hostile::ForeignEntry,
        Hostile::ForeignBlock,
        Hostile::PendingEntry,
        Hostile::PendingBlock,
        Hostile::InterruptedEntry,
        Hostile::InterruptedBlock,
    ] {
        let (accepted, refused) = run(hostile);

        assert!(refused > 0, "{hostile:?} must hit a visible identity");
        assert!(accepted > 0, "{hostile:?} must also hit a hidden identity");
    }
}

#[test]
fn given_an_interrupted_draft_holding_children_and_citations_when_a_retry_reuses_one_then_both_paths_refuse_it()
 {
    let (accepted, refused) = run(Hostile::InterruptedChild);

    assert!(refused > 0, "the reused child or citation must be refused");
    assert!(accepted + refused > 0);
}
