use std::collections::BTreeMap;

use nc_classic::{DatabaseDat, DatabaseRecord};
use nc_data::{
    CoreGameData, IntelTier, MaintenanceEvents, Planet, PlanetIntelSnapshot, PlanetIntelSource,
    has_homeworld_seed_production, is_homeworld_seed_ignoring_name, present_production_points,
};

/// Regenerate classic `DATABASE.DAT` from structured runtime state plus
/// per-player intel snapshots.
///
/// `pre_maint_planets` is the planet state before maintenance ran, used to
/// preserve orbit/build-queue edge cases when projecting classic rows.
pub fn build_database_dat(
    game_data: &CoreGameData,
    pre_maint_planets: &[Planet],
    planet_intel_by_viewer: &[BTreeMap<usize, PlanetIntelSnapshot>],
    events: &MaintenanceEvents,
    template: Option<&DatabaseDat>,
) -> DatabaseDat {
    let player_count = game_data.conquest.player_count as usize;
    let planet_count = game_data.planets.len();
    let expected_record_count = player_count * planet_count;
    let template = template
        .filter(|db| db.records.len() == expected_record_count)
        .cloned();
    let template = template.as_ref();
    let mut new_database = DatabaseDat::generate_from_planets_and_year(
        &game_data
            .planets
            .iter()
            .map(|planet| planet.name.clone())
            .collect::<Vec<_>>(),
        game_data.conquest.game_year,
        player_count,
        None,
    );
    let current_intel_year = game_data.conquest.game_year.saturating_sub(1);
    let current_game_year = game_data.conquest.game_year;
    let current_turn_grants = collect_planet_intel_sources(events);

    for player in 0..player_count {
        let viewer_empire = (player + 1) as u8;
        let viewer_is_active = game_data.players[player].mode.code() != 0;
        let previous_rows = planet_intel_by_viewer.get(player);
        for planet_idx in 0..planet_count {
            let record_idx = DatabaseDat::record_index(planet_idx, player, planet_count);
            let template_record = template.and_then(|db| db.records.get(record_idx));
            let record = &mut new_database.records[record_idx];
            let planet = &game_data.planets[planet_idx];
            let snapshot = previous_rows.and_then(|rows| rows.get(&(planet_idx + 1)));
            let current_turn_grant = current_turn_grants.get(&(viewer_empire, planet_idx));
            let template_is_orbit = template_record.map(is_orbit_record).unwrap_or(false);
            let snapshot_is_orbit = snapshot
                .map(|row| {
                    row.compat_is_orbit_seed
                        && row.seen_year.unwrap_or(0) == 0
                        && row.scout_year.unwrap_or(0) == 0
                })
                .unwrap_or(false);
            let template_shows_owned_world = template_record
                .filter(|_| !template_is_orbit)
                .map(|row| row.raw[0x15] == viewer_empire)
                .unwrap_or(false);
            let snapshot_shows_owned_world = snapshot
                .filter(|_| !snapshot_is_orbit)
                .map(|row| row.intel_tier == IntelTier::Owned)
                .unwrap_or(false);
            let owns_world = !snapshot_is_orbit
                && planet.owner.0 == viewer_empire
                && (viewer_is_active || template_shows_owned_world || snapshot_shows_owned_world);

            if owns_world {
                apply_owned_world_row(
                    record,
                    template_record,
                    snapshot,
                    planet,
                    current_intel_year,
                );
                continue;
            }

            if let Some(source) = current_turn_grant.copied() {
                apply_intel_grant_row(
                    record,
                    template_record,
                    planet,
                    current_intel_year,
                    current_game_year,
                    source,
                );
                continue;
            }

            if snapshot_is_orbit {
                let snapshot = snapshot.expect("snapshot_is_orbit requires a snapshot row");
                apply_compat_orbit_snapshot_row(record, snapshot, viewer_empire);
                continue;
            }

            if let Some(snapshot) = snapshot {
                apply_snapshot_row(record, template_record, snapshot);
                continue;
            }

            if let Some(template_record) = template_record.filter(|row| is_orbit_record(row)) {
                preserve_orbit_record(
                    record,
                    template_record,
                    game_data,
                    pre_maint_planets,
                    planet_idx,
                    player,
                    current_intel_year,
                );
                continue;
            }

            record.set_unknown_planet();
        }
    }

    new_database
}

