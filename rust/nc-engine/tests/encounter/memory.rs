//! Local sighting memory and surprise (combat decision 6; ADR atdsvb5y):
//! whole maintenance years over many seeds. The first exchange's +1 itself
//! is checked with scripted dice in `fleet_battle_tests.rs`; here the
//! memory that decides it is.

use nc_data::maintenance_types::FleetBattlePerspective;
use nc_data::{
    Coords, CoreGameData, DiplomacyOverride, EncounterDispositionEvent, EncounterDispositionReason,
    MaintenanceEvents, Order, has_any_force,
};
use nc_engine::run_maintenance_turn_with_context_and_seed;

use super::support::{
    Fleet, NO_SHIPS, Ships, add_starbase, attacks, board, ca, declared, first_contact, put, years,
};

const SEEDS: std::ops::Range<u64> = 0..300;
const MANY: std::ops::Range<u64> = 0..4000;
const A: [u8; 2] = [9, 10];

fn here() -> Coords {
    Coords::from_array(A)
}

fn enemies() -> Vec<DiplomacyOverride> {
    declared(&[(1, 2), (2, 1)])
}

/// One year of `game_data` under `seed`.
fn run(
    game_data: &mut CoreGameData,
    seed: u64,
    diplomacy: &[DiplomacyOverride],
) -> MaintenanceEvents {
    run_maintenance_turn_with_context_and_seed(game_data, seed, &[], diplomacy)
        .expect("maintenance should succeed")
}

/// Empire 1's three patrolling cruisers (ROE 10), which will find and attack
/// empire 2's `defender` (a single cruiser, ROE 0) stopped with them in deep
/// space at `A`.
fn ambush(defender: Order) -> CoreGameData {
    let mut scenario = board();
    put(
        &mut scenario,
        0,
        Fleet::on_station(1, A, false, Order::PatrolSector, ca(3)).roe(10),
    );
    put(
        &mut scenario,
        4,
        Fleet::on_station(2, A, false, defender, ca(1)),
    );
    scenario
}

/// Every battle empire `attacker` started at `A` this year, as (tick,
/// whether it struck by surprise), in order.
fn attacks_by(events: &MaintenanceEvents, attacker: u8) -> Vec<(u8, bool)> {
    let mut battles: Vec<_> = events
        .fleet_battle_events
        .iter()
        .filter(|event| {
            event.reporting_empire == attacker
                && event.coords == A
                && event.perspective == FleetBattlePerspective::Intercepted
        })
        .filter_map(|event| Some((event.stardate_week?, event.surprise)))
        .collect();
    battles.sort_unstable();
    battles
}

/// Whether `empire` has a fleet with ships at `A` in `game_data`.
fn present(game_data: &CoreGameData, empire: u8) -> bool {
    game_data
        .fleets
        .iter()
        .any(|fleet| fleet.owner.0 == empire && fleet.sector == here() && has_any_force(fleet))
}

/// The chance a stationary target of `ca` cruisers goes unsighted by
/// `ticks` scans of one observer at sensor `factor` in deep space, as
/// `unseen_probability` in `tools/combat-model/surprise_memory_review.py`.
/// `shown` is the target's order multiplier, which the control's Hold
/// targets leave at 1 and a patrol doubles.
fn unseen(ticks: i32, factor: f64, shown: f64, ca: f64) -> f64 {
    let visibility = (30.0 * ca).sqrt() * shown / 52.0;
    (1.0 - (visibility * factor / 100.0).min(1.0)).powi(ticks)
}

