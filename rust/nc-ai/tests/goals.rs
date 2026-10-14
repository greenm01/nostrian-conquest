//! Strategic goals (t155): the executor's own goals are allowed by the goal
//! masks, the masks reveal nothing about other empires, random goals within
//! the masks always make a turn the game accepts, and the strike and
//! colonize goals steer the fleets.

mod common;

use common::{scramble_others, seated_game};
use nc_ai::{
    Goals, MISSIONS, POSTURES, SeatedGame, TAX_CHOICES, Weights, commission_submission,
    decode_action, execute_goals, goal_masks, plan_goals, seat_masks,
};
use nc_data::Order;

const PLAYERS: u8 = 4;

/// Seeded splitmix64.
struct Rng(u64);

impl Rng {
    fn below(&mut self, n: usize) -> usize {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        ((z ^ (z >> 31)) % n as u64) as usize
    }

    fn allowed(&mut self, mask: &[bool]) -> usize {
        let allowed: Vec<usize> = (0..mask.len()).filter(|&idx| mask[idx]).collect();
        allowed[self.below(allowed.len())]
    }
}

/// Play `years`, every seat choosing goals with `choose`, and check each
/// turn is accepted with no dropped order.
fn play(
    game: &mut SeatedGame,
    years: u16,
    mut choose: impl FnMut(&SeatedGame, u8, &nc_ai::SeatMasks) -> Goals,
) {
    for _ in 0..years {
        for empire in 1..=PLAYERS {
            if let Some(commission) = commission_submission(&game.play.built.game_data, empire) {
                game.submit_turn(&commission).expect("commissioning");
            }
            let game_data = &game.play.built.game_data;
            let masks = seat_masks(game_data, empire);
            let goals = choose(game, empire, &masks);
            let memory = &game.seats[usize::from(empire) - 1];
            let action = execute_goals(game_data, memory, &masks, &Weights::TUNED, &goals);
            let decoded = decode_action(game_data, empire, &action);
            assert!(decoded.dropped_fleet_orders.is_empty());
            game.submit_turn(&decoded.submission)
                .expect("a goal turn should submit");
        }
        game.run_maintenance().expect("maintenance should run");
        if game.play.winner_state.winner_empire.is_some() {
            break;
        }
    }
}

fn allowed(goals: &Goals, masks: &nc_ai::GoalMasks) -> bool {
    goals.tax < TAX_CHOICES
        && goals.posture < POSTURES
        && goals
            .diplomacy
            .iter()
            .zip(&masks.diplomacy)
            .all(|(&choice, row)| row[choice])
        && masks.strike[goals.strike]
        && masks.colonize[goals.colonize]
}

#[test]
fn executor_goals_are_allowed_and_masks_read_only_the_seat() {
    let mut strikes = 0;
    let mut colonies = 0;
    for map_seed in 0..3 {
        let mut game = seated_game(PLAYERS, map_seed);
        let start_year = game.play.built.game_data.conquest.game_year;
        play(&mut game, 50, |game, empire, masks| {
            let game_data = &game.play.built.game_data;
            let memory = &game.seats[usize::from(empire) - 1];
            let goal_mask = goal_masks(game_data, memory, masks);
            let scrambled = scramble_others(game_data, empire);
            assert_eq!(
                goal_masks(&scrambled, memory, &seat_masks(&scrambled, empire)),
                goal_mask,
                "goal masks must not read other empires"
            );
            let goals = plan_goals(game_data, memory, masks, &Weights::TUNED, start_year);
            assert!(allowed(&goals, &goal_mask), "{goals:?}");
            strikes += usize::from(goals.strike > 0);
            colonies += usize::from(goals.colonize > 0);
            goals
        });
    }
    assert!(
        strikes > 0 && colonies > 0,
        "{strikes} strikes, {colonies} colonies"
    );
}

#[test]
fn random_goals_within_the_masks_always_make_a_turn() {
    let mut rng = Rng(17);
    for map_seed in 10..14 {
        let mut game = seated_game(PLAYERS, map_seed);
        play(&mut game, 40, |game, empire, masks| {
            let memory = &game.seats[usize::from(empire) - 1];
            let goal_mask = goal_masks(&game.play.built.game_data, memory, masks);
            Goals {
                tax: rng.below(TAX_CHOICES),
                diplomacy: goal_mask
                    .diplomacy
                    .iter()
                    .map(|row| rng.allowed(row))
                    .collect(),
                posture: rng.below(POSTURES),
                strike: rng.allowed(&goal_mask.strike),
                colonize: rng.allowed(&goal_mask.colonize),
            }
        });
    }
}

#[test]
fn hold_stops_every_strike_and_a_colony_goal_sends_the_first_etac() {
    let strike_missions = [Order::InvadeWorld, Order::BombardWorld];
    let mut steered = 0;
    let mut game = seated_game(PLAYERS, 3);
    let start_year = game.play.built.game_data.conquest.game_year;
    play(&mut game, 50, |game, empire, masks| {
        let game_data = &game.play.built.game_data;
        let memory = &game.seats[usize::from(empire) - 1];
        let goal_mask = goal_masks(game_data, memory, masks);
        let mut goals = plan_goals(game_data, memory, masks, &Weights::TUNED, start_year);

        let held = execute_goals(
            game_data,
            memory,
            masks,
            &Weights::TUNED,
            &Goals {
                strike: 0,
                ..goals.clone()
            },
        );
        assert!(
            held.fleets.iter().all(|choice| choice.mission == 0
                || !strike_missions.contains(&MISSIONS[choice.mission - 1])),
            "hold must stop every strike"
        );

        // Steer the first free ETAC to the last world it may colonize.
        if let Some(world) = goal_mask.colonize.iter().rposition(|&ok| ok)
            && world > 0
        {
            goals.colonize = world;
            let action = execute_goals(game_data, memory, masks, &Weights::TUNED, &goals);
            let sent = action.fleets.iter().any(|choice| {
                choice.mission > 0
                    && MISSIONS[choice.mission - 1] == Order::ColonizeWorld
                    && choice.target == world - 1
            });
            steered += usize::from(sent);
        }
        goals
    });
    assert!(steered > 0, "a colony goal should send an ETAC");
}
