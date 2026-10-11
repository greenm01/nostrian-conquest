use std::collections::BTreeMap;

use crate::homeworld_seed::is_named_homeworld_seed;
use crate::{
    CoreGameData, Planet, ProductionItemKind, STARDOCK_SLOT_COUNT, has_any_force,
    is_homeworld_seed_ignoring_name, present_production_points, stardock_slot_kind,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompactUnitSummaryStyle {
    JoinedCodes,
    DashedCodes,
    Words,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrbitPresenceSummary {
    pub fleets: usize,
    pub starbases: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OwnedPlanetStatus {
    Scorched,
    Homeworld,
    StarbasePresent,
    FactoriesDestroyed,
    FactoriesDamaged,
    FactoriesFunctional,
}

pub fn compact_unit_code(kind: ProductionItemKind) -> &'static str {
    match kind {
        ProductionItemKind::Destroyer => "DD",
        ProductionItemKind::Cruiser => "CA",
        ProductionItemKind::Battleship => "BB",
        ProductionItemKind::Scout => "SC",
        ProductionItemKind::Transport => "TT",
        ProductionItemKind::Etac => "ET",
        ProductionItemKind::Army => "AR",
        ProductionItemKind::GroundBattery => "GB",
        ProductionItemKind::Starbase => "SB",
        ProductionItemKind::Unknown(_) => "UN",
    }
}

pub fn stardock_unit_counts(planet: &Planet) -> BTreeMap<u8, u32> {
    let mut counts_by_kind = BTreeMap::<u8, u32>::new();
    for slot in 0..STARDOCK_SLOT_COUNT {
        let count = u32::from(u16::try_from(planet.stardock[slot]).unwrap_or(u16::MAX));
        let kind = stardock_slot_kind(planet, slot).code();
        if count == 0 || kind == 0 {
            continue;
        }
        *counts_by_kind.entry(kind).or_default() += count;
    }
    counts_by_kind
}

pub fn build_queue_unit_counts(planet: &Planet) -> BTreeMap<u8, u32> {
    let mut counts_by_kind = BTreeMap::<u8, u32>::new();
    for slot in 0..10 {
        let points = planet.build_queue[slot].points_owed;
        let kind_code = planet.build_queue[slot].kind.code();
        if points == 0 || kind_code == 0 {
            continue;
        }
        let kind = ProductionItemKind::from_code(kind_code);
        let qty = kind
            .build_cost()
            .map(|cost| points.div_ceil(cost))
            .unwrap_or(0);
        *counts_by_kind.entry(kind_code).or_default() += qty;
    }
    counts_by_kind
}

pub fn ordered_unit_count_entries(
    counts_by_kind: &BTreeMap<u8, u32>,
) -> Vec<(ProductionItemKind, u32)> {
    let mut ordered_kind_raws = vec![1, 2, 3, 4, 5, 6, 9, 8, 7];
    for kind in counts_by_kind.keys() {
        if !ordered_kind_raws.contains(kind) {
            ordered_kind_raws.push(*kind);
        }
    }
    ordered_kind_raws
        .into_iter()
        .filter_map(|kind| {
            let count = counts_by_kind.get(&kind).copied().unwrap_or(0);
            (count != 0).then_some((ProductionItemKind::from_code(kind), count))
        })
        .collect()
}

pub fn format_unit_counts(
    counts_by_kind: &BTreeMap<u8, u32>,
    style: CompactUnitSummaryStyle,
) -> String {
    let parts = ordered_unit_count_entries(counts_by_kind)
        .into_iter()
        .map(|(kind, count)| match style {
            CompactUnitSummaryStyle::JoinedCodes => {
                format!("{}{}", count, compact_unit_code(kind))
            }
            CompactUnitSummaryStyle::DashedCodes => {
                format!("{}-{}", count, compact_unit_code(kind))
            }
            CompactUnitSummaryStyle::Words => {
                format!("{} {}", count, unit_words(kind, count))
            }
        })
        .collect::<Vec<_>>();

    if parts.is_empty() {
        String::from("Nothing")
    } else {
        match style {
            CompactUnitSummaryStyle::JoinedCodes => parts.join(" "),
            CompactUnitSummaryStyle::DashedCodes | CompactUnitSummaryStyle::Words => {
                parts.join(", ")
            }
        }
    }
}

pub fn format_stardock_summary(planet: &Planet, style: CompactUnitSummaryStyle) -> String {
    format_unit_counts(&stardock_unit_counts(planet), style)
}

pub fn format_build_queue_summary(planet: &Planet, style: CompactUnitSummaryStyle) -> String {
    format_unit_counts(&build_queue_unit_counts(planet), style)
}

pub fn owned_orbit_presence(
    game_data: &CoreGameData,
    owner_empire_id: u8,
    coords: [u8; 2],
) -> OrbitPresenceSummary {
    OrbitPresenceSummary {
        fleets: game_data
            .fleets
            .iter()
            .filter(|fleet| {
                fleet.sector.to_array() == coords
                    && fleet.owner.0 == owner_empire_id
                    && has_any_force(fleet)
            })
            .count(),
        starbases: game_data
            .bases
            .iter()
            .filter(|base| {
                base.sector.to_array() == coords
                    && base.owner.0 == owner_empire_id
                    && base.base_id.0 != 0
            })
            .count(),
    }
}

pub fn format_owned_orbit_summary(summary: OrbitPresenceSummary) -> String {
    let mut parts = Vec::new();
    if summary.fleets > 0 {
        parts.push(format!(
            "{} {}",
            summary.fleets,
            if summary.fleets == 1 {
                "fleet"
            } else {
                "fleets"
            }
        ));
    }
    if summary.starbases > 0 {
        parts.push(format!(
            "{} {}",
            summary.starbases,
            if summary.starbases == 1 {
                "starbase"
            } else {
                "starbases"
            }
        ));
    }
    if parts.is_empty() {
        String::from("Nothing")
    } else {
        parts.join(", ")
    }
}

pub fn owned_planet_status(
    game_data: &CoreGameData,
    owner_empire_id: u8,
    planet_index_0_based: usize,
) -> OwnedPlanetStatus {
    let planet = &game_data.planets[planet_index_0_based];
    if planet.scorch_earth {
        return OwnedPlanetStatus::Scorched;
    }
    if is_homeworld_seed_ignoring_name(planet) {
        return OwnedPlanetStatus::Homeworld;
    }
    if game_data.planet_has_friendly_starbase(owner_empire_id, planet.sector.to_array()) {
        return OwnedPlanetStatus::StarbasePresent;
    }
    let present = present_production_points(planet);
    let potential = u16::from(planet.potential_production);
    if present == 0 && potential > 0 {
        OwnedPlanetStatus::FactoriesDestroyed
    } else if present < potential {
        OwnedPlanetStatus::FactoriesDamaged
    } else {
        OwnedPlanetStatus::FactoriesFunctional
    }
}

fn unit_words(kind: ProductionItemKind, count: u32) -> &'static str {
    match kind {
        ProductionItemKind::Destroyer => {
            if count == 1 {
                "destroyer"
            } else {
                "destroyers"
            }
        }
        ProductionItemKind::Cruiser => {
            if count == 1 {
                "cruiser"
            } else {
                "cruisers"
            }
        }
        ProductionItemKind::Battleship => {
            if count == 1 {
                "battleship"
            } else {
                "battleships"
            }
        }
        ProductionItemKind::Scout => {
            if count == 1 {
                "scout"
            } else {
                "scouts"
            }
        }
        ProductionItemKind::Transport => {
            if count == 1 {
                "troop transport"
            } else {
                "troop transports"
            }
        }
        ProductionItemKind::Etac => {
            if count == 1 {
                "ETAC"
            } else {
                "ETACs"
            }
        }
        ProductionItemKind::Army => {
            if count == 1 {
                "army"
            } else {
                "armies"
            }
        }
        ProductionItemKind::GroundBattery => {
            if count == 1 {
                "ground battery"
            } else {
                "ground batteries"
            }
        }
        ProductionItemKind::Starbase => {
            if count == 1 {
                "starbase"
            } else {
                "starbases"
            }
        }
        ProductionItemKind::Unknown(_) => "unknown units",
    }
}

/// One planet record for inspection output: sector, name, and whatever of
/// the build queue, owner, armies, and batteries is set.
pub fn planet_derived_summary(planet: &Planet) -> String {
    let text = planet.name.trim();
    let mut parts = vec![format!(
        "({},{}): {}",
        planet.sector.x, planet.sector.y, text
    )];
    if is_named_homeworld_seed(planet) {
        parts.push("likely_homeworld_seed".to_string());
    }
    let queued = build_queue_unit_counts(planet);
    if !queued.is_empty() {
        parts.push(format!(
            "build={}",
            format_unit_counts(&queued, CompactUnitSummaryStyle::JoinedCodes)
        ));
    }
    if planet.owner.0 != 0 {
        parts.push(format!(
            "owner_slot={} owner_status={:02x}",
            planet.owner.0, planet.years_under_owner
        ));
    }
    let armies = u8::try_from(planet.armies).unwrap_or(u8::MAX);
    if armies != 0 {
        parts.push(format!("armies={armies}"));
    }
    let batteries = u8::try_from(planet.ground_batteries).unwrap_or(u8::MAX);
    if batteries != 0 {
        parts.push(format!("batteries={batteries}"));
    }
    parts.join(" | ")
}
