//! Scouts that commission together split up to search alone.

mod common;

use common::seated_game;
use nc_ai::{
    Detach, Weights, commission_submission, decode_action, detach_choice, plan_turn, seat_masks,
};
use nc_data::has_combat_ships;

const PLAYERS: u8 = 4;

#[test]
fn scouts_built_together_split_up() {
    let half = detach_choice(Detach::HalfWarships);
    let mut splits = 0;
    for map_seed in 0..2 {
        let mut game = seated_game(PLAYERS, map_seed);
        let start_year = game.play.built.game_data.conquest.game_year;
        for _ in 0..30 {
            for empire in 1..=PLAYERS {
                if let Some(commission) = commission_submission(&game.play.built.game_data, empire)
                {
                    game.submit_turn(&commission).expect("commissioning");
                }
                let game_data = &game.play.built.game_data;
                let memory = &game.seats[usize::from(empire) - 1];
                let masks = seat_masks(game_data, empire);
                let action = plan_turn(game_data, memory, &masks, &Weights::TUNED, start_year);
                for choice in &action.fleets {
                    let fleet = &game_data.fleets[choice.record - 1];
                    if !has_combat_ships(fleet) && fleet.ships.scouts > 1 && choice.join == 0 {
                        assert_eq!(
                            choice.detach, half,
                            "fleet {} keeps its scouts",
                            choice.record
                        );
                        splits += 1;
                    }
                }
                let decoded = decode_action(game_data, empire, &action);
                assert!(decoded.dropped_fleet_orders.is_empty());
                game.submit_turn(&decoded.submission)
                    .expect("executor turn should submit");
            }
            game.run_maintenance().expect("maintenance should run");
        }
    }
    assert!(splits > 0, "some scouts should commission together");
}
