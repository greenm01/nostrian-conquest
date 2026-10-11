use std::collections::BTreeMap;

use crate::storage::{IntelTier, PlanetIntelSnapshot};
use crate::{CoreGameData, PlanetIntelEvent, PlanetIntelSource};
use crate::{has_any_force, present_production_points, stored_production_points};

pub fn merge_player_intel_from_runtime(
    game_data: &CoreGameData,
    viewer_empire_id: u8,
    year: u16,
    previous: Option<&BTreeMap<usize, PlanetIntelSnapshot>>,
    current_turn_grants: Option<&BTreeMap<usize, PlanetIntelEvent>>,
) -> BTreeMap<usize, PlanetIntelSnapshot> {
    game_data
        .planets
        .iter()
        .enumerate()
        .map(|(planet_index, planet)| {
            let planet_record_index_1_based = planet_index + 1;
            let previous_snapshot =
                previous.and_then(|rows| rows.get(&planet_record_index_1_based));
            let current_turn_grant =
                current_turn_grants.and_then(|rows| rows.get(&planet_record_index_1_based));
            let current_snapshot = snapshot_from_runtime(
                planet_record_index_1_based,
                game_data,
                planet,
                viewer_empire_id,
                year,
                current_turn_grant,
            );
            let mut merged_snapshot =
                merge_snapshot(previous_snapshot, &current_snapshot, viewer_empire_id, year);
            if previous_snapshot
                .map(|snapshot| {
                    snapshot.compat_is_orbit_seed
                        && snapshot.seen_year == Some(0)
                        && snapshot.scout_year == Some(0)
                })
                .unwrap_or(false)
                && current_turn_grant.is_none()
                && year == 3000
            {
                merged_snapshot.compat_is_orbit_seed = true;
                merged_snapshot.last_intel_year = Some(0);
                merged_snapshot.seen_year = Some(0);
                merged_snapshot.scout_year = Some(0);
            } else if previous_snapshot
                .map(|snapshot| {
                    snapshot.compat_is_orbit_seed
                        && snapshot.seen_year == Some(0)
                        && snapshot.scout_year == Some(0)
                })
                .unwrap_or(false)
                && current_turn_grant.is_none()
                && merged_snapshot.intel_tier == IntelTier::Owned
                && year > 3000
            {
                let compat_year = year.saturating_sub(1);
                merged_snapshot.seen_year = Some(compat_year);
                merged_snapshot.scout_year = Some(compat_year);
            }
            if current_turn_grant.is_some() || merged_snapshot.intel_tier != IntelTier::Owned {
                merged_snapshot.compat_is_orbit_seed = false;
            }
            if current_turn_grant.is_none()
                && previous_snapshot.is_some()
                && viewer_has_fleet_presence(game_data, viewer_empire_id, planet.sector.to_array())
            {
                merged_snapshot = refresh_visible_snapshot_from_runtime(
                    game_data,
                    &merged_snapshot,
                    planet,
                    year,
                );
            }
            (planet_record_index_1_based, merged_snapshot)
        })
        .collect()
}

pub fn build_runtime_planet_intel_snapshot(
    game_data: &CoreGameData,
    viewer_empire_id: u8,
    year: u16,
    planet_idx: usize,
    source: PlanetIntelSource,
) -> Option<PlanetIntelSnapshot> {
    let planet = game_data.planets.get(planet_idx)?;
    Some(snapshot_from_runtime_grant(
        PlanetIntelSnapshot {
            planet_record_index_1_based: planet_idx + 1,
            intel_tier: IntelTier::Unknown,
            compat_is_orbit_seed: false,
            last_intel_year: None,
            seen_year: None,
            scout_year: None,
            known_name: None,
            known_owner_empire_id: None,
            known_potential_production: None,
            known_armies: None,
            known_ground_batteries: None,
            known_starbase_count: None,
            known_current_production: None,
            known_stored_points: None,
            known_docked_summary: None,
            known_orbit_summary: None,
            compat_word_1e: None,
        },
        game_data,
        planet,
        source,
        viewer_empire_id,
        year,
    ))
}

