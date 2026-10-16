//! The scripted executor (t154) reads only its own seat, never needs an
//! order dropped, and beats the autopilot and aggressor policies. Its
//! doctrine switches (t159) replay the old behaviour exactly.

mod common;

use common::{scramble_others, seated_game};
use nc_ai::{
    Doctrine, MatchResult, SeatPolicy, Weights, commission_submission, decode_action, plan_turn,
    play_match, seat_masks,
};

const PLAYERS: u8 = 4;

#[test]
fn the_executor_reads_only_its_own_seat_and_needs_no_dropped_order() {
    for map_seed in 0..3 {
        let mut game = seated_game(PLAYERS, map_seed);
        let start_year = game.play.built.game_data.conquest.game_year;
        for _ in 0..40 {
            for empire in 1..=PLAYERS {
                if let Some(commission) = commission_submission(&game.play.built.game_data, empire)
                {
                    game.submit_turn(&commission).expect("commissioning");
                }
                let game_data = &game.play.built.game_data;
                let memory = &game.seats[usize::from(empire) - 1];
                let masks = seat_masks(game_data, empire);
                let action = plan_turn(game_data, memory, &masks, &Weights::TUNED, start_year);
                let scrambled = scramble_others(game_data, empire);
                assert_eq!(
                    plan_turn(
                        &scrambled,
                        memory,
                        &seat_masks(&scrambled, empire),
                        &Weights::TUNED,
                        start_year
                    ),
                    action,
                    "map {map_seed}, year {}, empire {empire}",
                    game_data.conquest.game_year
                );
                let decoded = decode_action(game_data, empire, &action);
                assert!(decoded.dropped_fleet_orders.is_empty());
                game.submit_turn(&decoded.submission)
                    .expect("executor turn should submit");
            }
            game.run_maintenance().expect("maintenance should run");
            if game.play.winner_state.winner_empire.is_some() {
                break;
            }
        }
    }
}

fn win_rate(opponent: SeatPolicy) -> f32 {
    let games = 8;
    let wins: f32 = (0..games)
        .map(|map| {
            let seat = map as usize % usize::from(PLAYERS);
            let mut policies = [opponent; PLAYERS as usize];
            policies[seat] = SeatPolicy::Executor(Weights::TUNED);
            play_match(map, &policies, 40).win_share(seat as u8 + 1)
        })
        .sum();
    wins / games as f32
}

#[test]
fn the_executor_beats_the_autopilot() {
    assert!(win_rate(SeatPolicy::Autopilot) >= 0.75);
}

#[test]
fn the_executor_beats_the_aggressor() {
    assert!(win_rate(SeatPolicy::Aggressor) >= 0.5);
}

/// The classic doctrine runs t154's executor code. Recorded results lock it
/// in: one tuned seat against three baseline seats, and two tuned seats
/// beside an Aggressor and an Autopilot seat, on two held-out maps over 60
/// years.
///
/// At t159 these matched t154's own results exactly. Since t178, every seat
/// learns what its own attacks reveal, which changes these games. The
/// values below are recorded with that intel.
#[test]
fn the_classic_doctrine_replays_t154_exactly() {
    let classic = Doctrine {
        land_grab: false,
        strike_weakest: false,
        suppress_first: false,
        scaled_garrisons: false,
        blockader: false,
        allocate: false,
        supply: false,
        defend: false,
    };
    let tuned = SeatPolicy::Executor(Weights::CLASSIC);
    let baseline = SeatPolicy::Executor(Weights {
        doctrine: classic,
        ..Weights::BASELINE
    });
    let mixed = [tuned, SeatPolicy::Aggressor, tuned, SeatPolicy::Autopilot];
    let recorded = |production: [u32; 4], worlds: [u32; 4]| MatchResult {
        years: 60,
        winner: None,
        production: production.to_vec(),
        worlds: worlds.to_vec(),
    };
    assert_eq!(
        play_match(100000, &[tuned, baseline, baseline, baseline], 60),
        recorded([527, 181, 264, 151], [7, 5, 4, 4])
    );
    assert_eq!(
        play_match(100001, &mixed, 60),
        recorded([277, 0, 1113, 100], [6, 0, 13, 1])
    );
}

#[test]
fn the_land_grab_switch_changes_play() {
    let classic = Weights {
        doctrine: Doctrine {
            land_grab: false,
            ..Doctrine::CURRENT
        },
        ..Weights::TUNED
    };
    let lineup = |weights| [SeatPolicy::Executor(weights); 4];
    assert_ne!(
        play_match(100000, &lineup(Weights::TUNED), 40),
        play_match(100000, &lineup(classic), 40)
    );
}

#[test]
fn the_strike_weakest_switch_changes_play() {
    let by_value = Weights {
        doctrine: Doctrine {
            strike_weakest: false,
            ..Doctrine::CURRENT
        },
        ..Weights::TUNED
    };
    let lineup = |weights| [SeatPolicy::Executor(weights); 4];
    let changed = (100000..100008).any(|map| {
        play_match(map, &lineup(Weights::TUNED), 60) != play_match(map, &lineup(by_value), 60)
    });
    assert!(
        changed,
        "choosing targets by defence should change some game"
    );
}

/// With `suppress_first`, a strike force invades only a world it has spent
/// a year bombarding (it is there, under Bombard orders), or one whose
/// batteries the seat knows are gone.
#[test]
fn suppress_first_invades_only_after_bombarding() {
    let weights = Weights {
        doctrine: Doctrine {
            suppress_first: true,
            ..Doctrine::CURRENT
        },
        ..Weights::TUNED
    };
    let invade = 1 + nc_ai::MISSIONS
        .iter()
        .position(|order| *order == nc_data::Order::InvadeWorld)
        .expect("Invade is offered");
    let mut invasions = 0;
    for map_seed in 0..4 {
        let mut game = seated_game(PLAYERS, map_seed);
        let start_year = game.play.built.game_data.conquest.game_year;
        for _ in 0..60 {
            for empire in 1..=PLAYERS {
                if let Some(commission) = commission_submission(&game.play.built.game_data, empire)
                {
                    game.submit_turn(&commission).expect("commissioning");
                }
                let game_data = &game.play.built.game_data;
                let memory = &game.seats[usize::from(empire) - 1];
                let masks = seat_masks(game_data, empire);
                let action = plan_turn(game_data, memory, &masks, &weights, start_year);
                for choice in action.fleets.iter().filter(|c| c.mission == invade) {
                    let fleet = &game_data.fleets[choice.record - 1];
                    let bombarded = fleet.order == nc_data::Order::BombardWorld
                        && fleet.sector == game_data.planets[choice.target].sector;
                    let batteries =
                        memory.planet_intel[&(choice.target + 1)].known_ground_batteries;
                    assert!(
                        bombarded || batteries == Some(0),
                        "invasion without bombardment into batteries {batteries:?}"
                    );
                    invasions += 1;
                }
                let decoded = decode_action(game_data, empire, &action);
                game.submit_turn(&decoded.submission)
                    .expect("executor turn should submit");
            }
            game.run_maintenance().expect("maintenance should run");
            if game.play.winner_state.winner_empire.is_some() {
                break;
            }
        }
    }
    assert!(
        invasions > 0,
        "some bombarding force should go on to invade"
    );
}