/// A sighting by any of an empire's units puts the opponent in its memory
/// there, and nothing else does; the memory outlives the year while the
/// empire stays.
#[test]
fn a_sighting_is_remembered_and_kept_across_years() {
    let mut scenario = board();
    put(
        &mut scenario,
        0,
        Fleet::on_station(1, A, false, Order::HoldPosition, ca(1)),
    );
    put(
        &mut scenario,
        4,
        Fleet::on_station(2, A, false, Order::PatrolSector, ca(1)),
    );
    let (mut sighted_by_2, mut sighted_by_1) = (0, 0);
    for seed in SEEDS {
        let mut game_data = scenario.clone();
        let events = run(&mut game_data, seed, &declared(&[]));
        for (viewer, opponent, count) in [(1, 2, &mut sighted_by_1), (2, 1, &mut sighted_by_2)] {
            let contact = first_contact(&events, viewer, A).is_some();
            assert_eq!(
                game_data.sightings.knows(viewer, here(), 0, opponent),
                contact,
                "seed {seed}: empire {viewer}'s memory follows its contact"
            );
            *count += usize::from(contact);
        }
        let remembered: Vec<_> = game_data.sightings.iter().collect();
        assert!(
            remembered
                .iter()
                .all(|s| s.sector == here() && s.layer == 0 && s.opponents.count_ones() == 1),
            "seed {seed}: nothing else is remembered: {remembered:?}"
        );

        // Nobody leaves, so the next two years keep it all, whatever they see.
        for later in 1..=2 {
            run(&mut game_data, seed + 1000 * later, &declared(&[]));
            for sighting in &remembered {
                assert!(
                    game_data.sightings.knows(
                        sighting.viewer,
                        sighting.sector,
                        sighting.layer,
                        sighting.opponents.trailing_zeros() as u8 + 1
                    ),
                    "seed {seed}: kept into year {later}: {sighting:?}"
                );
            }
        }
    }
    assert!(
        sighted_by_1 > 0 && sighted_by_2 > sighted_by_1,
        "the patrol sights the holder more often: {sighted_by_2} against {sighted_by_1}"
    );
}

/// A battle puts each side in the other's memory: the survivors of both
/// remember, and an empire with no fleet left there remembers nothing.
#[test]
fn a_combat_is_remembered_by_both_sides() {
    let mut scenario = board();
    put(
        &mut scenario,
        0,
        Fleet::on_station(1, A, false, Order::PatrolSector, ca(2)).roe(10),
    );
    put(
        &mut scenario,
        4,
        Fleet::on_station(2, A, false, Order::PatrolSector, ca(2)).roe(10),
    );
    let mut remembered_after_battle = 0;
    for (game_data, events) in years(&scenario, &enemies(), SEEDS) {
        let fought = attacks(&events, 1, A) + attacks(&events, 2, A) > 0;
        for (viewer, opponent) in [(1, 2), (2, 1)] {
            let remembers = game_data.sightings.knows(viewer, here(), 0, opponent);
            if fought && present(&game_data, viewer) {
                assert!(
                    remembers,
                    "empire {viewer} fought empire {opponent} and stayed"
                );
                remembered_after_battle += 1;
            }
            if !present(&game_data, viewer) {
                assert!(
                    !game_data.sightings.holds(viewer, here(), 0),
                    "empire {viewer} left or died"
                );
            }
        }
    }
    assert!(remembered_after_battle >= 100, "{remembered_after_battle}");
}

/// The attack is a surprise exactly when the defending empire had not
/// sighted the attacker, even by a unit that only watches (a starbase never
/// attacks): its first contact before the attack makes the memory. The
/// share of unsighted defenders follows the sighting formula the Python
/// control uses.
#[test]
fn surprise_needs_a_defender_that_has_not_sighted_the_attacker() {
    // Empire 1's five blockading cruisers (sensor factor 5) find empire 2's
    // starbase (visibility 90/52) in deep space; the base, at factor 30,
    // scans their 1.1-times-visible stack.
    let mut scenario = board();
    put(
        &mut scenario,
        0,
        Fleet::on_station(1, A, false, Order::GuardBlockadeWorld, ca(5)).roe(10),
    );
    add_starbase(&mut scenario, 2, 1, A, false);
    let (mut attacked, mut sighted_first, mut unsighted) = (0, 0usize, 0usize);
    // The base rolls once a tick, so a battle at tick t follows t - 1 rolls
    // of it for certain and the tick-t roll when its turn came first.
    let (mut at_least, mut at_most, mut variance) = (0.0, 0.0, 0.0);
    for (game_data, events) in years(&scenario, &enemies(), MANY) {
        let Some(&(tick, surprise)) = attacks_by(&events, 1).first() else {
            continue;
        };
        attacked += 1;
        let contact = first_contact(&events, 2, A);
        if surprise {
            assert!(
                contact.is_none_or(|seen| seen >= tick),
                "surprised at tick {tick} after contact at {contact:?}"
            );
            unsighted += 1;
        } else {
            assert!(
                contact.is_some_and(|seen| seen <= tick),
                "not surprised at tick {tick}, contact at {contact:?}"
            );
            sighted_first += 1;
        }
        // The attacker remembers the base either way.
        if present(&game_data, 1) {
            assert!(game_data.sightings.knows(1, here(), 0, 2));
        }
        let shown = 1.1;
        let low = 1.0 - unseen(i32::from(tick) - 1, 30.0, shown, 5.0);
        let high = 1.0 - unseen(i32::from(tick), 30.0, shown, 5.0);
        at_least += low;
        at_most += high;
        variance += high * (1.0 - high);
    }
    eprintln!(
        "{attacked} attacks: {sighted_first} after the base's own sighting, \
         expected between {at_least:.1} and {at_most:.1}"
    );
    assert!(attacked >= 500 && unsighted > 100 && sighted_first > 100);
    let slack = 4.5 * variance.sqrt() + 1.0;
    assert!(
        sighted_first as f64 >= at_least - slack && sighted_first as f64 <= at_most + slack,
        "{sighted_first} of {attacked} attacks followed the base's sighting, \
         expected between {at_least:.1} and {at_most:.1} (slack {slack:.1})"
    );
}

