//! Reinforcing strike forces under way by Join (t207).

mod common;

use common::seated_game;
use nc_ai::{Doctrine, Weights, commission_submission, decode_action, plan_turn, seat_masks};
use nc_data::Order;

const PLAYERS: u8 = 4;

/// With `reinforce`, warships join only the seat's own strike forces under
/// way, and the game accepts every join.
#[test]
fn reinforcements_join_strike_forces_under_way() {
    let weights = Weights {
        doctrine: Doctrine {
            reinforce: true,
            ..Doctrine::CURRENT
        },
        ..Weights::TUNED
    };
    let mut joins = 0;
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
                for turn in &decoded.submission.fleets {
                    for order in &turn.actions {
                        if let nc_data::FleetTurnAction::Join {
                            host_fleet_record_index_1_based: host,
                        } = order
                        {
                            let host = &game_data.fleets[host - 1];
                            assert_eq!(host.owner.0, empire);
                            assert!(matches!(
                                host.order,
                                Order::BombardWorld | Order::InvadeWorld | Order::BlitzWorld
                            ));
                            joins += 1;
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
    assert!(joins > 0, "some strike force should be reinforced");
}
