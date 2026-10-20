//! Blitzing worlds whose batteries are gone (t207).

mod common;

use common::seated_game;
use nc_ai::{Doctrine, Weights, commission_submission, decode_action, plan_turn, seat_masks};
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
