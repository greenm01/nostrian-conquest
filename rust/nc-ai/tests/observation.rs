//! A seat's observation holds only what its empire has learned (t151).
//!
//! The tests play aggressor games and check, every year and for every seat,
//! that changing other empires' records or reports leaves the observation
//! unchanged.

mod common;

use common::{scramble_others, seated_game};
use nc_ai::{
    CONTACT_MEMORY_YEARS, CONTACT_WIDTH, FLEET_WIDTH, GLOBAL_WIDTH, Observation, PLANET_WIDTH,
    STARBASE_WIDTH, SeatedGame, observe,
};
use nc_data::{EncounterDispositionEvent, MaintenanceEvents};
use nc_fuzz::{ChoiceTape, Policy, plan_turn};

const PLAYERS: u8 = 4;
const YEARS: u16 = 40;

fn choices(seed: u64) -> ChoiceTape {
    ChoiceTape::new(
        (0..65_536u32)
            .map(|n| n.wrapping_mul(2_654_435_761) ^ seed as u32)
            .collect(),
    )
}

/// Play aggressor years, calling `check` with the game and the year's
/// events after each maintenance.
fn play_years(map_seed: u64, mut check: impl FnMut(&SeatedGame, &mut MaintenanceEvents)) {
    let mut game = seated_game(PLAYERS, map_seed);
    let mut choices = choices(map_seed);
    for _ in 0..YEARS {
        for empire in 1..=PLAYERS {
            let planned = plan_turn(
                Policy::Aggressor,
                &game.play.built.game_data,
                empire,
                &mut choices,
            );
            game.submit_turn(&planned.submission)
                .expect("planned turn should submit");
        }
        let mut events = game.run_maintenance().expect("maintenance should run");
        check(&game, &mut events);
        if game.play.winner_state.winner_empire.is_some() {
            break;
        }
    }
}

/// Change every contact and attack report addressed to an empire other than
/// `empire`.
fn scramble_reports_to_others(scrambled: &mut MaintenanceEvents, empire: u8) {
    for event in scrambled
        .scout_contact_events
        .iter_mut()
        .filter(|event| event.viewer_empire != empire)
    {
        event.large_vessels += 50;
        event.coords[0] = event.coords[0].wrapping_add(1);
    }
    for event in scrambled
        .fleet_battle_events
        .iter_mut()
        .filter(|event| event.reporting_empire != empire)
    {
        event.enemy_initial.battleships += 50;
    }
    for event in scrambled
        .starbase_sighting_events
        .iter_mut()
        .filter(|event| event.viewer_empire != empire)
    {
        event.coords[1] = event.coords[1].wrapping_add(1);
    }
    for event in &mut scrambled.encounter_disposition_events {
        match event {
            EncounterDispositionEvent::NoEngagement {
                owner_empire,
                large_vessels,
                ..
            } if *owner_empire != empire => *large_vessels += 50,
            EncounterDispositionEvent::Retreated {
                owner_empire,
                enemy_initial,
                ..
            }
            | EncounterDispositionEvent::PursuitFire {
                owner_empire,
                enemy_initial,
                ..
            } if *owner_empire != empire => enemy_initial.battleships += 50,
            _ => {}
        }
    }
    for event in scrambled
        .bombard_events
        .iter_mut()
        .filter(|event| event.attacker_empire != empire)
    {
        event.defender_battery_losses = event.defender_battery_losses.wrapping_add(1);
        event.defender_armies_initial = event.defender_armies_initial.wrapping_add(5);
    }
    for event in scrambled
        .assault_report_events
        .iter_mut()
        .filter(|event| event.attacker_empire != empire)
    {
        event.defender_batteries_initial = event.defender_batteries_initial.wrapping_add(3);
        event.defender_army_losses = event.defender_army_losses.wrapping_add(2);
    }
    scrambled
        .planet_intel_events
        .retain(|event| event.viewer_empire == empire);
}

fn assert_shape(observation: &Observation, planets: usize) {
    assert_eq!(observation.global.len(), GLOBAL_WIDTH);
    assert_eq!(observation.planets.len(), planets * PLANET_WIDTH);
    assert_eq!(observation.fleets.len() % FLEET_WIDTH, 0);
    assert_eq!(observation.contacts.len() % CONTACT_WIDTH, 0);
    assert_eq!(observation.starbases.len() % STARBASE_WIDTH, 0);
}

