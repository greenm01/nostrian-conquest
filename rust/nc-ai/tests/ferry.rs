//! Ferrying armies between the seat's own worlds (t191).

mod common;

use common::{scramble_others, seated_game};
use nc_ai::{Doctrine, Weights, commission_submission, decode_action, plan_turn, seat_masks};

const PLAYERS: u8 = 4;

/// With `ferry`, transports without warships unload armies only at the
/// seat's own colonies, never at home, and the executor still reads only
/// its own seat.
#[test]
fn ferries_unload_armies_at_the_seats_colonies() {
    let weights = Weights {
        doctrine: Doctrine {
            ferry: true,
            ..Doctrine::CURRENT
        },
        ..Weights::TUNED
    };
    let mut unloads = 0;
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
                let scrambled = scramble_others(game_data, empire);
                assert_eq!(
                    plan_turn(
                        &scrambled,
                        memory,
                        &seat_masks(&scrambled, empire),
                        &weights,
                        start_year
                    ),
                    action
                );
                let home = game_data.players[usize::from(empire) - 1].homeworld;
                for choice in action.fleets.iter().filter(|choice| choice.unload) {
                    let fleet = &game_data.fleets[choice.record - 1];
                    let ships = &fleet.ships;
                    assert_eq!(ships.battleships + ships.cruisers + ships.destroyers, 0);
                    let world = game_data
                        .planets
                        .iter()
                        .position(|planet| planet.sector == fleet.sector)
                        .expect("an unload is at a world");
                    assert_eq!(game_data.planets[world].owner.0, empire);
                    assert_ne!(
                        world + 1,
                        usize::from(home),
                        "ferries do not unload at home"
                    );
                    unloads += 1;
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
    assert!(unloads > 0, "some ferry should unload at a colony");
}
