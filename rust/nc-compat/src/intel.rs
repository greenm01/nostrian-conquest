use std::collections::BTreeMap;

use nc_classic::{DatabaseDat, DatabaseRecord};
use nc_data::{CoreGameData, IntelTier, PlanetIntelSnapshot, PlayerStarmapWorld};

pub fn merge_player_intel_from_compat(
    game_data: &CoreGameData,
    database: &DatabaseDat,
    viewer_empire_id: u8,
    year: u16,
    previous: Option<&BTreeMap<usize, PlanetIntelSnapshot>>,
) -> BTreeMap<usize, PlanetIntelSnapshot> {
    let planet_count = game_data.planets.len();
    let viewer_index = viewer_empire_id.saturating_sub(1) as usize;
    let viewer_is_active = game_data
        .players
        .get(viewer_index)
        .map(|player| player.mode.to_raw() != 0)
        .unwrap_or(false);
    compat_worlds(game_data, database, viewer_empire_id)
        .into_iter()
        .map(|world| {
            let planet_record_index_1_based = world.planet_record_index_1_based;
            let previous_snapshot =
                previous.and_then(|rows| rows.get(&planet_record_index_1_based));
            let record_index = DatabaseDat::record_index(
                planet_record_index_1_based - 1,
                viewer_index,
                planet_count,
            );
            let fallback_record = DatabaseRecord::new_zeroed();
            let db_record = database
                .records
                .get(record_index)
                .unwrap_or(&fallback_record);
            let mut current_snapshot =
                snapshot_from_compat_world(&world, viewer_empire_id, db_record);
            if current_snapshot.compat_is_orbit_seed
                && viewer_is_active
                && current_snapshot.intel_tier == IntelTier::Owned
            {
                if let Some(planet) = game_data.planets.get(planet_record_index_1_based - 1) {
                    current_snapshot.known_name = Some(planet.name.trim().to_string());
                }
                current_snapshot.last_intel_year = Some(year);
            } else if !current_snapshot.compat_is_orbit_seed
                && current_snapshot.intel_tier != IntelTier::Unknown
                && current_snapshot.last_intel_year == Some(0)
            {
                current_snapshot.last_intel_year = Some(year);
            }
            let merged_snapshot =
                merge_snapshot(previous_snapshot, &current_snapshot, viewer_empire_id, year);
            (planet_record_index_1_based, merged_snapshot)
        })
        .collect()
}

pub fn extract_player_intel_from_compat_database(
    game_data: &CoreGameData,
    database: &DatabaseDat,
    year: u16,
) -> Vec<BTreeMap<usize, PlanetIntelSnapshot>> {
    (1..=game_data.conquest.player_count)
        .map(|viewer_empire_id| {
            merge_player_intel_from_compat(game_data, database, viewer_empire_id, year, None)
        })
        .collect()
}

fn compat_worlds(
    game_data: &CoreGameData,
    database: &DatabaseDat,
    viewer_empire_id: u8,
) -> Vec<PlayerStarmapWorld> {
    let planet_count = game_data.planets.len();
    let viewer_index = viewer_empire_id.saturating_sub(1) as usize;
    game_data
        .planets
        .iter()
        .enumerate()
        .map(|(planet_index, planet)| {
            let record_index = DatabaseDat::record_index(planet_index, viewer_index, planet_count);
            let fallback_record = DatabaseRecord::new_zeroed();
            let db_record = database
                .records
                .get(record_index)
                .unwrap_or(&fallback_record);
            let decoded_name = decode_known_name(db_record);
            let known_name = decoded_name.clone();
            let known_owner_empire_id =
                decode_known_owner_empire_id(db_record, game_data, decoded_name.is_some());
            let known_owner_empire_name = known_owner_empire_id.and_then(|empire_id| {
                (empire_id >= 1).then(|| {
                    game_data.players[empire_id as usize - 1]
                        .empire_name
                        .trim()
                        .to_string()
                })
            });
            let known_potential_production = decode_known_u16(db_record.raw[0x1c]);
            let known_armies = decode_known_u8(db_record.raw[0x23]);
            let known_ground_batteries = decode_known_u8(db_record.raw[0x25]);

            PlayerStarmapWorld {
                planet_record_index_1_based: planet_index + 1,
                coords: planet.sector.to_array(),
                intel_tier: IntelTier::infer_from_fields(
                    viewer_empire_id,
                    known_owner_empire_id,
                    known_name.as_deref(),
                    known_potential_production,
                    known_armies,
                    known_ground_batteries,
                ),
                known_name,
                known_owner_empire_id,
                known_owner_empire_name,
                known_potential_production,
                known_armies,
                known_ground_batteries,
                known_starbase_count: None,
                known_current_production: decode_known_u8(db_record.raw[0x1d]),
                known_stored_points: decode_known_u16_word(db_record.word_at(0x1e)),
                known_docked_summary: None,
                known_orbit_summary: None,
            }
        })
        .collect()
}