fn collect_planet_intel_sources(
    events: &MaintenanceEvents,
) -> BTreeMap<(u8, usize), PlanetIntelSource> {
    let mut sources = BTreeMap::new();
    for event in &events.planet_intel_events {
        sources.insert((event.viewer_empire, event.planet_idx), event.source);
    }
    sources
}

fn is_orbit_record(record: &DatabaseRecord) -> bool {
    record.is_compat_orbit_seed()
}

fn preserve_orbit_record(
    record: &mut DatabaseRecord,
    template_record: &DatabaseRecord,
    game_data: &CoreGameData,
    pre_maint_planets: &[Planet],
    planet_idx: usize,
    player_idx: usize,
    intel_year: u16,
) {
    record.copy_from(template_record);
    record.set_planet_name("Not Named Yet");
    set_year_word(record, 0x16, Some(intel_year));
    set_year_word(record, 0x18, Some(intel_year));
    set_year_word(record, 0x27, Some(intel_year));

    if planet_idx < pre_maint_planets.len() {
        let had_build_queue =
            (0..10).any(|slot| pre_maint_planets[planet_idx].build_queue[slot].points_owed > 0);
        if had_build_queue {
            record.raw[0x1e] = 0x00;
        }
    }

    if planet_idx < game_data.planets.len() {
        let planet = &game_data.planets[planet_idx];
        let planet_owner = planet.owner.0 as usize;
        if has_homeworld_seed_production(planet)
            && planet_owner > 0
            && planet_owner == player_idx + 1
        {
            let player_mode = game_data.players[player_idx].mode.code();
            let autopilot = u8::from(game_data.players[player_idx].autopilot);
            let ai_ran = player_mode == 0xff || (player_mode == 0x01 && autopilot == 0x01);
            if ai_ran {
                let owner_slot = planet_owner as u8;
                record.raw[0x1e] = unresolved_orbit_status_low_byte(
                    owner_slot,
                    &planet.name,
                    Some(template_record),
                );
                record.raw[0x23] = u8::try_from(planet.armies).unwrap_or(u8::MAX);
                record.raw[0x24] = 0x00;
            }
        }
    }
}

fn apply_compat_orbit_snapshot_row(
    record: &mut DatabaseRecord,
    snapshot: &PlanetIntelSnapshot,
    viewer_empire: u8,
) {
    if let Some(name) = snapshot.known_name.as_deref() {
        record.set_unknown_planet();
        record.set_planet_name(name);
    } else {
        record.set_blank_unknown_planet();
    }
    record.raw[0x15] = snapshot.known_owner_empire_id.unwrap_or(viewer_empire);
    if let Some(potential) = snapshot.known_potential_production {
        record.raw[0x1c] = potential.min(u16::from(u8::MAX)) as u8;
    }
    if let Some(current_production) = snapshot.known_current_production {
        record.raw[0x1d] = current_production;
    }
    if let Some(word_1e) = snapshot.compat_word_1e {
        record.set_word_at(0x1e, word_1e);
    }
    record.raw[0x23] = snapshot.known_armies.unwrap_or(0xff);
    record.raw[0x24] = if snapshot.known_armies.is_some() {
        0x00
    } else {
        0xff
    };
    record.raw[0x25] = snapshot.known_ground_batteries.unwrap_or(0xff);
    record.raw[0x26] = if snapshot.known_ground_batteries.is_some() {
        0x00
    } else {
        0xff
    };
    set_year_word(record, 0x16, snapshot.seen_year);
    set_year_word(record, 0x18, snapshot.seen_year);
    set_year_word(record, 0x27, snapshot.scout_year);
}