#[test]
fn other_empires_records_do_not_change_an_observation() {
    for map_seed in 0..3 {
        play_years(map_seed, |game, _| {
            let game_data = &game.play.built.game_data;
            for seat in &game.seats {
                let observation = observe(game_data, seat);
                assert_shape(&observation, game_data.planets.len());
                assert_eq!(
                    observe(&scramble_others(game_data, seat.empire), seat),
                    observation,
                    "map {map_seed}, year {}, empire {}",
                    game_data.conquest.game_year,
                    seat.empire
                );
            }
        });
    }
}

#[test]
fn reports_to_other_empires_teach_a_seat_nothing() {
    for map_seed in 0..3 {
        let mut before: Option<SeatedGame> = None;
        play_years(map_seed, |game, events| {
            if let Some(before) = before.replace(game.clone()) {
                // Every seat must match learning from the real reports.
                // Scrambling changes the events in place, so one seat a
                // year, in rotation, is checked against scrambled reports.
                let game_data = &game.play.built.game_data;
                for seat in &before.seats {
                    let mut honest = seat.clone();
                    honest.learn_year(game_data, events);
                    assert_eq!(&honest, &game.seats[usize::from(seat.empire) - 1]);
                }
                let year = usize::from(game_data.conquest.game_year);
                let seat = &before.seats[year % before.seats.len()];
                let mut fooled = seat.clone();
                scramble_reports_to_others(events, seat.empire);
                fooled.learn_year(game_data, events);
                assert_eq!(
                    &fooled,
                    &game.seats[usize::from(seat.empire) - 1],
                    "map {map_seed}, empire {}",
                    seat.empire
                );
            }
        });
    }
}

#[test]
fn seats_remember_contacts_for_a_limited_time() {
    let mut contacts = 0;
    play_years(1, |game, _| {
        let year = game.play.built.game_data.conquest.game_year;
        for seat in &game.seats {
            assert!(
                seat.contacts
                    .iter()
                    .all(|contact| year - contact.year < CONTACT_MEMORY_YEARS)
            );
            contacts += seat.contacts.len();
        }
    });
    assert!(contacts > 0, "aggressor games should report foreign forces");
}

#[test]
fn a_new_seat_knows_its_own_worlds_and_no_others() {
    let game = seated_game(PLAYERS, 0);
    let game_data = &game.play.built.game_data;
    for seat in &game.seats {
        for (idx, planet) in game_data.planets.iter().enumerate() {
            let known_owner = seat.planet_intel[&(idx + 1)].known_owner_empire_id;
            if planet.owner.0 == seat.empire {
                assert_eq!(known_owner, Some(seat.empire));
            } else {
                assert_eq!(known_owner, None, "planet {}", idx + 1);
            }
        }
    }
}

/// A sighting the seat's fleet could not identify tells the seat what it
/// tells a player: an alien fleet is there, not whose it is or its size.
#[test]
fn an_unidentified_sighting_names_no_owner_or_size() {
    let game_data = nc_engine::GameStateBuilder::new()
        .with_player_count(PLAYERS)
        .with_year(3001)
        .build_initialized_baseline()
        .expect("baseline should build");
    let sighting = |identified| nc_data::ScoutContactEvent {
        viewer_empire: 1,
        source: nc_data::ContactReportSource::Fleet(1),
        reporting_fleet_number: Some(1),
        reporting_initial: nc_data::ShipLosses::default(),
        reporting_loaded_armies_initial: 0,
        coords: [5, 6],
        target_empire: 2,
        target_fleet_number: Some(3),
        small_vessels: 0,
        medium_vessels: 4,
        large_vessels: 1,
        target_speed: 2,
        in_system: false,
        identified,
        decision: None,
        stardate_week: Some(10),
    };
    for (identified, empire, medium) in [(false, 0, 0), (true, 2, 4)] {
        let mut memory = nc_ai::SeatMemory::new(&game_data, 1);
        let mut events = MaintenanceEvents::default();
        events.scout_contact_events.push(sighting(identified));
        memory.learn_year(&game_data, &events);
        let contact = memory.contacts.last().expect("the sighting is remembered");
        assert_eq!(contact.coords, [5, 6]);
        assert_eq!(
            (contact.empire, contact.medium, contact.large > 0),
            (empire, medium, identified)
        );
    }
}
