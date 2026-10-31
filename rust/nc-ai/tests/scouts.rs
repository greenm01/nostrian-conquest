//! Scouts that commission together split up to search alone.

mod common;

use common::seated_game;
use nc_ai::{
    Detach, Weights, commission_submission, decode_action, detach_choice, plan_turn, seat_masks,
};
use nc_data::{ProductionItemKind, has_combat_ships, stardock_slot_for_kind};
use nc_engine::production_item_kind_code;

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

/// Scouts never ride with a strike force: a world's scouts commission as a
/// fleet of their own, even when warships come out of the same stardock.
#[test]
fn scouts_commission_apart_from_warships() {
    let mut game = seated_game(PLAYERS, 0);
    let game_data = &mut game.play.built.game_data;
    let home = game_data
        .planets
        .iter()
        .position(|planet| planet.owner.0 == 1)
        .expect("empire 1 has a world");
    for (kind, count) in [
        (ProductionItemKind::Cruiser, 3),
        (ProductionItemKind::Scout, 2),
    ] {
        let slot = stardock_slot_for_kind(production_item_kind_code(kind)).expect("a dock slot");
        game_data.planets[home].stardock[slot] = count;
    }
    let before = game_data.fleets.len();

    let commission = commission_submission(game_data, 1).expect("ships wait in dock");
    game.submit_turn(&commission).expect("commissioning");

    let fleets = &game.play.built.game_data.fleets[before..];
    assert_eq!(fleets.len(), 2, "{fleets:?}");
    assert!(
        fleets
            .iter()
            .any(|fleet| fleet.ships.scouts == 2 && !has_combat_ships(fleet))
    );
    assert!(
        fleets
            .iter()
            .any(|fleet| fleet.ships.cruisers == 3 && fleet.ships.scouts == 0)
    );
}
