//! Blitzing worlds whose batteries are gone (t207).

mod common;

use common::seated_game;
use nc_ai::{
    Doctrine, Families, Weights, commission_submission, decode_action, hand_to_network, plan_turn,
    seat_masks,
};
use nc_data::Order;

const PLAYERS: u8 = 4;

/// With `blitz`, the executor lands on worlds by blitz once their batteries
/// are known to be gone, and only then: every blitz target is an enemy
/// world the seat knows has no batteries.
#[test]
fn blitzes_land_only_where_the_batteries_are_known_gone() {
    let weights = Weights {
        doctrine: Doctrine {
            blitz: true,
            ..Doctrine::CURRENT
        },
        ..Weights::TUNED
    };
    let mut blitzes = 0;
    for map_seed in 0..2 {
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
                let decoded = decode_action(game_data, empire, &action);
                for turn in &decoded.submission.fleets {
                    for order in &turn.actions {
                        if let nc_data::FleetTurnAction::Order {
                            order_code,
                            target: Some(target),
                            ..
                        } = order
                            && Order::from_code(*order_code) == Order::BlitzWorld
                        {
                            let world = game_data
                                .planets
                                .iter()
                                .position(|planet| planet.sector.to_array() == *target)
                                .expect("a blitz targets a world");
                            let intel = &memory.planet_intel[&(world + 1)];
                            assert_eq!(intel.known_ground_batteries, Some(0));
                            assert!(
                                intel
                                    .known_owner_empire_id
                                    .is_some_and(|owner| owner != 0 && owner != empire)
                            );
                            blitzes += 1;
                        }
                    }
                }
                game.submit_turn(&decoded.submission)
                    .expect("executor turn should submit");
            }
            game.run_maintenance().expect("maintenance should run");
            if game.play.winner_state.winner_empire.is_some() {
                break;
            }
        }
    }
    assert!(blitzes > 0, "some world should be blitzed");
}

/// A hybrid seat's network invades where the executor would blitz: handing
/// it the executor's own invasions (blitz off) under the blitz doctrine
/// gives back the executor's blitzes, and nothing else changes.
#[test]
fn network_invasions_land_as_the_executor_would() {
    let on = Weights {
        doctrine: Doctrine {
            blitz: true,
            ..Doctrine::CURRENT
        },
        ..Weights::TUNED
    };
    let off = Weights {
        doctrine: Doctrine {
            blitz: false,
            ..Doctrine::CURRENT
        },
        ..Weights::TUNED
    };
    let every = Families {
        colonize: true,
        scout: true,
        strike: true,
        convoy: true,
        builds: false,
    };
    let mut converted = 0;
    for map_seed in 0..2 {
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
                let action = plan_turn(game_data, memory, &masks, &on, start_year);
                let network = plan_turn(game_data, memory, &masks, &off, start_year);
                converted += action
                    .fleets
                    .iter()
                    .zip(&network.fleets)
                    .filter(|(ours, theirs)| ours != theirs)
                    .count();
                let mut handed = action.clone();
                hand_to_network(
                    game_data,
                    memory,
                    &on,
                    every,
                    &mut handed,
                    &network.fleets,
                    &[],
                );
                assert_eq!(handed, action);
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
    assert!(converted > 0, "some invasion should become a blitz");
}
