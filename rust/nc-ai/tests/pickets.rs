//! Pickets on the approaches (t203), built from what a colony has left.

mod common;

use common::{scramble_others, seated_game};
use nc_ai::{
    Doctrine, SeatedGame, Weights, commission_submission, decode_action, plan_turn, seat_masks,
};
use nc_data::ProductionItemKind;

const PLAYERS: u8 = 4;

fn with(doctrine: Doctrine) -> Weights {
    Weights {
        doctrine,
        ..Weights::TUNED
    }
}

/// Play `years` of executor turns on `map_seed`, calling `check` with the
/// game and each empire before its turn is submitted.
fn play(map_seed: u64, years: u16, weights: Weights, mut check: impl FnMut(&SeatedGame, u8, u16)) {
    let mut game = seated_game(PLAYERS, map_seed);
    let start_year = game.play.built.game_data.conquest.game_year;
    for _ in 0..years {
        for empire in 1..=PLAYERS {
            if let Some(commission) = commission_submission(&game.play.built.game_data, empire) {
                game.submit_turn(&commission).expect("commissioning");
            }
            check(&game, empire, start_year);
            let game_data = &game.play.built.game_data;
            let memory = &game.seats[usize::from(empire) - 1];
            let action = plan_turn(
                game_data,
                memory,
                &seat_masks(game_data, empire),
                &weights,
                start_year,
            );
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

/// With `pickets`, colonies spend what is left after their garrison on
/// destroyers for picket duty after the land grab, and the executor still
/// reads only its own seat.
#[test]
fn colonies_build_pickets_from_what_is_left() {
    let weights = with(Doctrine {
        pickets: true,
        ..Doctrine::CURRENT
    });
    let destroyer = 1 + nc_engine::BUILD_UNITS
        .iter()
        .position(|unit| unit.kind == ProductionItemKind::Destroyer)
        .expect("destroyers are built");
    let mut pickets = 0;
    for map_seed in 0..2 {
        play(map_seed, 40, weights, |game, empire, start_year| {
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
            // Colonies build destroyers only as pickets, from what is left.
            let home = usize::from(game_data.players[usize::from(empire) - 1].homeworld);
            pickets += action
                .planets
                .iter()
                .filter(|planet| planet.record != home && planet.build.contains(&destroyer))
                .count();
        });
    }
    assert!(pickets > 0, "some colony should build a picket");
}
