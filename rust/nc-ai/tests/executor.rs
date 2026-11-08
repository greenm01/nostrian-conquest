//! The scripted executor (t154) reads only its own seat, never needs an
//! order dropped, and beats the autopilot and aggressor policies. Its
//! doctrine switches (t159) replay the old behaviour exactly.

mod common;

use common::{scramble_others, seated_game};
use nc_ai::{
    Doctrine, MatchResult, SeatPolicy, Weights, commission_submission, decode_action, plan_turn,
    play_match, seat_masks,
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

/// The classic doctrine runs t154's executor code. Recorded results lock it
/// in: one tuned seat against three baseline seats, and two tuned seats
/// beside an Aggressor and an Autopilot seat, on two held-out maps over 60
/// years.
///
/// At t159 these matched t154's own results exactly. Since t178, every seat
/// learns what its own attacks reveal, which changes these games, since
/// t198 no mixed fleet gets a combat bonus, and since t206 heavies cover
/// destroyers, which finish cripples at half price. Since t194 an attacker
/// learns a world's figures after bombarding it. Since t255 each hull fires
/// on its own, and since t258 batteries share their fire by ship count. The
/// values below are recorded with all six.
#[test]
fn the_classic_doctrine_replays_t154_exactly() {
    let classic = Doctrine {
        land_grab: false,
        strike_weakest: false,
        suppress_first: false,
        scaled_garrisons: false,
        blockader: false,
        allocate: false,
        supply: false,
        defend: false,
        mixed_hulls: false,
        keep_scouts: false,
        watch: false,
        forward_muster: false,
        pickets: false,
        ferry: false,
        blitz: false,
        reinforce: false,
        keep_starbase: false,
        colony_starbases: false,
        emergency_salvage: false,
        growth_tax: false,
        guard_bases: false,
        repair_at_bases: false,
        etac_share: false,
        salvage_etacs: false,
        split_etacs: false,
        retarget_etacs: false,
        sortie: false,
    };
    let tuned = SeatPolicy::Executor(Weights::CLASSIC);
    let baseline = SeatPolicy::Executor(Weights {
        doctrine: classic,
        ..Weights::BASELINE
    });
    let mixed = [tuned, SeatPolicy::Aggressor, tuned, SeatPolicy::Autopilot];
    let recorded = |production: [u32; 4], worlds: [u32; 4]| MatchResult {
        years: 60,
        winner: None,
        production: production.to_vec(),
        worlds: worlds.to_vec(),
    };
    assert_eq!(
        play_match(100000, &[tuned, baseline, baseline, baseline], 60),
        recorded([618, 102, 111, 275], [9, 3, 3, 5])
    );
    assert_eq!(
        play_match(100001, &mixed, 60),
        recorded([258, 59, 711, 0], [7, 1, 12, 0])
    );
}

#[test]
fn the_land_grab_switch_changes_play() {
    let classic = Weights {
        doctrine: Doctrine {
            land_grab: false,
            ..Doctrine::CURRENT
        },
        ..Weights::TUNED
    };
    let lineup = |weights| [SeatPolicy::Executor(weights); 4];
    assert_ne!(
        play_match(100000, &lineup(Weights::TUNED), 40),
        play_match(100000, &lineup(classic), 40)
    );
}

#[test]
fn the_strike_weakest_switch_changes_play() {
    let by_value = Weights {
        doctrine: Doctrine {
            strike_weakest: false,
            ..Doctrine::CURRENT
        },
        ..Weights::TUNED
    };
    let lineup = |weights| [SeatPolicy::Executor(weights); 4];
    let changed = (100000..100008).any(|map| {
        play_match(map, &lineup(Weights::TUNED), 60) != play_match(map, &lineup(by_value), 60)
    });
    assert!(
        changed,
        "choosing targets by defence should change some game"
    );
}

/// With `suppress_first`, a strike force invades only a world it has spent
/// a year bombarding (it is there, under Bombard orders), or one whose
/// batteries the seat knows are gone.
#[test]
fn suppress_first_invades_only_after_bombarding() {
    let weights = Weights {
        doctrine: Doctrine {
            suppress_first: true,
            ..Doctrine::CURRENT
        },
        ..Weights::TUNED
    };
    let invade = 1 + nc_ai::MISSIONS
        .iter()
        .position(|order| *order == nc_data::Order::InvadeWorld)
        .expect("Invade is offered");
    let mut invasions = 0;
    for map_seed in 0..4 {
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
                for choice in action.fleets.iter().filter(|c| c.mission == invade) {
                    let fleet = &game_data.fleets[choice.record - 1];
                    let bombarded = fleet.order == nc_data::Order::BombardWorld
                        && fleet.sector == game_data.planets[choice.target].sector;
                    let batteries =
                        memory.planet_intel[&(choice.target + 1)].known_ground_batteries;
                    assert!(
                        bombarded || batteries == Some(0),
                        "invasion without bombardment into batteries {batteries:?}"
                    );
                    invasions += 1;
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
        invasions > 0,
        "some bombarding force should go on to invade"
    );
}

#[test]
fn the_executor_orders_only_ground_defences_at_a_defensive_world() {
    let mut game = seated_game(PLAYERS, 0);
    let start_year = game.play.built.game_data.conquest.game_year;
    for planet in &mut game.play.built.game_data.planets {
        planet.defensive = planet.owner.0 == 1;
    }
    for _ in 0..20 {
        for empire in 1..=PLAYERS {
            if let Some(commission) = commission_submission(&game.play.built.game_data, empire) {
                game.submit_turn(&commission).expect("commissioning");
            }
            let game_data = &game.play.built.game_data;
            let memory = &game.seats[usize::from(empire) - 1];
            let action = plan_turn(
                game_data,
                memory,
                &seat_masks(game_data, empire),
                &Weights::TUNED,
                start_year,
            );
            let decoded = decode_action(game_data, empire, &action);
            game.submit_turn(&decoded.submission)
                .expect("executor turn should submit");
        }
        for planet in game.play.built.game_data.planets.iter() {
            if planet.defensive {
                assert!(
                    planet
                        .build_queue
                        .iter()
                        .all(|slot| slot.points_owed == 0 || slot.kind.is_ground_defense()),
                    "{:?}",
                    planet.build_queue
                );
            }
        }
        game.run_maintenance().expect("maintenance should run");
    }
}

/// Plays `years` of four executor seats, seat 1 with `first` and the rest
/// with `rest`, calling `after_year` after each maintenance; returns the
/// salvage orders given.
fn play_executors(
    first: nc_ai::Doctrine,
    rest: nc_ai::Doctrine,
    map_seed: u64,
    years: usize,
    mut after_year: impl FnMut(usize, &nc_data::CoreGameData),
) -> u32 {
    let weights = |empire: u8| Weights {
        doctrine: if empire == 1 { first } else { rest },
        ..Weights::TUNED
    };
    let mut game = seated_game(PLAYERS, map_seed);
    let start_year = game.play.built.game_data.conquest.game_year;
    let mut salvages = 0;
    for year in 0..years {
        for empire in 1..=PLAYERS {
            if let Some(commission) = commission_submission(&game.play.built.game_data, empire) {
                game.submit_turn(&commission).expect("commissioning");
            }
            let game_data = &game.play.built.game_data;
            let memory = &game.seats[usize::from(empire) - 1];
            let action = plan_turn(
                game_data,
                memory,
                &seat_masks(game_data, empire),
                &weights(empire),
                start_year,
            );
            let decoded = decode_action(game_data, empire, &action);
            salvages += decoded
                .submission
                .fleets
                .iter()
                .flat_map(|block| &block.actions)
                .filter(|action| {
                    matches!(action, nc_data::FleetTurnAction::Order { order_code, .. }
                        if *order_code == nc_data::Order::Salvage.code())
                })
                .count() as u32;
            game.submit_turn(&decoded.submission)
                .expect("executor turn should submit");
        }
        game.run_maintenance().expect("maintenance should run");
        after_year(year, &game.play.built.game_data);
    }
    salvages
}

/// ETACs the seats hold, in fleets and stardocks, after `years` of four
/// executor seats with `doctrine`, summed over the years no world was free.
fn idle_etacs(doctrine: nc_ai::Doctrine, map_seed: u64, years: usize) -> (u32, u32) {
    let mut idle = 0;
    let salvages = play_executors(doctrine, doctrine, map_seed, years, |_, game_data| {
        if game_data.planets.iter().all(|planet| planet.owner.0 != 0) {
            idle += game_data
                .fleets
                .iter()
                .filter(|fleet| fleet.owner.0 != 0)
                .map(|fleet| u32::from(fleet.ships.etacs))
                .sum::<u32>()
                + game_data
                    .planets
                    .iter()
                    .map(|planet| planet.stardock.etacs)
                    .sum::<u32>();
        }
    });
    (idle, salvages)
}

#[test]
fn etacs_are_sized_to_the_free_worlds_and_salvaged_once_none_are_left() {
    let on = nc_ai::Doctrine::CURRENT;
    let mut off = on;
    off.etac_share = false;
    off.salvage_etacs = false;
    let (mut before, mut after, mut salvages) = (0, 0, 0);
    for map_seed in 0..4 {
        before += idle_etacs(off, map_seed, 40).0;
        let (idle, salvaged) = idle_etacs(on, map_seed, 40);
        after += idle;
        salvages += salvaged;
    }
    eprintln!("idle ETAC-years: {before} before, {after} after; {salvages} salvage orders");
    assert!(salvages > 0);
    assert!(
        after * 4 < before,
        "{after} idle ETAC-years against {before}"
    );
}

/// Over the first `years`, with seat 1 on `first` and the rest on `rest`:
/// seat 1's fleet-years of colony fleets carrying two or more ETACs, its
/// fleet-years flying to a world another empire owns, and its world-years.
fn colony_race(
    first: nc_ai::Doctrine,
    rest: nc_ai::Doctrine,
    map_seed: u64,
    years: usize,
) -> (u32, u32, u32) {
    let (mut paired, mut wasted, mut worlds) = (0, 0, 0);
    play_executors(first, rest, map_seed, years, |_, game_data| {
        for fleet in game_data
            .fleets
            .iter()
            .filter(|fleet| fleet.owner.0 == 1 && fleet.ships.etacs > 0)
        {
            paired += u32::from(fleet.ships.etacs >= 2);
            let owner = game_data
                .planets
                .iter()
                .find(|planet| planet.sector == fleet.target)
                .map(|planet| planet.owner.0);
            wasted += u32::from(
                fleet.order == nc_data::Order::ColonizeWorld
                    && owner.is_some_and(|owner| owner != 0 && owner != 1),
            );
        }
        worlds += game_data
            .planets
            .iter()
            .filter(|planet| planet.owner.0 == 1)
            .count() as u32;
    });
    (paired, wasted, worlds)
}

#[test]
fn colony_fleets_part_and_turn_back_from_taken_worlds() {
    let on = nc_ai::Doctrine::CURRENT;
    let mut off = on;
    off.split_etacs = false;
    off.retarget_etacs = false;
    let (mut before, mut after) = ((0, 0, 0), (0, 0, 0));
    for map_seed in 0..16 {
        let old = colony_race(off, off, map_seed, 8);
        let new = colony_race(on, off, map_seed, 8);
        before = (before.0 + old.0, before.1 + old.1, before.2 + old.2);
        after = (after.0 + new.0, after.1 + new.1, after.2 + new.2);
    }
    eprintln!(
        "seat 1 (paired colony fleet-years, years flown at taken worlds, world-years): {before:?} -> {after:?}"
    );
    assert!(before.0 > 0 && after.0 == 0, "{after:?} against {before:?}");
    assert!(after.2 > before.2, "{after:?} against {before:?}");
}
