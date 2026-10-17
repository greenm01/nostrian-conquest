//! Seat actions (t152): every choice the masks allow decodes into a turn the
//! game accepts, and the masks reveal nothing about other empires.

mod common;

use common::{scramble_others, seated_game};
use nc_ai::{
    BUILD_AMOUNT_CHOICES, DIPLOMACY_CHOICES, FleetChoice, MISSIONS, PlanetChoice, SPEED_CHOICES,
    SeatAction, SeatMasks, SeatedGame, TAX_CHOICES, WAYPOINT_CHOICES, commission_submission,
    decode_action, seat_masks, sector_mission, waypoint,
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

/// The sector a sector-mission choice sends its fleet to.
fn decoded_target(game_data: &nc_data::CoreGameData, choice: &FleetChoice) -> [u8; 2] {
    waypoint(
        game_data.fleets[choice.record - 1].sector.to_array(),
        game_data.planets[choice.target].sector.to_array(),
        choice.waypoint,
    )
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
                    waypoint: rng.below(WAYPOINT_CHOICES),
                    speed: rng.below(SPEED_CHOICES),
                    roe: rng.allowed(&mask.roe),
                    load: mask.load && rng.below(2) == 0,
                    detach: if rng.below(8) == 0 {
                        rng.allowed(&mask.detach)
                    } else {
                        0
                    },
                    join: if rng.below(8) == 0 {
                        match rng.allowed(&mask.join) {
                            0 => 0,
                            host => masks.fleets[host - 1].record,
                        }
                    } else {
                        0
                    },
                    station: None,
                    unload: mask.unload && rng.below(2) == 0,
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
    guard_starbase: usize,
    detaches: usize,
    deep_space: usize,
    joins: usize,
    joins_with_detach: usize,
    unloads: usize,
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
            tally.guard_starbase += action
                .fleets
                .iter()
                .filter(|c| c.mission > 0 && MISSIONS[c.mission - 1] == Order::GuardStarbase)
                .count();
            tally.deep_space += action
                .fleets
                .iter()
                .filter(|c| {
                    c.mission > 0
                        && sector_mission(MISSIONS[c.mission - 1])
                        && !game_data
                            .planets
                            .iter()
                            .any(|p| p.sector.to_array() == decoded_target(game_data, c))
                })
                .count();
            tally.detaches += action.fleets.iter().filter(|c| c.detach > 0).count();
            tally.joins += action.fleets.iter().filter(|c| c.join > 0).count();
            tally.joins_with_detach += action
                .fleets
                .iter()
                .filter(|c| c.join > 0 && c.detach > 0)
                .count();
            tally.unloads += action.fleets.iter().filter(|c| c.unload).count();
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

/// Random play includes the join and unload orders (t190), joins with
/// detaches among them, and the game accepts every one.
#[test]
fn random_masked_turns_are_always_accepted() {
    let tally = play_games(40, 30);
    assert!(
        tally.joins > 0 && tally.joins_with_detach > 0 && tally.unloads > 0,
        "{} joins, {} with a detach, {} unloads",
        tally.joins,
        tally.joins_with_detach,
        tally.unloads
    );
}

/// The t152 exit: 1,000 seeded games with no rejected order. Run with
/// `cargo test --release -p nc-ai --test action -- --ignored --nocapture`.
#[test]
#[ignore]
fn a_thousand_random_masked_games_are_always_accepted() {
    let tally = play_games(1000, 30);
    println!(
        "{} game-years, {} fleet orders ({} Guard a Starbase, {} to deep space), \
         {} detaches, {} joins ({} with a detach), {} unloads, {} builds, \
         {} second colonizers dropped",
        tally.years,
        tally.fleet_orders,
        tally.guard_starbase,
        tally.deep_space,
        tally.detaches,
        tally.joins,
        tally.joins_with_detach,
        tally.unloads,
        tally.builds,
        tally.dropped
    );
    assert!(
        tally.guard_starbase > 0,
        "random play should guard a starbase"
    );
    assert!(tally.detaches > 0, "random play should detach");
    assert!(
        tally.deep_space > 0,
        "random play should order fleets into deep space"
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

/// Join and a named station (t189) decode into orders the game accepts.
#[test]
fn a_join_and_a_station_decode_into_legal_orders() {
    let mut game = seated_game(PLAYERS, 9);
    let game_data = &game.play.built.game_data;
    let masks = seat_masks(game_data, 1);
    let warships: Vec<usize> = game_data
        .fleets
        .iter()
        .enumerate()
        .filter(|(_, fleet)| fleet.owner.0 == 1 && nc_data::has_combat_ships(fleet))
        .map(|(idx, _)| idx + 1)
        .collect();
    assert!(
        warships.len() >= 2,
        "the empire starts with two warship fleets"
    );
    let (joiner, host) = (warships[0], warships[1]);
    let patrol = MISSIONS
        .iter()
        .position(|order| *order == Order::PatrolSector)
        .unwrap()
        + 1;
    let station = [3u8, 5u8];
    let fleets = masks
        .fleets
        .iter()
        .map(|mask| FleetChoice {
            record: mask.record,
            mission: if mask.record == host { patrol } else { 0 },
            target: 0,
            waypoint: 0,
            speed: 0,
            roe: 0,
            load: false,
            detach: 0,
            join: if mask.record == joiner { host } else { 0 },
            station: (mask.record == host).then_some(station),
            unload: false,
        })
        .collect();
    let action = SeatAction {
        tax: 12,
        diplomacy: vec![0; masks.diplomacy.len()],
        planets: masks
            .planets
            .iter()
            .map(|mask| PlanetChoice {
                record: mask.record,
                build: [0, 0],
                amount: [0, 0],
            })
            .collect(),
        fleets,
    };
    let decoded = decode_action(game_data, 1, &action);
    assert!(decoded.dropped_fleet_orders.is_empty());
    let actions = |record: usize| -> &Vec<nc_data::FleetTurnAction> {
        &decoded
            .submission
            .fleets
            .iter()
            .find(|block| block.fleet_record_index_1_based == record)
            .expect("the fleet has a block")
            .actions
    };
    assert!(actions(joiner).contains(&nc_data::FleetTurnAction::Join {
        host_fleet_record_index_1_based: host,
    }));
    assert!(actions(host).iter().any(|action| matches!(
        action,
        nc_data::FleetTurnAction::Order { order_code, target: Some(target), .. }
            if *order_code == Order::PatrolSector.code() && *target == station
    )));
    game.submit_turn(&decoded.submission)
        .expect("the game accepts the turn");
}