fn infer_intel_tier(viewer_empire_id: u8, world: &PlayerStarmapWorld) -> IntelTier {
    IntelTier::infer_from_world(viewer_empire_id, world)
}

fn infer_intel_tier_from_snapshot(
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

fn snapshot_from_world(
    world: &PlayerStarmapWorld,
    viewer_empire_id: u8,
    last_intel_year: Option<u16>,
) -> PlanetIntelSnapshot {
    PlanetIntelSnapshot {
        planet_record_index_1_based: world.planet_record_index_1_based,
        intel_tier: infer_intel_tier(viewer_empire_id, world),
        compat_is_orbit_seed: false,
        last_intel_year,
        seen_year: last_intel_year,
        scout_year: last_intel_year,
        known_name: world.known_name.clone(),
        known_owner_empire_id: world.known_owner_empire_id,
        known_potential_production: world.known_potential_production,
        known_armies: world.known_armies,
        known_ground_batteries: world.known_ground_batteries,
        known_starbase_count: world.known_starbase_count,
        known_current_production: world.known_current_production,
        known_stored_points: world.known_stored_points,
        known_docked_summary: world.known_docked_summary.clone(),
        known_orbit_summary: world.known_orbit_summary.clone(),
        compat_word_1e: world.known_stored_points,
    }
}

fn snapshot_from_compat_world(
    world: &PlayerStarmapWorld,
    viewer_empire_id: u8,
    record: &DatabaseRecord,
) -> PlanetIntelSnapshot {
    let mut snapshot = snapshot_from_world(world, viewer_empire_id, None);
    snapshot.compat_is_orbit_seed = record.is_compat_orbit_seed_for_viewer(viewer_empire_id);
    if !matches!(snapshot.intel_tier, IntelTier::Unknown) {
        snapshot.last_intel_year = Some(record.word_at(0x16));
        snapshot.seen_year = Some(record.word_at(0x16));
        snapshot.scout_year = Some(record.word_at(0x27));
        snapshot.compat_word_1e = decode_known_u16_word(record.word_at(0x1e));
    } else {
        snapshot.compat_word_1e = None;
    }
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
        && left.seen_year == right.seen_year
        && left.scout_year == right.scout_year
        && left.compat_word_1e == right.compat_word_1e
}

fn decode_known_name(record: &DatabaseRecord) -> Option<String> {
    let name = String::from_utf8_lossy(record.planet_name_bytes())
        .trim()
        .to_string();
    if name.is_empty() || name.eq_ignore_ascii_case("unknown") {
        None
    } else {
        Some(name)
    }
}

fn decode_known_owner_empire_id(
    record: &DatabaseRecord,
    game_data: &CoreGameData,
    has_visible_name: bool,
) -> Option<u8> {
    let raw = record.raw[0x15];
    if raw == 0 && has_visible_name {
        Some(0)
    } else if raw >= 1 && raw <= game_data.conquest.player_count {
        Some(raw)
    } else {
        None
    }
}

fn decode_known_u16(raw: u8) -> Option<u16> {
    if raw == 0 || raw == 0xff {
        None
    } else {
        Some(raw as u16)
    }
}

fn decode_known_u8(raw: u8) -> Option<u8> {
    if raw == 0 || raw == 0xff {
        None
    } else {
        Some(raw)
    }
}

fn decode_known_u16_word(raw: u16) -> Option<u16> {
    if raw == 0 || raw == 0xffff {
        None
    } else {
        Some(raw)
    }
}