/// A defender that hardly looks, a Hold fleet at sensor factor 1, is almost
/// never the one to sight first: nearly every attack on it is a surprise,
/// as the formula says, and the few that are not follow its own sighting.
#[test]
fn a_holding_defender_is_surprised_nearly_always() {
    let scenario = ambush(Order::HoldPosition);
    let (mut attacked, mut sighted_first) = (0, 0);
    let (mut expected, mut variance) = (0.0, 0.0);
    for (_, events) in years(&scenario, &enemies(), MANY) {
        let Some(&(tick, surprise)) = attacks_by(&events, 1).first() else {
            continue;
        };
        attacked += 1;
        let contact = first_contact(&events, 2, A);
        if surprise {
            assert!(
                contact.is_none_or(|seen| seen >= tick),
                "{tick}, {contact:?}"
            );
        } else {
            assert!(
                contact.is_some_and(|seen| seen <= tick),
                "{tick}, {contact:?}"
            );
        }
        sighted_first += usize::from(!surprise);
        let sighted = 1.0 - unseen(i32::from(tick) - 1, 1.0, 2.0, 3.0);
        expected += sighted;
        variance += sighted * (1.0 - sighted);
    }
    // Contacts at the attack's own tick may or may not come first; the
    // formula bound is the tick before, and the count may exceed it only by
    // those ties.
    assert!(attacked >= 500);
    assert!(
        (sighted_first as f64) < expected + 4.5 * variance.sqrt() + 0.02 * attacked as f64 + 1.0,
        "{sighted_first} of {attacked} attacks followed the defender's sighting, expected about {expected:.1}"
    );
    assert!(
        sighted_first * 10 < attacked,
        "{sighted_first} of {attacked}"
    );
}

/// A weak fleet that sights a stronger one and declines the fight by its
/// ROE has still sighted it: the stronger fleet's later attack on it is not
/// a surprise.
#[test]
fn a_sighting_whose_trigger_was_cancelled_still_counts() {
    let mut scenario = board();
    // Empire 1's lone destroyer (ROE 6, closing on what it sees) declines
    // empire 2's five cruisers; they patrol the same sector and attack it.
    put(
        &mut scenario,
        0,
        Fleet::on_station(
            1,
            A,
            false,
            Order::HoldPosition,
            Ships { dd: 1, ..NO_SHIPS },
        )
        .roe(6),
    );
    put(
        &mut scenario,
        4,
        Fleet::on_station(2, A, false, Order::PatrolSector, ca(5)).roe(10),
    );
    let (mut declined, mut after_declining, mut surprised) = (0, 0, 0);
    for (_, events) in years(&scenario, &enemies(), MANY) {
        let Some(&(tick, surprise)) = attacks_by(&events, 2).first() else {
            continue;
        };
        let declined_at = events
            .encounter_disposition_events
            .iter()
            .filter_map(|event| match *event {
                EncounterDispositionEvent::NoEngagement {
                    owner_empire: 1,
                    reason: EncounterDispositionReason::RoeDeclined,
                    stardate_week: Some(week),
                    ..
                } => Some(week),
                _ => None,
            })
            .min();
        declined += usize::from(declined_at.is_some());
        match (surprise, declined_at) {
            (false, Some(week)) => {
                assert!(week <= tick, "declined at {week}, attacked at {tick}");
                after_declining += 1;
            }
            (true, declined_at) => {
                assert!(
                    declined_at.is_none_or(|week| week >= tick),
                    "surprised at {tick} after declining at {declined_at:?}"
                );
                surprised += 1;
            }
            (false, None) => panic!("not a surprise at {tick}, but empire 1 never sighted"),
        }
    }
    assert!(
        declined > 0 && after_declining > 0 && surprised > 0,
        "{declined} declinations, {after_declining} attacks after one, {surprised} surprises"
    );
}

