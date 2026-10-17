//! Early warning (t203): pickets on the approaches, threats tracked from
//! what the seat has seen, and waiting warships that sail to meet them.

mod common;

use common::{scramble_others, seated_game};
use nc_ai::{
    Contact, ContactSource, Doctrine, MISSIONS, SeatAction, SeatedGame, Weights,
    commission_submission, decode_action, plan_turn, seat_masks,
};
use nc_data::{Order, ProductionItemKind};

const PLAYERS: u8 = 4;

fn with(doctrine: Doctrine) -> Weights {
    Weights {
        doctrine,
        ..Weights::TUNED
    }
}

fn mission(order: Order) -> usize {
    1 + MISSIONS
        .iter()
        .position(|mission| *mission == order)
        .expect("offered")
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

/// With `pickets`, colonies build lone destroyers after the land grab, and
/// the executor still reads only its own seat.
#[test]
fn pickets_are_built_alone_at_colonies() {
    let weights = with(Doctrine {
        pickets: true,
        track: true,
        intercept: true,
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
            pickets += action
                .planets
                .iter()
                .filter(|planet| planet.build == [destroyer, 0] && planet.amount == [0, 0])
                .count();
        });
    }
    assert!(pickets > 0, "some colony should build a picket");
}

/// The seat's colonies, if a fleet of warships waits at a world of its own.
fn colonies(game: &SeatedGame, empire: u8) -> Vec<(usize, [u8; 2])> {
    let game_data = &game.play.built.game_data;
    let home = usize::from(game_data.players[usize::from(empire) - 1].homeworld);
    let waiting = game_data.fleets.iter().any(|fleet| {
        fleet.owner.0 == empire
            && nc_data::has_combat_ships(fleet)
            && matches!(fleet.order, Order::HoldPosition | Order::RendezvousSector)
    });
    if !waiting {
        return Vec::new();
    }
    game_data
        .planets
        .iter()
        .enumerate()
        .filter(|(idx, planet)| planet.owner.0 == empire && idx + 1 != home)
        .map(|(idx, planet)| (idx, planet.sector.to_array()))
        .collect()
}

/// `memory` with a one-destroyer force of `attacker` seen at `coords` this
/// year, and `attacker` recorded as having attacked the seat.
fn sighted(game: &SeatedGame, empire: u8, attacker: u8, coords: [u8; 2]) -> nc_ai::SeatMemory {
    let year = game.play.built.game_data.conquest.game_year;
    let mut memory = game.seats[usize::from(empire) - 1].clone();
    memory.contacts.push(Contact {
        year,
        coords,
        empire: attacker,
        source: ContactSource::Scout,
        small: 1,
        medium: 0,
        large: 0,
        ships: None,
        starbases: 0,
    });
    memory.attackers.insert(attacker, year);
    memory
}

fn guards(action: &SeatAction, idx: usize) -> bool {
    action.fleets.iter().any(|choice| {
        choice.target == idx
            && [Order::GuardStarbase, Order::GuardBlockadeWorld]
                .iter()
                .any(|order| choice.mission == mission(*order))
    })
}

/// With `track` and `intercept`, a force from an empire that attacked the
/// seat, seen beside a colony, draws a waiting fleet to guard it, before
/// the seat has declared war; without `track` it draws nothing.
#[test]
fn a_waiting_fleet_sails_to_meet_an_attacker() {
    let tracking = with(Doctrine {
        track: true,
        intercept: true,
        ..Doctrine::CURRENT
    });
    let blind = with(Doctrine {
        intercept: true,
        ..Doctrine::CURRENT
    });
    let (mut sailed, mut tried) = (0, 0);
    for map_seed in 0..4 {
        play(map_seed, 20, Weights::TUNED, |game, empire, start_year| {
            let Some(&(idx, colony)) = colonies(game, empire).first() else {
                return;
            };
            let attacker = empire % PLAYERS + 1;
            let game_data = &game.play.built.game_data;
            let memory = sighted(game, empire, attacker, colony);
            let masks = seat_masks(game_data, empire);
            tried += 1;
            assert!(
                !guards(
                    &plan_turn(game_data, &memory, &masks, &blind, start_year),
                    idx
                ) || game_data.players[usize::from(empire) - 1].diplomacy
                    [usize::from(attacker) - 1]
                    == nc_data::DiplomaticRelation::Enemy,
                "an undeclared attacker should not count without track"
            );
            if guards(
                &plan_turn(game_data, &memory, &masks, &tracking, start_year),
                idx,
            ) {
                sailed += 1;
            }
        });
    }
    assert!(
        tried > 0 && sailed > 0,
        "{sailed} of {tried} sightings drew a guard"
    );
}

/// With `track`, a force seen leaving its empire's world threatens the
/// seat's world it is heading for, beyond the threat range.
#[test]
fn a_tracked_force_threatens_the_world_ahead_of_it() {
    let tracking = with(Doctrine {
        track: true,
        intercept: true,
        ..Doctrine::CURRENT
    });
    let gap = |a: [u8; 2], b: [u8; 2]| {
        (f32::from(a[0]) - f32::from(b[0])).hypot(f32::from(a[1]) - f32::from(b[1]))
    };
    let mut sailed = 0;
    for map_seed in 0..4 {
        play(map_seed, 30, Weights::TUNED, |game, empire, start_year| {
            let game_data = &game.play.built.game_data;
            let memory = &game.seats[usize::from(empire) - 1];
            let masks = seat_masks(game_data, empire);
            let rivals: Vec<(u8, [u8; 2])> = memory
                .planet_intel
                .iter()
                .filter_map(|(&record, intel)| {
                    let owner = intel.known_owner_empire_id?;
                    (owner != 0 && owner != empire)
                        .then(|| (owner, game_data.planets[record - 1].sector.to_array()))
                })
                .collect();
            for (_, colony) in colonies(game, empire) {
                // A sighting one sector out from the colony's nearest rival
                // world, toward the colony.
                let Some(&(rival, origin)) = rivals
                    .iter()
                    .min_by(|a, b| gap(a.1, colony).total_cmp(&gap(b.1, colony)))
                else {
                    continue;
                };
                let span = gap(origin, colony);
                let seen: [u8; 2] = std::array::from_fn(|axis| {
                    let (from, to) = (f32::from(origin[axis]), f32::from(colony[axis]));
                    (from + (to - from) / span).round() as u8
                });
                let memory = sighted(game, empire, rival, seen);
                let action = plan_turn(game_data, &memory, &masks, &tracking, start_year);
                // Only the heading can threaten a world beyond the range.
                let far = game_data.planets.iter().enumerate().any(|(idx, planet)| {
                    planet.owner.0 == empire
                        && gap(planet.sector.to_array(), seen) > 4.0
                        && guards(&action, idx)
                });
                if far {
                    sailed += 1;
                }
            }
        });
    }
    assert!(
        sailed > 0,
        "a force heading for a world beyond the range should draw a guard"
    );
}
