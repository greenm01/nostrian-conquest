//! The campaign seed on `ScenarioPlay` (t150): a cloned start state replays
//! exactly under one seed, and different seeds change how games end.

use nc_data::CoreGameData;
use nc_fuzz::{CaseSetup, ChoiceTape, Policy, plan_turn};
use nc_harness::{ScenarioPlay, build_scenario};

fn start() -> ScenarioPlay {
    let setup = CaseSetup {
        player_count: 2,
        map_seed: 3,
        policies: vec![Policy::Aggressor; 2],
        collapsed_empire: None,
        rally: false,
        war: false,
        armadas: vec![None; 2],
        turns: 100,
    };
    ScenarioPlay::new(build_scenario(&setup.scenario_spec()).expect("setup should build"))
}

/// Aggressor seats with a fixed choice list, so only the campaign seed varies.
fn play(start: &ScenarioPlay, campaign_seed: u64) -> CoreGameData {
    let mut play = start.clone().with_campaign_seed(campaign_seed);
    let mut choices = ChoiceTape::new(
        (0..8192u32)
            .map(|n| n.wrapping_mul(2_654_435_761))
            .collect(),
    );
    for _ in 0..100 {
        for empire in 1..=2 {
            let planned = plan_turn(
                Policy::Aggressor,
                &play.built.game_data,
                empire,
                &mut choices,
            );
            play.submit_turn(&planned.submission)
                .expect("planned turn should submit");
        }
        play.run_maintenance().expect("maintenance should run");
        if play.winner_state.winner_empire.is_some() {
            break;
        }
    }
    play.built.game_data
}

#[test]
fn one_campaign_seed_replays_exactly() {
    let start = start();
    assert_eq!(play(&start, 7), play(&start, 7));
}

#[test]
fn campaign_seeds_change_how_games_end() {
    let start = start();
    let first = play(&start, 0);
    assert!(
        (1..8).any(|seed| play(&start, seed) != first),
        "eight campaign seeds all ended in the same state"
    );
}

#[test]
fn the_default_campaign_seed_is_zero() {
    assert_eq!(start().campaign_seed, 0);
}
