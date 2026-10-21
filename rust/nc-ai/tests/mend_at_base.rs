//! Damaged warships mending beside the home starbase (t207).

mod common;

use common::seated_game;
use nc_ai::{
    Doctrine, MISSIONS, Weights, commission_submission, decode_action, plan_turn, seat_masks,
};
use nc_data::Order;

const PLAYERS: u8 = 4;

/// With `keep_starbase` and `mend_at_base`, damaged warships guard the
/// home starbase, and only fleets with crippled ships are sent there.
#[test]
fn damaged_warships_guard_the_home_starbase() {
    let weights = Weights {
        doctrine: Doctrine {
            keep_starbase: true,
            mend_at_base: true,
            ..Doctrine::CURRENT
        },
        ..Weights::TUNED
    };
    let mut guards = 0;
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
                for choice in &action.fleets {
                    if choice.mission > 0 && MISSIONS[choice.mission - 1] == Order::GuardStarbase {
                        let fleet = &game_data.fleets[choice.record - 1];
                        if fleet.order != Order::GuardStarbase {
                            assert!(fleet.crippled.total() > 0);
                            guards += 1;
                        }
                    }
                }
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
    assert!(guards > 0, "some damaged fleet should guard the starbase");
}