/// Memory that survived from an earlier year decides the same way: a
/// defender that remembers the attacker is never surprised, and memory of
/// other empires, or the attacker's own memory, changes nothing.
#[test]
fn memory_from_an_earlier_year_removes_the_surprise() {
    let scenario = ambush(Order::HoldPosition);
    let remembering = |viewer: u8, opponent: u8| {
        let mut seeded = scenario.clone();
        seeded.sightings.note(viewer, here(), 0, opponent);
        seeded
    };
    let aware = remembering(2, 1);
    let other_empire = remembering(2, 3);
    let the_attackers = remembering(1, 2);
    let (mut surprises, mut compared, mut later) = (0, 0, 0);
    for seed in MANY {
        let year = |scenario: &CoreGameData| {
            let mut game_data = scenario.clone();
            let events = run(&mut game_data, seed, &enemies());
            attacks_by(&events, 1)
        };
        let plain = year(&scenario);
        let (aware, other, theirs) = (year(&aware), year(&other_empire), year(&the_attackers));
        // The same dice reach the same first attack; with the defender's
        // memory it is no surprise, and the battles after it differ.
        assert_eq!(
            plain.first().map(|&(tick, _)| tick),
            aware.first().map(|&(tick, _)| tick),
            "seed {seed}"
        );
        assert!(aware.iter().all(|&(_, surprise)| !surprise), "seed {seed}");
        // A battle remembers itself: none after the first is a surprise.
        for battles in [&plain, &aware] {
            later += battles.len().saturating_sub(1);
            assert!(
                battles.iter().skip(1).all(|&(_, surprise)| !surprise),
                "seed {seed}: {battles:?}"
            );
        }
        assert_eq!(plain, other, "seed {seed}: memory of another empire");
        assert_eq!(plain, theirs, "seed {seed}: the attacker's own memory");
        surprises += plain.iter().filter(|&&(_, surprise)| surprise).count();
        compared += plain.len();
    }
    assert!(
        surprises > 100 && compared >= surprises,
        "{surprises} of {compared}"
    );
    assert!(later > 20, "{later} later battles");
}

/// The memory lasts while the empire has a unit in the sector and layer,
/// through any number of years, whether or not the opponent is there; an
/// opponent that comes back finds a defender that still remembers it.
#[test]
fn memory_lasts_while_the_empire_stays_whatever_the_opponent_does() {
    let mut scenario = board();
    put(
        &mut scenario,
        0,
        Fleet::on_station(1, A, false, Order::HoldPosition, ca(1)),
    );
    // Empire 2's cruiser is not there at all.
    scenario.sightings.note(1, here(), 0, 2);
    let mut surprises_without_memory = 0;
    for seed in 0..100 {
        let mut game_data = scenario.clone();
        for year in 0..3 {
            run(&mut game_data, seed + 10 * year, &declared(&[]));
            assert!(
                game_data.sightings.knows(1, here(), 0, 2),
                "seed {seed}, year {year}: the holder still remembers empire 2"
            );
        }
        assert!(!present(&game_data, 2), "empire 2 never came");

        // Empire 2 arrives in strength, at ROE 10, and finds the holder; it
        // is no surprise in any year, though it is where memory is absent.
        for (mut returned, remembers) in [(game_data.clone(), true), (scenario.clone(), false)] {
            if !remembers {
                returned.sightings.forget(1, here(), 0);
            }
            put(
                &mut returned,
                4,
                Fleet::on_station(2, A, false, Order::PatrolSector, ca(3)).roe(10),
            );
            for year in 0..4 {
                let events = run(&mut returned, 1000 + seed + 10 * year, &enemies());
                for (_, surprise) in attacks_by(&events, 2) {
                    assert!(
                        !(remembers && surprise),
                        "seed {seed}: surprised a defender that remembered"
                    );
                    surprises_without_memory += usize::from(surprise);
                }
            }
        }
    }
    assert!(surprises_without_memory > 0);
}

