//! The sortie doctrine (t263): a strike fleet the network leaves parked on
//! Hold or Rendezvous takes the executor's attack when there is one.

mod common;

use common::seated_game;
use nc_ai::{
    Doctrine, Families, MISSIONS, Weights, commission_submission, decode_action, hand_to_network,
    plan_turn, seat_masks,
};
use nc_data::Order;

const PLAYERS: u8 = 4;

/// Executor seats play; each turn a network that keeps every Hold or
/// Rendezvous fleet parked is handed the strike fleets. With sortie on, the
/// executor's attacks from parked fleets stand; with it off, the network's
/// orders replace them.
#[test]
fn a_parked_strike_fleet_takes_the_executors_attack() {
    let weights = |sortie| Weights {
        doctrine: Doctrine {
            sortie,
            ..Doctrine::CURRENT
        },
        ..Weights::TUNED
    };
    let strike = Families {
        strike: true,
        ..Families::NONE
    };
    let mut sorties = 0;
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
                let action = plan_turn(game_data, memory, &masks, &weights(true), start_year);
                let parked: Vec<_> = action
                    .fleets
                    .iter()
                    .map(|choice| {
                        let order = game_data.fleets[choice.record - 1].order;
                        let attack = choice.mission > 0
                            && matches!(
                                MISSIONS[choice.mission - 1],
                                Order::BombardWorld | Order::InvadeWorld | Order::BlitzWorld
                            );
                        if attack && matches!(order, Order::HoldPosition | Order::RendezvousSector)
                        {
                            nc_ai::FleetChoice {
                                mission: 0,
                                join: 0,
                                ..*choice
                            }
                        } else {
                            *choice
                        }
                    })
                    .collect();
                sorties += parked
                    .iter()
                    .zip(&action.fleets)
                    .filter(|(a, b)| a != b)
                    .count();
                for (sortie, expected) in [(true, &action.fleets), (false, &parked)] {
                    let mut handed = action.clone();
                    hand_to_network(
                        game_data,
                        memory,
                        &weights(sortie),
                        strike,
                        &mut handed,
                        &parked,
                        &[],
                    );
                    // Only strike fleets are handed over, so compare those.
                    for ((got, want), original) in handed
                        .fleets
                        .iter()
                        .zip(expected.iter())
                        .zip(&action.fleets)
                    {
                        let family = nc_ai::fleet_family(&game_data.fleets[got.record - 1]);
                        if family == nc_ai::Family::Strike {
                            assert_eq!(got, want, "sortie {sortie}");
                        } else {
                            assert_eq!(got, original);
                        }
                    }
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
        sorties > 0,
        "some parked fleet should have an attack to take"
    );
}
