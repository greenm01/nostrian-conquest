//! The crippled-hull counters: recovery timing, the starbase's extra turn,
//! and the order hulls leave in. The arithmetic mirrors the cohort model in
//! `tools/combat-model/recovery.py`.

use nc_data::{CrippledHulls, CrippledShips, HullClass, RECOVERY_TURNS, Recovered};

fn hulls(a: u32, b: u32, fresh: u32) -> CrippledHulls {
    CrippledHulls {
        turns: [a, b],
        fresh,
    }
}

/// Passes that must run after the pass that cripples one hull before it is
/// whole, with or without starbase assistance (the model's `timeline`).
fn passes_until_whole(assisted: bool) -> usize {
    let mut state = CrippledHulls::default();
    state.cripple(1);
    state.recover(&CrippledHulls::default());
    let helped = |state: &CrippledHulls| {
        if assisted {
            *state
        } else {
            CrippledHulls::default()
        }
    };
    for pass in 1.. {
        let assist = helped(&state);
        if state.recover(&assist).hulls == 1 {
            return pass;
        }
    }
    unreachable!()
}

#[test]
fn rt_is_two_for_every_class() {
    assert_eq!(RECOVERY_TURNS, 2);
}

#[test]
fn a_hull_crippled_in_pass_n_is_whole_after_pass_n_plus_two() {
    // Crippled in N; the step ending N enters it at RT; steps N+1 and N+2
    // count it down, so it fights whole from N+3.
    assert_eq!(passes_until_whole(false), 2);
}

#[test]
fn starbase_assistance_brings_a_hull_back_one_pass_early() {
    assert_eq!(passes_until_whole(true), 1);
}

#[test]
fn this_pass_damage_enters_at_rt_and_never_recovers_in_its_own_step() {
    for assisted in [CrippledHulls::default(), hulls(5, 5, 0)] {
        let mut state = hulls(0, 0, 3);
        let recovered = state.recover(&assisted);
        assert_eq!(recovered, Recovered::default());
        assert_eq!(state, hulls(0, 3, 0));
    }
}

#[test]
fn the_step_counts_every_cohort_down_by_one_and_recovers_the_last() {
    let mut state = hulls(4, 3, 2);
    let recovered = state.recover(&CrippledHulls::default());
    assert_eq!(recovered, Recovered { hulls: 4, early: 0 });
    assert_eq!(state, hulls(3, 2, 0));
}

#[test]
fn assistance_moves_only_the_assisted_hulls_one_extra_turn() {
    let mut state = hulls(2, 4, 0);
    let recovered = state.recover(&hulls(2, 3, 0));
    // Two with one turn left recover either way; of the four with two left,
    // the three assisted recover early and the other one has a turn to go.
    assert_eq!(recovered, Recovered { hulls: 5, early: 3 });
    assert_eq!(state, hulls(1, 0, 0));
}

#[test]
fn assistance_beyond_the_hulls_present_is_ignored() {
    let mut state = hulls(0, 1, 0);
    let recovered = state.recover(&hulls(9, 9, 9));
    assert_eq!(recovered, Recovered { hulls: 1, early: 1 });
    assert!(state.is_empty());
}

#[test]
fn a_late_merge_does_not_pass_its_hulls_off_as_assisted() {
    // 3 hulls were here at the start and stayed; 4 more merged in.
    let assisted = hulls(0, 3, 0);
    let mut merged = hulls(0, 3, 0);
    merged.add(&hulls(0, 4, 0));
    let present = merged;
    let recovered = merged.recover(&assisted.limited_to(&present));
    assert_eq!(recovered, Recovered { hulls: 3, early: 3 });
    assert_eq!(merged, hulls(4, 0, 0));
}

#[test]
fn losses_cost_assistance_only_after_the_unassisted_hulls_are_gone() {
    let assisted = hulls(0, 3, 0);
    // Two unassisted hulls joined the three; finishing fire takes two.
    let mut now = hulls(0, 5, 0);
    now.take(2);
    assert_eq!(assisted.limited_to(&now), hulls(0, 3, 0));
    // A third loss reaches an assisted hull.
    now.take(1);
    assert_eq!(assisted.limited_to(&now), hulls(0, 2, 0));
}

#[test]
fn take_removes_this_pass_first_then_the_most_turns_left() {
    let mut state = hulls(3, 4, 2);
    assert_eq!(state.take(2), hulls(0, 0, 2));
    assert_eq!(state.take(5), hulls(1, 4, 0));
    assert_eq!(state, hulls(2, 0, 0));
}

#[test]
fn take_never_removes_more_than_exist() {
    let mut state = hulls(1, 0, 1);
    assert_eq!(state.take(10), hulls(1, 0, 1));
    assert!(state.is_empty());
}

#[test]
fn a_split_and_merge_conserves_every_counter() {
    let original = hulls(3, 4, 2);
    let mut kept = original;
    let detached = kept.take(6);
    let mut rejoined = kept;
    rejoined.add(&detached);
    assert_eq!(rejoined, original);
    // The counts add, so recovering the parts equals recovering the whole.
    let (mut whole, mut a, mut b) = (original, kept, detached);
    let none = CrippledHulls::default();
    let total = whole.recover(&none).hulls;
    assert_eq!(a.recover(&none).hulls + b.recover(&none).hulls, total);
    a.add(&b);
    assert_eq!(a, whole);
}

#[test]
fn ships_sum_their_classes() {
    let mut crippled = CrippledShips::default();
    assert!(crippled.is_empty());
    crippled.get_mut(HullClass::Battleship).cripple(1);
    crippled.get_mut(HullClass::Destroyer).cripple(2);
    crippled.get_mut(HullClass::Destroyer).turns[1] += 1;
    assert_eq!(crippled.total(), 4);

    let mut other = CrippledShips::default();
    other.get_mut(HullClass::Battleship).cripple(2);
    crippled.add(&other);
    assert_eq!(crippled.get(HullClass::Battleship).total(), 3);
}
