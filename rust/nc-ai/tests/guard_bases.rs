//! Guarding bases by Guard a Starbase: threatened worlds' bases, and bases
//! that mend crippled ships a turn sooner (t207).

mod common;

use common::seated_game;
use nc_ai::{Doctrine, Weights, commission_submission, decode_action, plan_turn, seat_masks};
use nc_data::Order;

const PLAYERS: u8 = 4;

/// Play two 60-year games with `doctrine`; count each year's fleets on
/// Guard a Starbase that `counted` accepts. Every guard is at the guard's
/// own world and the game accepts every order.
fn guards(doctrine: Doctrine, counted: impl Fn(&nc_data::Fleet) -> bool) -> u32 {
    let weights = Weights {
        doctrine,
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
                let decoded = decode_action(game_data, empire, &action);
                assert!(decoded.dropped_fleet_orders.is_empty());
                game.submit_turn(&decoded.submission)
                    .expect("executor turn should submit");
            }
            let game_data = &game.play.built.game_data;
            for fleet in &game_data.fleets {
                if fleet.order == Order::GuardStarbase {
                    let world = game_data
                        .planets
                        .iter()
                        .find(|planet| planet.sector == fleet.target)
                        .expect("a guarded base orbits a world");
                    assert_eq!(world.owner, fleet.owner);
                    guards += u32::from(counted(fleet));
                }
            }
            game.run_maintenance().expect("maintenance should run");
            if game.play.winner_state.winner_empire.is_some() {
                break;
            }
        }
    }
    guards
}

#[test]
fn warships_guard_their_own_bases() {
    let doctrine = Doctrine {
        guard_bases: true,
        ..Doctrine::CURRENT
    };
    assert!(
        guards(doctrine, |_| true) > 0,
        "some base should be guarded"
    );
}

#[test]
fn crippled_fleets_mend_beside_their_own_bases() {
    let doctrine = Doctrine {
        repair_at_bases: true,
        ..Doctrine::CURRENT
    };
    let mending = guards(doctrine, |fleet| !fleet.crippled.is_empty());
    assert!(mending > 0, "some crippled fleet should mend at a base");
}
