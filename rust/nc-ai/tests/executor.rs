//! The scripted executor (t154) reads only its own seat, never needs an
//! order dropped, and beats the autopilot and aggressor policies.

mod common;

use common::{scramble_others, seated_game};
use nc_ai::{
    SeatPolicy, Weights, commission_submission, decode_action, plan_turn, play_match, seat_masks,
};

const PLAYERS: u8 = 4;

#[test]
fn the_executor_reads_only_its_own_seat_and_needs_no_dropped_order() {
    for map_seed in 0..3 {
        let mut game = seated_game(PLAYERS, map_seed);
        let start_year = game.play.built.game_data.conquest.game_year;
        for _ in 0..40 {
            for empire in 1..=PLAYERS {
                if let Some(commission) = commission_submission(&game.play.built.game_data, empire)
                {
                    game.submit_turn(&commission).expect("commissioning");
                }
                let game_data = &game.play.built.game_data;
                let memory = &game.seats[usize::from(empire) - 1];
                let masks = seat_masks(game_data, empire);
                let action = plan_turn(game_data, memory, &masks, &Weights::TUNED, start_year);
                let scrambled = scramble_others(game_data, empire);
                assert_eq!(
                    plan_turn(
                        &scrambled,
                        memory,
                        &seat_masks(&scrambled, empire),
                        &Weights::TUNED,
                        start_year
                    ),
                    action,
                    "map {map_seed}, year {}, empire {empire}",
                    game_data.conquest.game_year
                );
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
}

fn win_rate(opponent: SeatPolicy) -> f32 {
    let games = 8;
    let wins: f32 = (0..games)
        .map(|map| {
            let seat = map as usize % usize::from(PLAYERS);
            let mut policies = [opponent; PLAYERS as usize];
            policies[seat] = SeatPolicy::Executor(Weights::TUNED);
            play_match(map, &policies, 40).win_share(seat as u8 + 1)
        })
        .sum();
    wins / games as f32
}

#[test]
fn the_executor_beats_the_autopilot() {
    assert!(win_rate(SeatPolicy::Autopilot) >= 0.75);
}

#[test]
fn the_executor_beats_the_aggressor() {
    assert!(win_rate(SeatPolicy::Aggressor) >= 0.5);
}