/// Memory ends with the empire's last unit there, whether it leaves or is
/// destroyed.
#[test]
fn memory_ends_when_the_empires_presence_does() {
    // It sets off for deep space and is gone from the sector by year's end.
    let mut scenario = board();
    put(
        &mut scenario,
        0,
        Fleet::on_station(1, A, false, Order::HoldPosition, ca(1)).heading(
            Order::MoveOnly,
            [9, 4],
            false,
            3,
        ),
    );
    scenario.sightings.note(1, here(), 0, 2);
    scenario.sightings.note(2, here(), 0, 1);
    let mut game_data = scenario;
    run(&mut game_data, 1, &declared(&[]));
    assert!(!present(&game_data, 1));
    assert!(!game_data.sightings.holds(1, here(), 0), "it left");
    assert!(
        !game_data.sightings.holds(2, here(), 0),
        "empire 2 has no unit there to remember"
    );

    // It is destroyed where it stands: five cruisers against one destroyer.
    let mut scenario = board();
    put(
        &mut scenario,
        0,
        Fleet::on_station(
            1,
            A,
            false,
            Order::HoldPosition,
            Ships { dd: 1, ..NO_SHIPS },
        ),
    );
    put(
        &mut scenario,
        4,
        Fleet::on_station(2, A, false, Order::PatrolSector, ca(5)).roe(10),
    );
    scenario.sightings.note(1, here(), 0, 2);
    let mut destroyed = 0;
    for (game_data, _) in years(&scenario, &enemies(), SEEDS) {
        if !present(&game_data, 1) {
            destroyed += 1;
            assert!(!game_data.sightings.holds(1, here(), 0), "it died");
        } else {
            assert!(game_data.sightings.knows(1, here(), 0, 2));
        }
    }
    assert!(destroyed > 20, "{destroyed} years destroyed the destroyer");
}

/// A gap in presence clears the memory even when the empire has a unit
/// there again before the year ends: a fleet leaving at tick 9 and another
/// arriving at tick 30 share nothing, but one arriving at tick 13 while the
/// first lingers (it leaves at tick 26) keeps it.
#[test]
fn a_gap_in_presence_clears_the_memory_within_a_year() {
    let relay = |leaver_speed: u8, arriver_from: [u8; 2]| {
        let mut scenario = board();
        put(
            &mut scenario,
            0,
            Fleet::on_station(1, A, false, Order::HoldPosition, ca(1))
                .heading(Order::MoveOnly, [9, 4], false, leaver_speed)
                .max_speed(leaver_speed),
        );
        put(
            &mut scenario,
            1,
            Fleet::on_station(1, arriver_from, false, Order::HoldPosition, ca(1))
                .heading(Order::MoveOnly, A, false, 6)
                .max_speed(6),
        );
        scenario.sightings.note(1, here(), 0, 2);
        let mut game_data = scenario;
        run(&mut game_data, 1, &declared(&[]));
        game_data
    };

    let gap = relay(3, [9, 14]);
    assert_eq!(gap.fleets[1].sector, here(), "the second fleet arrived");
    assert_ne!(gap.fleets[0].sector, here(), "the first had left");
    assert!(
        !gap.sightings.holds(1, here(), 0),
        "nobody was there between"
    );

    let overlap = relay(1, [9, 12]);
    assert_eq!(overlap.fleets[1].sector, here(), "the second fleet arrived");
    assert_ne!(overlap.fleets[0].sector, here(), "the first had left");
    assert!(
        overlap.sightings.knows(1, here(), 0, 2),
        "somebody was always there"
    );
}