fn apply_snapshot_row(
    record: &mut DatabaseRecord,
    template_record: Option<&DatabaseRecord>,
    snapshot: &PlanetIntelSnapshot,
) {
    let Some(name) = snapshot.known_name.as_deref() else {
        record.set_unknown_planet();
        return;
    };
    let owner_slot = snapshot.known_owner_empire_id.unwrap_or(0);
    let Some(potential) = snapshot.known_potential_production else {
        record.set_unknown_planet();
        return;
    };
    let current_production = snapshot
        .known_current_production
        .map(u16::from)
        .or_else(|| template_current_production(template_record));
    let word_1e = snapshot
        .compat_word_1e
        .or(snapshot.known_stored_points)
        .or_else(|| template_word_1e(template_record));
    if let (Some(armies), Some(batteries)) =
        (snapshot.known_armies, snapshot.known_ground_batteries)
    {
        apply_visible_row(
            record,
            template_record,
            name,
            owner_slot,
            potential,
            current_production,
            word_1e,
            Some(armies),
            Some(batteries),
            snapshot.seen_year.or(snapshot.last_intel_year),
            snapshot.scout_year.or(snapshot.last_intel_year),
        );
    } else {
        apply_visible_row(
            record,
            template_record,
            name,
            owner_slot,
            potential,
            current_production,
            word_1e,
            None,
            None,
            snapshot.seen_year.or(snapshot.last_intel_year),
            snapshot.scout_year.or(snapshot.last_intel_year),
        );
    }
}

fn apply_owned_world_row(
    record: &mut DatabaseRecord,
    template_record: Option<&DatabaseRecord>,
    snapshot: Option<&PlanetIntelSnapshot>,
    planet: &nc_data::Planet,
    intel_year: u16,
) {
    let potential = u16::from(planet.potential_production);
    apply_visible_row(
        record,
        template_record,
        planet.name.as_str(),
        planet.owner.0,
        potential,
        template_current_production(template_record).or(Some(potential)),
        snapshot
            .and_then(|snapshot| snapshot.compat_word_1e)
            .or(Some(owned_row_word_1e(planet, template_record))),
        Some(u8::try_from(planet.armies).unwrap_or(u8::MAX)),
        Some(u8::try_from(planet.ground_batteries).unwrap_or(u8::MAX)),
        Some(intel_year),
        Some(intel_year),
    );
}

fn apply_intel_grant_row(
    record: &mut DatabaseRecord,
    template_record: Option<&DatabaseRecord>,
    planet: &nc_data::Planet,
    intel_year: u16,
    current_game_year: u16,
    source: PlanetIntelSource,
) {
    let potential = u16::from(planet.potential_production);
    let (current_production, word_1e, armies, batteries, seen_year, scout_year) = match source {
        PlanetIntelSource::ScoutSolarSystem => (
            Some(scout_visible_current_production(planet)),
            template_word_1e(template_record).or(Some(0x23)),
            Some(u8::try_from(planet.armies).unwrap_or(u8::MAX)),
            Some(u8::try_from(planet.ground_batteries).unwrap_or(u8::MAX)),
            Some(intel_year),
            Some(intel_year),
        ),
        PlanetIntelSource::ViewWorld => (
            template_current_production(template_record),
            template_word_1e(template_record),
            None,
            None,
            Some(intel_year),
            Some(intel_year),
        ),
        PlanetIntelSource::ColonizeBlockedByOwner => (
            template_current_production(template_record),
            template_word_1e(template_record),
            None,
            None,
            Some(intel_year),
            Some(intel_year),
        ),
        PlanetIntelSource::CivilDisorderContact => {
            apply_civil_disorder_contact_row(
                record,
                template_record,
                planet.owner.0,
                current_game_year,
            );
            return;
        }
        PlanetIntelSource::AssaultSuccess => (
            template_current_production(template_record),
            template_word_1e(template_record),
            Some(u8::try_from(planet.armies).unwrap_or(u8::MAX)),
            Some(u8::try_from(planet.ground_batteries).unwrap_or(u8::MAX)),
            Some(intel_year),
            Some(intel_year),
        ),
        PlanetIntelSource::AssaultFailure => {
            (None, None, None, None, Some(current_game_year), None)
        }
        // As ECMAINT writes it (the bombard-heavy fixture).
        PlanetIntelSource::Bombardment => (
            Some(scout_visible_current_production(planet)),
            template_word_1e(template_record).or(Some(0)),
            Some(u8::try_from(planet.armies).unwrap_or(u8::MAX)),
            Some(u8::try_from(planet.ground_batteries).unwrap_or(u8::MAX)),
            Some(intel_year),
            Some(intel_year),
        ),
    };
    apply_visible_row(
        record,
        template_record,
        planet.name.as_str(),
        planet.owner.0,
        potential,
        current_production,
        word_1e,
        armies,
        batteries,
        seen_year,
        scout_year,
    );
}

