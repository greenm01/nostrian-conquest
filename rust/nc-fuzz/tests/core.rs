//! The shared fuzz core: fixed choice lists play clean games, a written
//! scenario replays to the same state, and the invariants catch tampering.

use nc_data::{CoreGameData, MaintenanceEvents, default_player_lifecycle_states};
use nc_engine::build_seeded_initialized_game;
use nc_fuzz::{
    CaseRun, ChoiceTape, FuzzConfig, TurnObservation, Violation, check_turn, run_case,
    write_scenario,
};
use nc_harness::{CombatScenarioSpec, run_combat_scenario};

/// A fixed pseudo-random choice list.
fn tape(seed: u64) -> ChoiceTape {
    let mut state = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
    ChoiceTape::new(
        (0..8192)
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                (state >> 32) as u32
            })
            .collect(),
    )
}

fn play(seed: u64) -> CaseRun {
    run_case(&mut tape(seed), &FuzzConfig { max_turns: 6 })
}

#[test]
fn fixed_choice_lists_play_clean_games() {
    for seed in 0..12 {
        let run = play(seed);
        assert!(
            run.violations.is_empty(),
            "seed {seed}: {:?}",
            run.violations
        );
        assert!(!run.turns.is_empty(), "seed {seed} played no turn");
    }
}

#[test]
fn same_choices_play_the_same_game() {
    for seed in [3, 7] {
        let (first, second) = (play(seed), play(seed));
        assert_eq!(first.turns, second.turns, "seed {seed} submissions");
        assert_eq!(first.final_state, second.final_state, "seed {seed} state");
    }
}

#[test]
fn an_empty_choice_list_is_a_valid_game() {
    let run = run_case(&mut ChoiceTape::default(), &FuzzConfig { max_turns: 3 });
    assert!(run.violations.is_empty(), "{:?}", run.violations);
    assert_eq!(run.size(), (1, 0));
}

#[test]
fn written_scenario_replays_to_the_same_state() {
    for seed in [1, 4, 9] {
        let run = play(seed);
        let dir =
            std::env::temp_dir().join(format!("nc-fuzz-replay-{}-{seed}", std::process::id()));
        let path = write_scenario(&run, &dir).expect("scenario should write");
        let spec = CombatScenarioSpec::load_kdl(&path).expect("scenario should parse");
        let replay = run_combat_scenario(&spec).expect("scenario should replay");
        assert_eq!(
            replay.built.game_data, run.final_state,
            "seed {seed} replay diverged"
        );
        std::fs::remove_dir_all(&dir).ok();
    }
}

fn observe(before: &CoreGameData, after: &CoreGameData) -> Vec<Violation> {
    let lifecycle = default_player_lifecycle_states(after.conquest.player_count());
    check_turn(&TurnObservation {
        before,
        after,
        events: &MaintenanceEvents::default(),
        report_rows: &[],
        flagged: after,
        lifecycle: &lifecycle,
    })
}

fn tampered(change: impl FnOnce(&mut CoreGameData)) -> Vec<Violation> {
    let before = build_seeded_initialized_game(2, 3000, 7).expect("game should build");
    let mut after = before.clone();
    change(&mut after);
    observe(&before, &after)
}

#[test]
fn untouched_state_has_no_violations() {
    assert_eq!(tampered(|_| {}), Vec::new());
}

#[test]
fn invariants_catch_tampering() {
    let off_map = tampered(|state| state.fleets[0].set_current_location_coords_raw([0, 5]));
    assert!(
        matches!(off_map.as_slice(), [Violation::CoordinateOutOfMap { .. }]),
        "{off_map:?}"
    );

    let shared = tampered(|state| {
        let coords = state.planets[0].coords_raw();
        state.planets[1].set_coords_raw(coords);
    });
    assert!(
        shared
            .iter()
            .any(|violation| matches!(violation, Violation::PlanetsShareSector { .. })),
        "{shared:?}"
    );

    let lost = tampered(|state| {
        let fleet = &mut state.fleets[2];
        fleet.set_destroyer_count(fleet.destroyer_count() - 1);
    });
    assert!(
        matches!(lost.as_slice(), [Violation::ShipConservation { .. }]),
        "{lost:?}"
    );

    let seized = tampered(|state| state.planets[1].set_owner_empire_slot_raw(1));
    assert!(
        matches!(
            seized.as_slice(),
            [Violation::UnexplainedOwnershipChange {
                planet_record: 2,
                from: 2,
                to: 1
            }]
        ),
        "{seized:?}"
    );
}