pub fn latest_planet_intel_grants_for_viewer(
    events: &crate::MaintenanceEvents,
    viewer_empire_id: u8,
) -> BTreeMap<usize, PlanetIntelEvent> {
    let mut grants = BTreeMap::new();
    for event in events
        .planet_intel_events
        .iter()
        .filter(|event| event.viewer_empire_raw == viewer_empire_id)
    {
        let entry = grants
            .entry(event.planet_idx + 1)
            .or_insert_with(|| event.clone());
        if event.stardate_week.unwrap_or(0) >= entry.stardate_week.unwrap_or(0) {
            *entry = event.clone();
        }
    }
    grants
}

pub(crate) fn infer_intel_tier_from_snapshot(
    viewer_empire_id: u8,
    snapshot: &PlanetIntelSnapshot,
) -> IntelTier {
    if snapshot.known_owner_empire_id == Some(viewer_empire_id) {
        IntelTier::Owned
    } else if snapshot.known_armies.is_some() || snapshot.known_ground_batteries.is_some() {
        IntelTier::Full
    } else if snapshot.known_name.is_some()
        || snapshot.known_owner_empire_id.is_some()
        || snapshot.known_potential_production.is_some()
    {
        IntelTier::Partial
    } else {
        IntelTier::Unknown
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct OrbitPresence {
    fleet_count: u32,
    starbase_count: u8,
}

pub fn active_starbase_count_at(game_data: &CoreGameData, coords: [u8; 2]) -> u8 {
    orbit_presence(game_data, coords).starbase_count
}

fn snapshot_from_runtime(
    planet_record_index_1_based: usize,
    game_data: &CoreGameData,
    planet: &crate::Planet,
    viewer_empire_id: u8,
    year: u16,
    current_turn_grant: Option<&PlanetIntelEvent>,
) -> PlanetIntelSnapshot {
    let owner_empire_id = planet.owner.0;
    if owner_empire_id == viewer_empire_id {
        let starbase_count = active_starbase_count_at(game_data, planet.sector.to_array());
        return PlanetIntelSnapshot {
            planet_record_index_1_based,
            intel_tier: IntelTier::Owned,
            compat_is_orbit_seed: false,
            last_intel_year: None,
            seen_year: None,
            scout_year: None,
            known_name: Some(planet.name.trim().to_string()),
            known_owner_empire_id: Some(viewer_empire_id),
            known_potential_production: Some(u16::from(planet.potential_production)),
            known_armies: Some(u8::try_from(planet.armies).unwrap_or(u8::MAX)),
            known_ground_batteries: Some(u8::try_from(planet.ground_batteries).unwrap_or(u8::MAX)),
            known_starbase_count: Some(starbase_count),
            known_current_production: Some(present_production_points(planet))
                .map(|value| value.min(u16::from(u8::MAX)) as u8),
            known_stored_points: Some(
                stored_production_points(planet).min(u32::from(u16::MAX)) as u16
            ),
            known_docked_summary: None,
            known_orbit_summary: None,
            compat_word_1e: None,
        };
    }

    let mut snapshot = PlanetIntelSnapshot {
        planet_record_index_1_based,
        intel_tier: IntelTier::Unknown,
        compat_is_orbit_seed: false,
        last_intel_year: None,
        seen_year: None,
        scout_year: None,
        known_name: None,
        known_owner_empire_id: None,
        known_potential_production: None,
        known_armies: None,
        known_ground_batteries: None,
        known_starbase_count: None,
        known_current_production: None,
        known_stored_points: None,
        known_docked_summary: None,
        known_orbit_summary: None,
        compat_word_1e: None,
    };

    if let Some(grant) = current_turn_grant {
        snapshot =
            snapshot_from_turn_grant(snapshot, game_data, planet, viewer_empire_id, year, grant);
    }

    snapshot.intel_tier = infer_intel_tier_from_snapshot(viewer_empire_id, &snapshot);
    snapshot
}

fn snapshot_from_turn_grant(
    snapshot: PlanetIntelSnapshot,
    game_data: &CoreGameData,
    planet: &crate::Planet,
    viewer_empire_id: u8,
    year: u16,
    grant: &PlanetIntelEvent,
) -> PlanetIntelSnapshot {
    if let Some(observed) = &grant.observed_snapshot {
        let mut snapshot = observed.clone();
        snapshot.planet_record_index_1_based = grant.planet_idx + 1;
        snapshot.intel_tier = infer_intel_tier_from_snapshot(viewer_empire_id, &snapshot);
        return snapshot;
    }

    snapshot_from_runtime_grant(
        snapshot,
        game_data,
        planet,
        grant.source,
        viewer_empire_id,
        year,
    )
}

fn snapshot_from_runtime_grant(
    mut snapshot: PlanetIntelSnapshot,
    game_data: &CoreGameData,
    planet: &crate::Planet,
    source: PlanetIntelSource,
    viewer_empire_id: u8,
    year: u16,
) -> PlanetIntelSnapshot {
    let compat_year = year.saturating_sub(1);

    match source {
        PlanetIntelSource::ScoutSolarSystem => {
            snapshot.known_name = Some(planet.name.trim().to_string());
            snapshot.known_owner_empire_id = Some(planet.owner.0);
            snapshot.known_potential_production = Some(u16::from(planet.potential_production));
            let orbit_presence = orbit_presence(game_data, planet.sector.to_array());
            snapshot.known_armies = Some(u8::try_from(planet.armies).unwrap_or(u8::MAX));
            snapshot.known_ground_batteries =
                Some(u8::try_from(planet.ground_batteries).unwrap_or(u8::MAX));
            snapshot.known_starbase_count = Some(orbit_presence.starbase_count);
            snapshot.known_current_production = Some(present_production_points(planet))
                .map(|value| value.min(u16::from(u8::MAX)) as u8);
            snapshot.known_stored_points =
                Some(stored_production_points(planet).min(u32::from(u16::MAX)) as u16);
            snapshot.known_docked_summary = Some(crate::format_stardock_summary(
                planet,
                crate::CompactUnitSummaryStyle::Words,
            ));
            snapshot.known_orbit_summary = Some(format_orbit_summary(orbit_presence));
            snapshot.compat_word_1e = Some(0x23);
            snapshot.last_intel_year = Some(compat_year);
            snapshot.seen_year = Some(compat_year);
            snapshot.scout_year = Some(compat_year);
        }
        PlanetIntelSource::AssaultSuccess => {
            snapshot.known_name = Some(planet.name.trim().to_string());
            snapshot.known_owner_empire_id = Some(planet.owner.0);
            snapshot.known_potential_production = Some(u16::from(planet.potential_production));
            snapshot.known_armies = Some(u8::try_from(planet.armies).unwrap_or(u8::MAX));
            snapshot.known_ground_batteries =
                Some(u8::try_from(planet.ground_batteries).unwrap_or(u8::MAX));
            snapshot.last_intel_year = Some(compat_year);
            snapshot.seen_year = Some(compat_year);
            snapshot.scout_year = Some(compat_year);
        }
        PlanetIntelSource::ViewWorld => {
            snapshot.known_name = Some(planet.name.trim().to_string());
            snapshot.known_owner_empire_id = Some(planet.owner.0);
            snapshot.known_potential_production = Some(u16::from(planet.potential_production));
            snapshot.last_intel_year = Some(compat_year);
            snapshot.seen_year = Some(compat_year);
            snapshot.scout_year = Some(compat_year);
        }
        PlanetIntelSource::ColonizeBlockedByOwner => {
            snapshot.known_name = Some(planet.name.trim().to_string());
            snapshot.known_owner_empire_id = Some(planet.owner.0);
            snapshot.known_potential_production = Some(u16::from(planet.potential_production));
            snapshot.last_intel_year = Some(compat_year);
            snapshot.seen_year = Some(compat_year);
            snapshot.scout_year = Some(compat_year);
        }
        PlanetIntelSource::CivilDisorderContact => {
            snapshot.known_owner_empire_id = Some(planet.owner.0);
            snapshot.last_intel_year = Some(compat_year);
            snapshot.seen_year = Some(compat_year);
            snapshot.scout_year = Some(compat_year);
        }
        PlanetIntelSource::AssaultFailure => {
            snapshot.known_name = Some(planet.name.trim().to_string());
            snapshot.known_owner_empire_id = Some(planet.owner.0);
            snapshot.known_potential_production = Some(u16::from(planet.potential_production));
            snapshot.last_intel_year = Some(year);
            snapshot.seen_year = Some(year);
            snapshot.scout_year = Some(0);
        }
    }

    snapshot.intel_tier = infer_intel_tier_from_snapshot(viewer_empire_id, &snapshot);
    snapshot
}

fn merge_snapshot(
    previous: Option<&PlanetIntelSnapshot>,
    current: &PlanetIntelSnapshot,
    viewer_empire_id: u8,
    year: u16,
) -> PlanetIntelSnapshot {
    let mut merged = current.clone();
    if let Some(previous) = previous {
        if merged.known_name.is_none() {
            merged.known_name = previous.known_name.clone();
        }
        if merged.known_owner_empire_id.is_none() {
            merged.known_owner_empire_id = previous.known_owner_empire_id;
        }
        if merged.known_potential_production.is_none() {
            merged.known_potential_production = previous.known_potential_production;
        }
        if merged.known_armies.is_none() {
            merged.known_armies = previous.known_armies;
        }
        if merged.known_ground_batteries.is_none() {
            merged.known_ground_batteries = previous.known_ground_batteries;
        }
        if merged.known_starbase_count.is_none() {
            merged.known_starbase_count = previous.known_starbase_count;
        }
        if merged.known_current_production.is_none() {
            merged.known_current_production = previous.known_current_production;
        }
        if merged.known_stored_points.is_none() {
            merged.known_stored_points = previous.known_stored_points;
        }
        if merged.known_docked_summary.is_none() {
            merged.known_docked_summary = previous.known_docked_summary.clone();
        }
        if merged.known_orbit_summary.is_none() {
            merged.known_orbit_summary = previous.known_orbit_summary.clone();
        }
        if merged.compat_word_1e.is_none() {
            merged.compat_word_1e = previous.compat_word_1e;
        }
        if !merged.compat_is_orbit_seed {
            merged.compat_is_orbit_seed = previous.compat_is_orbit_seed;
        }
        if merged.seen_year.is_none() {
            merged.seen_year = previous.seen_year;
        }
        if merged.scout_year.is_none() {
            merged.scout_year = previous.scout_year;
        }
    }

    merged.intel_tier = infer_intel_tier_from_snapshot(viewer_empire_id, &merged);
    let compat_year = year.saturating_sub(1);
    merged.last_intel_year = match merged.intel_tier {
        IntelTier::Unknown => None,
        IntelTier::Owned => merged.last_intel_year.or_else(|| {
            previous
                .and_then(|snapshot| snapshot.last_intel_year)
                .map(|_| compat_year)
        }),
        _ => {
            let previous_year = previous.and_then(|snapshot| snapshot.last_intel_year);
            if merged.last_intel_year.is_some() {
                merged.last_intel_year
            } else if previous
                .map(|snapshot| snapshot_fingerprint_matches(snapshot, &merged))
                .unwrap_or(false)
            {
                previous_year.or(Some(compat_year))
            } else {
                Some(compat_year)
            }
        }
    };
    if !matches!(merged.intel_tier, IntelTier::Unknown) {
        merged.seen_year = merged.seen_year.or(match merged.intel_tier {
            IntelTier::Owned => previous
                .and_then(|snapshot| snapshot.seen_year)
                .map(|_| compat_year),
            _ => Some(merged.last_intel_year.unwrap_or(compat_year)),
        });
    }
    if !matches!(merged.intel_tier, IntelTier::Unknown) {
        merged.scout_year = merged.scout_year.or(match merged.intel_tier {
            IntelTier::Owned => previous
                .and_then(|snapshot| snapshot.scout_year)
                .map(|_| compat_year),
            _ => merged.seen_year,
        });
    }

    merged
}

fn viewer_has_fleet_presence(
    game_data: &CoreGameData,
    viewer_empire_id: u8,
    coords: [u8; 2],
) -> bool {
    game_data.fleets.iter().any(|fleet| {
        fleet.owner.0 == viewer_empire_id
            && fleet.sector.to_array() == coords
            && has_any_force(fleet)
    })
}

fn refresh_visible_snapshot_from_runtime(
    game_data: &CoreGameData,
    snapshot: &PlanetIntelSnapshot,
    planet: &crate::Planet,
    year: u16,
) -> PlanetIntelSnapshot {
    if matches!(snapshot.intel_tier, IntelTier::Unknown | IntelTier::Owned) {
        return snapshot.clone();
    }

    let compat_year = year.saturating_sub(1);
    let mut refreshed = snapshot.clone();
    refreshed.last_intel_year = Some(compat_year);
    refreshed.seen_year = Some(compat_year);
    if snapshot.scout_year.is_some() && snapshot.scout_year != Some(0) {
        refreshed.scout_year = Some(compat_year);
    }
    refreshed.known_name = Some(planet.name.trim().to_string());
    refreshed.known_owner_empire_id = Some(planet.owner.0);
    refreshed.known_potential_production = Some(u16::from(planet.potential_production));

    if refreshed.intel_tier == IntelTier::Full {
        let orbit_presence = orbit_presence(game_data, planet.sector.to_array());
        refreshed.known_armies = Some(u8::try_from(planet.armies).unwrap_or(u8::MAX));
        refreshed.known_ground_batteries =
            Some(u8::try_from(planet.ground_batteries).unwrap_or(u8::MAX));
        refreshed.known_starbase_count = Some(orbit_presence.starbase_count);
        refreshed.known_current_production = Some(present_production_points(planet))
            .map(|value| value.min(u16::from(u8::MAX)) as u8);
        refreshed.known_stored_points =
            Some(stored_production_points(planet).min(u32::from(u16::MAX)) as u16);
        refreshed.known_docked_summary = Some(crate::format_stardock_summary(
            planet,
            crate::CompactUnitSummaryStyle::Words,
        ));
        refreshed.known_orbit_summary = Some(format_orbit_summary(orbit_presence));
    }

    refreshed
}

fn snapshot_fingerprint_matches(left: &PlanetIntelSnapshot, right: &PlanetIntelSnapshot) -> bool {
    left.intel_tier == right.intel_tier
        && left.compat_is_orbit_seed == right.compat_is_orbit_seed
        && left.known_name == right.known_name
        && left.known_owner_empire_id == right.known_owner_empire_id
        && left.known_potential_production == right.known_potential_production
        && left.known_armies == right.known_armies
        && left.known_ground_batteries == right.known_ground_batteries
        && left.known_starbase_count == right.known_starbase_count
        && left.known_current_production == right.known_current_production
        && left.known_stored_points == right.known_stored_points
        && left.known_docked_summary == right.known_docked_summary
        && left.known_orbit_summary == right.known_orbit_summary
        && left.seen_year == right.seen_year
        && left.scout_year == right.scout_year
        && left.compat_word_1e == right.compat_word_1e
}

fn orbit_presence(game_data: &CoreGameData, coords: [u8; 2]) -> OrbitPresence {
    let fleet_count = game_data
        .fleets
        .iter()
        .filter(|fleet| fleet.sector.to_array() == coords && has_any_force(fleet))
        .count() as u32;
    let starbase_count = game_data
        .bases
        .iter()
        .filter(|base| base.sector.to_array() == coords && base.base_id.0 != 0)
        .count()
        .min(usize::from(u8::MAX)) as u8;

    OrbitPresence {
        fleet_count,
        starbase_count,
    }
}

fn format_orbit_summary(orbit_presence: OrbitPresence) -> String {
    let fleet_count = orbit_presence.fleet_count;
    let starbase_count = u32::from(orbit_presence.starbase_count);

    let mut parts = Vec::new();
    if fleet_count > 0 {
        parts.push(format!(
            "{} {}",
            fleet_count,
            if fleet_count == 1 { "fleet" } else { "fleets" }
        ));
    }
    if starbase_count > 0 {
        parts.push(format!(
            "{} {}",
            starbase_count,
            if starbase_count == 1 {
                "starbase"
            } else {
                "starbases"
            }
        ));
    }

    if parts.is_empty() {
        "Nothing".to_string()
    } else {
        parts.join(", ")
    }
}
