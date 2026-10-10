//! Ship conservation through maintenance, derived from events.
//!
//! Maintenance builds into stardocks, not fleets, and merges move ships
//! between fleets of one empire, so fleet ships across all empires change
//! only by reported losses, successful salvage, and civil-disorder
//! defections:
//!
//! after = before - battle/retreat/bombard/assault losses
//!                - salvaged fleets - defected fleets
//!
//! A salvaged fleet is identified by its standing Salvage order and target
//! before maintenance, a defected fleet by its empire and fleet number, and
//! either carries the ships of fleets merged into it this turn; when a
//! removed fleet is ambiguous the turn is not checked.

use std::collections::{BTreeMap, BTreeSet};

use nc_data::{
    CoreGameData, EncounterDispositionEvent, FleetRecord, Order, SalvageResolvedEvent, ShipLosses,
};

use super::{TurnObservation, Violation};

/// Ship counts in `[dd, ca, bb, sc, tt, et]` order.
type Ships = [u32; 6];

pub fn check(observation: &TurnObservation) -> Vec<Violation> {
    let before = fleet_ships(observation.before);
    let after = fleet_ships(observation.after);
    let Some(mut removed) = removed_fleets(observation) else {
        return Vec::new();
    };
    let events = observation.events;
    // One task force's losses at one location, reported by its battle event
    // and again by each of its fleets when it withdrew under ROE.
    let mut fleet_battle_losses = BTreeMap::<(u8, [u8; 2]), Ships>::new();
    let mut report = |empire: u8, coords: [u8; 2], losses: &ShipLosses| {
        let total = fleet_battle_losses.entry((empire, coords)).or_default();
        for (kind, lost) in losses_as_ships(losses).into_iter().enumerate() {
            total[kind] = total[kind].max(lost);
        }
    };
    for event in &events.fleet_battle_events {
        report(
            event.reporting_empire_raw,
            event.coords,
            &event.friendly_losses,
        );
    }
    for event in &events.encounter_disposition_events {
        if let EncounterDispositionEvent::Retreated {
            owner_empire_raw,
            coords,
            losses_sustained,
            ..
        }
        | EncounterDispositionEvent::PursuitFire {
            owner_empire_raw,
            coords,
            losses_sustained,
            ..
        } = event
        {
            report(*owner_empire_raw, *coords, losses_sustained);
        }
    }
    for losses in fleet_battle_losses.into_values() {
        add_ships(&mut removed, losses);
    }
    let mut add = |losses: &ShipLosses| add_ships(&mut removed, losses_as_ships(losses));
    for event in &events.bombard_events {
        add(&event.attacker_losses);
    }
    for event in &events.assault_report_events {
        add(&event.attacker_ship_losses);
    }

    let expected = std::array::from_fn(|kind| before[kind].saturating_sub(removed[kind]));
    if after == expected {
        Vec::new()
    } else {
        vec![Violation::ShipConservation {
            before,
            after,
            expected,
        }]
    }
}

/// Ships each fleet carries once this turn's merges are applied, keyed by
/// empire and fleet number.
fn merged_fleets(observation: &TurnObservation) -> BTreeMap<(u8, u16), Ships> {
    let mut fleets = BTreeMap::new();
    for fleet in &observation.before.fleets.records {
        add_ships(fleets.entry(fleet_key(fleet)).or_default(), ships_of(fleet));
    }
    let mut seen = BTreeSet::new();
    for event in &observation.events.fleet_merge_events {
        let owner = event.owner_empire_raw;
        if !seen.insert((owner, event.host_fleet_number, event.absorbed_fleet_number)) {
            continue;
        }
        let absorbed = fleets
            .remove(&(owner, u16::from(event.absorbed_fleet_number)))
            .unwrap_or_default();
        add_ships(
            fleets
                .entry((owner, u16::from(event.host_fleet_number)))
                .or_default(),
            absorbed,
        );
    }
    fleets
}

/// Ships removed whole by successful salvage and civil-disorder defection,
/// or `None` when a removed fleet cannot be identified.
fn removed_fleets(observation: &TurnObservation) -> Option<Ships> {
    let merged = merged_fleets(observation);
    let mut removed = [0; 6];
    for event in &observation.events.salvage_events {
        let SalvageResolvedEvent::Succeeded {
            owner_empire_raw,
            coords,
            ..
        } = *event
        else {
            continue;
        };
        let mut candidates = observation.before.fleets.records.iter().filter(|fleet| {
            fleet.owner_empire_raw() == owner_empire_raw
                && fleet.standing_order_kind() == Order::Salvage
                && fleet.standing_order_target_coords_raw() == coords
        });
        let fleet = candidates.next()?;
        if candidates.next().is_some() {
            return None;
        }
        add_ships(&mut removed, *merged.get(&fleet_key(fleet))?);
    }
    for event in &observation.events.fleet_defection_events {
        let key = (event.reporting_empire_raw, u16::from(event.fleet_number));
        add_ships(&mut removed, *merged.get(&key)?);
    }
    Some(removed)
}

fn fleet_key(fleet: &FleetRecord) -> (u8, u16) {
    (fleet.owner_empire_raw(), fleet.local_slot_word_raw())
}

fn fleet_ships(state: &CoreGameData) -> Ships {
    let mut total = [0; 6];
    for fleet in &state.fleets.records {
        add_ships(&mut total, ships_of(fleet));
    }
    total
}

fn ships_of(fleet: &FleetRecord) -> Ships {
    [
        u32::from(fleet.destroyer_count()),
        u32::from(fleet.cruiser_count()),
        u32::from(fleet.battleship_count()),
        u32::from(fleet.scout_count()),
        u32::from(fleet.troop_transport_count()),
        u32::from(fleet.etac_count()),
    ]
}

fn losses_as_ships(losses: &ShipLosses) -> Ships {
    [
        losses.destroyers,
        losses.cruisers,
        losses.battleships,
        losses.scouts,
        losses.transports,
        losses.etacs,
    ]
}

fn add_ships(total: &mut Ships, ships: Ships) {
    for (sum, count) in total.iter_mut().zip(ships) {
        *sum += count;
    }
}
