//! Ship conservation through maintenance, derived from events.
//!
//! Maintenance builds into stardocks, not fleets, and merges move ships
//! between fleets of one empire, so fleet ships across all empires change
//! only by reported losses, successful salvage, civil-disorder
//! defections and the planetless rogue orders pass:
//!
//! after = before - battle/retreat/bombard/assault losses
//!                - salvaged fleets - defected fleets - disbanded unarmed fleets
//!
//! A salvaged fleet is identified by its standing Salvage order and target
//! before maintenance, a defected fleet by its empire and fleet number, and
//! either carries the ships of fleets merged into it this turn; when a
//! removed fleet is ambiguous, or a defecting empire also took losses, the
//! turn is not checked.

use std::collections::{BTreeMap, BTreeSet};

use nc_data::{
    CoreGameData, EncounterDispositionEvent, Fleet, Order, PlayerMode, SalvageResolvedEvent,
    ShipLosses, has_combat_ships, troop_transports,
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
    // An empire's battle losses at one location: a departure interception
    // and the main battle there each report their own, and each fleet that
    // withdrew under ROE reports its share again.
    let mut fleet_battle_losses = BTreeMap::<(u8, [u8; 2]), Ships>::new();
    for event in &events.fleet_battle_events {
        add_ships(
            fleet_battle_losses
                .entry((event.reporting_empire, event.coords))
                .or_default(),
            losses_as_ships(&event.friendly_losses),
        );
    }
    let mut report = |empire: u8, coords: [u8; 2], losses: &ShipLosses| {
        let total = fleet_battle_losses.entry((empire, coords)).or_default();
        for (kind, lost) in losses_as_ships(losses).into_iter().enumerate() {
            total[kind] = total[kind].max(lost);
        }
    };
    for event in &events.encounter_disposition_events {
        if let EncounterDispositionEvent::Retreated {
            owner_empire,
            coords,
            losses_sustained,
            ..
        }
        | EncounterDispositionEvent::PursuitFire {
            owner_empire,
            coords,
            losses_sustained,
            ..
        } = event
        {
            report(*owner_empire, *coords, losses_sustained);
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
    for fleet in &observation.before.fleets {
        add_ships(fleets.entry(fleet_key(fleet)).or_default(), ships_of(fleet));
    }
    let mut seen = BTreeSet::new();
    for event in &observation.events.fleet_merge_events {
        let owner = event.owner_empire;
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
    // The pre-tick orders pass disbands unarmed fleets of planetless rogues,
    // including civil-disorder empires converted at its start. It reads the
    // saved planet count; those empires do not participate in the AI merge.
    for fleet in &observation.before.fleets {
        let Some(player) = observation
            .before
            .players
            .get(usize::from(fleet.owner.0).wrapping_sub(1))
        else {
            continue;
        };
        if player.planet_count == 0
            && matches!(player.mode, PlayerMode::Rogue | PlayerMode::CivilDisorder)
            && !has_combat_ships(fleet)
        {
            add_ships(&mut removed, ships_of(fleet));
        }
    }
    for event in &observation.events.salvage_events {
        let SalvageResolvedEvent::Succeeded {
            owner_empire,
            fleet_number,
            ..
        } = *event;
        // The fleet's target may have changed during the year, when its
        // world was lost, so it is found by its number.
        let mut candidates = observation.before.fleets.iter().filter(|fleet| {
            fleet.owner.0 == owner_empire
                && fleet.order == Order::Salvage
                && fleet.fleet_number.0 == u16::from(fleet_number)
        });
        let fleet = candidates.next()?;
        if candidates.next().is_some() {
            return None;
        }
        add_ships(&mut removed, *merged.get(&fleet_key(fleet))?);
    }
    for event in &observation.events.fleet_defection_events {
        // Combat runs before defection, so a defecting empire that also took
        // losses leaves the defected fleet's final composition unknown.
        if took_losses(observation, event.reporting_empire) {
            return None;
        }
        let key = (event.reporting_empire, u16::from(event.fleet_number));
        add_ships(&mut removed, *merged.get(&key)?);
    }
    Some(removed)
}

fn took_losses(observation: &TurnObservation, empire: u8) -> bool {
    let events = observation.events;
    events
        .fleet_battle_events
        .iter()
        .any(|event| event.reporting_empire == empire)
        || events.encounter_disposition_events.iter().any(|event| {
            matches!(
                event,
                EncounterDispositionEvent::Retreated { owner_empire, .. }
                    | EncounterDispositionEvent::PursuitFire { owner_empire, .. }
                    if *owner_empire == empire
            )
        })
        || events
            .bombard_events
            .iter()
            .any(|event| event.attacker_empire == empire)
        || events
            .assault_report_events
            .iter()
            .any(|event| event.attacker_empire == empire)
}

fn fleet_key(fleet: &Fleet) -> (u8, u16) {
    (fleet.owner.0, fleet.fleet_number.0)
}

fn fleet_ships(state: &CoreGameData) -> Ships {
    let mut total = [0; 6];
    for fleet in &state.fleets {
        add_ships(&mut total, ships_of(fleet));
    }
    total
}

fn ships_of(fleet: &Fleet) -> Ships {
    [
        u32::from(fleet.ships.destroyers),
        u32::from(fleet.ships.cruisers),
        u32::from(fleet.ships.battleships),
        u32::from(fleet.ships.scouts),
        u32::from(troop_transports(fleet)),
        u32::from(fleet.ships.etacs),
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
