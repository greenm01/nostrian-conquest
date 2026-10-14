//! Seat actions (t152): every choice the masks allow decodes into a turn the
//! game accepts, and the masks reveal nothing about other empires.

mod common;

use common::{scramble_others, seated_game};
use nc_ai::{
    BUILD_AMOUNT_CHOICES, DIPLOMACY_CHOICES, FleetChoice, MISSIONS, PlanetChoice, SPEED_CHOICES,
    SeatAction, SeatMasks, SeatedGame, TAX_CHOICES, commission_submission, decode_action,
    seat_masks,
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

    /// A uniformly chosen allowed index.
    fn allowed(&mut self, mask: &[bool]) -> usize {
        let allowed: Vec<usize> = (0..mask.len()).filter(|&idx| mask[idx]).collect();
        allowed[self.below(allowed.len())]
    }
}

fn random_action(masks: &SeatMasks, rng: &mut Rng) -> SeatAction {
    SeatAction {
        tax: rng.below(TAX_CHOICES),
        diplomacy: masks
            .diplomacy
            .iter()
            .map(|mask| {
                // Mostly keep relations, so games are not all-out war.
                if rng.below(8) == 0 {
                    rng.allowed(mask)
                } else {
                    0
                }
            })
            .collect(),
        planets: masks
            .planets
            .iter()
            .map(|mask| PlanetChoice {
                record: mask.record,
                build: [rng.allowed(&mask.build), rng.allowed(&mask.build)],
                amount: [
                    rng.below(BUILD_AMOUNT_CHOICES),
                    rng.below(BUILD_AMOUNT_CHOICES),
                ],
            })
            .collect(),
        fleets: masks
            .fleets
            .iter()
            .map(|mask| {
                let mission = rng.allowed(&mask.mission);
                FleetChoice {
                    record: mask.record,
                    mission,
                    target: rng.allowed(&mask.targets[mission]),
                    speed: rng.below(SPEED_CHOICES),
                    roe: rng.allowed(&mask.roe),
                    load: mask.load && rng.below(2) == 0,
                }
            })
            .collect(),
    }
}

#[derive(Default)]
struct Tally {
    years: usize,
    fleet_orders: usize,
    builds: usize,
    dropped: usize,
}

/// Play `years` of random masked turns; every submission must be accepted
/// and only a second colonizer of one world may be dropped.
fn play_random(game: &mut SeatedGame, years: u16, rng: &mut Rng, tally: &mut Tally) {
    for _ in 0..years {
        for empire in 1..=PLAYERS {
            if let Some(commission) = commission_submission(&game.play.built.game_data, empire) {
                game.submit_turn(&commission)
                    .expect("commissioning should be accepted");
            }
            let game_data = &game.play.built.game_data;
            let masks = seat_masks(game_data, empire);
            assert_eq!(masks.diplomacy.len(), usize::from(PLAYERS));
            assert!(
                masks
                    .diplomacy
                    .iter()
                    .all(|row| row.len() == DIPLOMACY_CHOICES)
            );
            let action = random_action(&masks, rng);
            let decoded = decode_action(game_data, empire, &action);
            for record in &decoded.dropped_fleet_orders {
                let choice = action
                    .fleets
                    .iter()
                    .find(|choice| choice.record == *record)
                    .expect("a dropped fleet was chosen for");
                assert_eq!(
                    MISSIONS[choice.mission - 1],
                    Order::ColonizeWorld,
                    "only a second colonizer may be dropped"
                );
            }
            tally.dropped += decoded.dropped_fleet_orders.len();
            tally.fleet_orders += action.fleets.iter().filter(|c| c.mission > 0).count();
            tally.builds += action
                .planets
                .iter()
                .flat_map(|c| c.build)
                .filter(|&build| build > 0)
                .count();
            game.submit_turn(&decoded.submission).unwrap_or_else(|err| {
                panic!(
                    "year {}, empire {empire}: {err}\n{:?}",
                    game.play.built.game_data.conquest.game_year, decoded.submission
                )
            });
        }
        game.run_maintenance().expect("maintenance should run");
        tally.years += 1;
        if game.play.winner_state.winner_empire.is_some() {
            break;
        }
    }
}

fn play_games(games: u64, years: u16) -> Tally {
    let mut tally = Tally::default();
    for map_seed in 0..games {
        let mut game = seated_game(PLAYERS, map_seed);
        play_random(&mut game, years, &mut Rng(map_seed), &mut tally);
    }
    assert!(tally.fleet_orders > 0 && tally.builds > 0);
    tally
}

#[test]
fn random_masked_turns_are_always_accepted() {
    play_games(40, 30);
}

/// The t152 exit: 1,000 seeded games with no rejected order. Run with
/// `cargo test --release -p nc-ai --test action -- --ignored --nocapture`.
#[test]
#[ignore]
fn a_thousand_random_masked_games_are_always_accepted() {
    let tally = play_games(1000, 30);
    println!(
        "{} game-years, {} fleet orders, {} builds, {} second colonizers dropped",
        tally.years, tally.fleet_orders, tally.builds, tally.dropped
    );
}

#[test]
fn masks_reveal_nothing_about_other_empires() {
    for map_seed in 0..3 {
        let mut game = seated_game(PLAYERS, map_seed);
        let mut rng = Rng(map_seed);
        for _ in 0..20 {
            for empire in 1..=PLAYERS {
                let game_data = &game.play.built.game_data;
                assert_eq!(
                    seat_masks(&scramble_others(game_data, empire), empire),
                    seat_masks(game_data, empire),
                    "map {map_seed}, year {}, empire {empire}",
                    game_data.conquest.game_year
                );
            }
            play_random(&mut game, 1, &mut rng, &mut Tally::default());
        }
    }
}