fn apply_civil_disorder_contact_row(
    record: &mut DatabaseRecord,
    template_record: Option<&DatabaseRecord>,
    owner_slot: u8,
    current_game_year: u16,
) {
    if let Some(template_record) = template_record {
        record.copy_from(template_record);
    } else {
        record.set_unknown_planet();
    }
    record.raw[0x15] = owner_slot;
    set_year_word(record, 0x16, Some(current_game_year));
    set_year_word(record, 0x18, Some(current_game_year));
    set_year_word(record, 0x27, Some(current_game_year));
}

fn apply_visible_row(
    record: &mut DatabaseRecord,
    template_record: Option<&DatabaseRecord>,
    planet_name: &str,
    owner_slot: u8,
    potential: u16,
    current_production: Option<u16>,
    word_1e: Option<u16>,
    armies: Option<u8>,
    batteries: Option<u8>,
    seen_year: Option<u16>,
    scout_year: Option<u16>,
) {
    if let Some(template_record) = template_record {
        record.copy_from(template_record);
    } else {
        record.set_unknown_planet();
    }
    record.set_planet_name(planet_name);
    record.raw[0x15] = owner_slot;
    set_year_word(record, 0x16, seen_year);
    set_year_word(record, 0x18, seen_year);
    record.raw[0x1c] = potential.min(u16::from(u8::MAX)) as u8;
    if let Some(current_production) = current_production {
        record.raw[0x1d] = current_production.min(u16::from(u8::MAX)) as u8;
    }
    if let Some(word_1e) = word_1e {
        record.set_word_at(0x1e, word_1e);
    }
    record.raw[0x23] = armies.unwrap_or(0xff);
    record.raw[0x24] = if armies.is_some() { 0x00 } else { 0xff };
    record.raw[0x25] = batteries.unwrap_or(0xff);
    record.raw[0x26] = if batteries.is_some() { 0x00 } else { 0xff };
    set_year_word(record, 0x27, scout_year);
}

fn set_year_word(record: &mut DatabaseRecord, offset: usize, year: Option<u16>) {
    let bytes = year.unwrap_or(0).to_le_bytes();
    record.raw[offset] = bytes[0];
    record.raw[offset + 1] = bytes[1];
}

fn template_current_production(template_record: Option<&DatabaseRecord>) -> Option<u16> {
    template_record
        .map(|record| record.raw[0x1d])
        .filter(|value| *value != 0xff)
        .map(u16::from)
}

fn template_word_1e(template_record: Option<&DatabaseRecord>) -> Option<u16> {
    template_record
        .map(|record| record.word_at(0x1e))
        .filter(|value| *value != u16::MAX)
}

fn scout_visible_current_production(planet: &nc_data::Planet) -> u16 {
    present_production_points(planet)
}

fn owned_row_word_1e(planet: &nc_data::Planet, template_record: Option<&DatabaseRecord>) -> u16 {
    if is_homeworld_seed_ignoring_name(planet)
        && template_record
            .filter(|row| {
                row.word_at(0x1e) == 0x23
                    && row.word_at(0x16) == 0
                    && row.word_at(0x18) == 0
                    && row.word_at(0x27) == 0
            })
            .is_some()
    {
        template_word_1e(template_record).unwrap_or(0x23)
    } else if planet.name.eq_ignore_ascii_case("not named yet") {
        0x23
    } else if let Some(template_record) =
        template_record.filter(|row| row.raw[0x1e] >= 0x41 && row.raw[0x1e] != 0xff)
    {
        template_record.word_at(0x1e)
    } else {
        u16::from(0x40u8.saturating_add(planet.owner.0))
    }
}

fn unresolved_orbit_status_low_byte(
    owner_slot: u8,
    planet_name: &str,
    template_record: Option<&DatabaseRecord>,
) -> u8 {
    if planet_name.eq_ignore_ascii_case("not named yet") {
        0x23
    } else if let Some(template_record) =
        template_record.filter(|row| row.raw[0x1e] >= 0x41 && row.raw[0x1e] != 0xff)
    {
        template_record.raw[0x1e]
    } else {
        0x40u8.saturating_add(owner_slot)
    }
}
