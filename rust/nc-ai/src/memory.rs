//! What one empire knows, carried from year to year.

use std::collections::BTreeMap;

use nc_data::{
    CoreGameData, EncounterDispositionEvent, IntelTier, MaintenanceEvents, PlanetIntelSnapshot,
    ShipLosses, latest_planet_intel_grants_for_viewer, merge_player_intel_from_runtime,
};

/// How many years a fleet or starbase contact stays in memory.
pub const CONTACT_MEMORY_YEARS: u16 = 5;

/// Which report named a foreign force.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ContactSource {
    /// A fleet or starbase detected it without fighting.
    Scout,
    /// One of the empire's fleets met it and did not fight, or withdrew.
    Encounter,
    /// The empire fought it.
    Battle,
    /// One of the empire's starbases shares its sector.
    Starbase,
}

/// A foreign force one empire's reports named.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Contact {
    /// The game year the report arrived.
    pub year: u16,
    pub coords: [u8; 2],
    pub empire: u8,
    pub source: ContactSource,
    /// Vessel size classes, as scout and encounter reports give them.
    pub small: u32,
    pub medium: u32,
    pub large: u32,
    /// Ship counts by type, when a battle or withdrawal report gives them.
    pub ships: Option<ShipLosses>,
    /// Starbases in the force.
    pub starbases: u32,
}

/// What one empire knows: planet intel as the campaign store keeps it, and
/// the foreign forces its reports named in recent years.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SeatMemory {
    pub empire: u8,
    pub planet_intel: BTreeMap<usize, PlanetIntelSnapshot>,
    pub contacts: Vec<Contact>,
}

impl SeatMemory {
    /// The empire's knowledge at the start of a game: its own worlds.
    pub fn new(game_data: &CoreGameData, empire: u8) -> Self {
        let year = game_data.conquest.game_year;
        Self {
            empire,
            planet_intel: merge_player_intel_from_runtime(game_data, empire, year, None, None),
            contacts: Vec::new(),
        }
    }

    /// Fold one maintenance year's reports into what the empire knows. Only
    /// events addressed to this empire are read.
    pub fn learn_year(&mut self, game_data: &CoreGameData, events: &MaintenanceEvents) {
        let year = game_data.conquest.game_year;
        let grants = latest_planet_intel_grants_for_viewer(events, self.empire);
        self.planet_intel = merge_player_intel_from_runtime(
            game_data,
            self.empire,
            year,
            Some(&self.planet_intel),
            Some(&grants),
        );
        self.learn_attack_reports(events, year);
        self.contacts
            .retain(|contact| year.saturating_sub(contact.year) < CONTACT_MEMORY_YEARS);
        self.contacts
            .extend(contacts_reported_to(events, self.empire, year));
    }
}

impl SeatMemory {
    /// What the empire's own attacks reported about their targets (t178):
    /// the world's batteries and armies after the fight, as the attacker's
    /// bombardment and assault reports state them. A human player reads the
    /// defences going in and the losses inflicted, so knows what is left.
    fn learn_attack_reports(&mut self, events: &MaintenanceEvents, year: u16) {
        // The attacks happened in the year just played; scouting intel is
        // dated the same way.
        let seen = year.saturating_sub(1);
        let empire = self.empire;
        let mut learn = |planet_idx: usize, batteries: u8, armies: u8| {
            let Some(intel) = self.planet_intel.get_mut(&(planet_idx + 1)) else {
                return;
            };
            if intel.known_owner_empire_id == Some(empire) {
                return;
            }
            intel.known_ground_batteries = Some(batteries);
            intel.known_armies = Some(armies);
            intel.last_intel_year = Some(seen);
            intel.seen_year = Some(seen);
            intel.intel_tier = IntelTier::Full;
        };
        for event in events
            .bombard_events
            .iter()
            .filter(|event| event.attacker_empire == empire)
        {
            learn(
                event.planet_idx,
                event
                    .defender_batteries_initial
                    .saturating_sub(event.defender_battery_losses),
                event
                    .defender_armies_initial
                    .saturating_sub(event.defender_army_losses),
            );
        }
        for event in events
            .assault_report_events
            .iter()
            .filter(|event| event.attacker_empire == empire)
        {
            learn(
                event.planet_idx,
                event
                    .defender_batteries_initial
                    .saturating_sub(event.defender_battery_losses),
                event
                    .defender_armies_initial
                    .saturating_sub(event.defender_army_losses_softening)
                    .saturating_sub(event.defender_army_losses),
            );
        }
    }
}

fn contacts_reported_to(events: &MaintenanceEvents, viewer: u8, year: u16) -> Vec<Contact> {
    let sized = |coords, empire, source, small, medium, large| Contact {
        year,
        coords,
        empire,
        source,
        small,
        medium,
        large,
        ships: None,
        starbases: 0,
    };
    let typed = |coords, empire, source, ships: ShipLosses, starbases| Contact {
        ships: Some(ships),
        starbases,
        ..sized(coords, empire, source, 0, 0, 0)
    };

    let mut contacts = Vec::new();
    for event in events
        .scout_contact_events
        .iter()
        .filter(|event| event.viewer_empire == viewer)
    {
        // A sighting that was not identified tells the player only where an
        // alien fleet is (empire 0 is unknown), and the seat no more.
        contacts.push(if event.identified {
            sized(
                event.coords,
                event.target_empire,
                ContactSource::Scout,
                event.small_vessels,
                event.medium_vessels,
                event.large_vessels,
            )
        } else {
            sized(event.coords, 0, ContactSource::Scout, 0, 0, 0)
        });
    }
    for event in &events.encounter_disposition_events {
        match *event {
            EncounterDispositionEvent::NoEngagement {
                owner_empire,
                coords,
                target_empire,
                small_vessels,
                medium_vessels,
                large_vessels,
                ..
            } if owner_empire == viewer => contacts.push(sized(
                coords,
                target_empire,
                ContactSource::Encounter,
                small_vessels,
                medium_vessels,
                large_vessels,
            )),
            EncounterDispositionEvent::Retreated {
                owner_empire,
                coords,
                target_empire,
                enemy_initial,
                ..
            }
            | EncounterDispositionEvent::PursuitFire {
                owner_empire,
                coords,
                target_empire,
                enemy_initial,
                ..
            } if owner_empire == viewer => contacts.push(typed(
                coords,
                target_empire,
                ContactSource::Encounter,
                enemy_initial,
                0,
            )),
            _ => {}
        }
    }
    for event in events
        .fleet_battle_events
        .iter()
        .filter(|event| event.reporting_empire == viewer)
    {
        let empire = event.enemy_empires.first().copied().unwrap_or(0);
        contacts.push(typed(
            event.coords,
            empire,
            ContactSource::Battle,
            event.enemy_initial,
            event.enemy_initial_starbases,
        ));
    }
    for event in events
        .starbase_sighting_events
        .iter()
        .filter(|event| event.viewer_empire == viewer)
    {
        contacts.push(typed(
            event.coords,
            event.target_empire,
            ContactSource::Starbase,
            ShipLosses::default(),
            1,
        ));
    }
    contacts
}
