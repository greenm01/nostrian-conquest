use std::collections::BTreeMap;

use nc_data::{
    CommissionFleetDraft, CoreGameData, EmpirePlanetEconomyRow, GameStateMutationError,
    PlanetRecord, ProductionItemKind, STARDOCK_SLOT_COUNT, build_queue_unit_counts,
};

use crate::{BUILD_UNITS, build_quantity_from_points, build_unit_spec_by_kind, max_quantity};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlanetBuildViewStats {
    pub committed_points: u32,
    pub budget: u32,
    pub treasury_left: u32,
    pub points_left: u32,
    pub building_count: u32,
    pub docked_count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlanetBuildOrderLine {
    pub kind: ProductionItemKind,
    pub points_remaining: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlanetBuildListEntry {
    pub kind: ProductionItemKind,
    pub queue_qty: u32,
    pub points: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlanetBuildSpecifyEntry {
    pub number: u8,
    pub kind: ProductionItemKind,
    pub label: &'static str,
    pub cost: u32,
    pub queued_qty: u32,
    pub selectable: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlanetCommissionSlotEntry {
    pub slot_0_based: usize,
    pub kind: ProductionItemKind,
    pub qty: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlanetCommissionDraftEntry {
    pub direct_slot_0_based: Option<usize>,
    pub kind: ProductionItemKind,
    pub remaining_qty: u16,
    pub fleet_qty: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanetCommissionDraftState {
    pub draft_slots: Vec<usize>,
    pub rows: Vec<PlanetCommissionDraftEntry>,
}

pub fn production_item_kind_raw(kind: ProductionItemKind) -> u8 {
    kind.to_raw()
}

pub fn planet_build_committed_points(planet: &PlanetRecord) -> u32 {
    (0..10)
        .map(|slot| u32::from(planet.build_count_raw(slot)))
        .sum::<u32>()
}

pub fn planet_building_unit_count(planet: &PlanetRecord) -> u32 {
    (0..10)
        .map(|slot| {
            let points = u32::from(planet.build_count_raw(slot));
            let kind = ProductionItemKind::from_raw(planet.build_kind_raw(slot));
            build_quantity_from_points(kind, points)
        })
        .sum::<u32>()
}

pub fn planet_docked_unit_count(planet: &PlanetRecord) -> u32 {
    (0..STARDOCK_SLOT_COUNT)
        .map(|slot| u32::from(planet.stardock_count_raw(slot)))
        .sum::<u32>()
}

pub fn planet_build_orders(planet: &PlanetRecord) -> Vec<PlanetBuildOrderLine> {
    (0..10)
        .filter_map(|slot| {
            let points = planet.build_count_raw(slot);
            let kind_raw = planet.build_kind_raw(slot);
            if points == 0 || kind_raw == 0 {
                None
            } else {
                Some(PlanetBuildOrderLine {
                    kind: ProductionItemKind::from_raw(kind_raw),
                    points_remaining: points,
                })
            }
        })
        .collect()
}

pub fn planet_build_view(
    game_data: &CoreGameData,
    row: &EmpirePlanetEconomyRow,
) -> Result<PlanetBuildViewStats, GameStateMutationError> {
    let planet = game_data
        .planets
        .records
        .get(row.planet_record_index_1_based - 1)
        .ok_or(GameStateMutationError::MissingPlanetRecord {
            index_1_based: row.planet_record_index_1_based,
        })?;
    let committed_points = planet_build_committed_points(planet);
    let available_points = u32::from(row.build_capacity).min(row.stored_production_points);
    Ok(PlanetBuildViewStats {
        committed_points,
        budget: available_points,
        treasury_left: row
            .stored_production_points
            .saturating_sub(committed_points),
        points_left: available_points.saturating_sub(committed_points),
        building_count: planet_building_unit_count(planet),
        docked_count: planet_docked_unit_count(planet),
    })
}

pub fn planet_build_max_quantity(
    game_data: &CoreGameData,
    row: &EmpirePlanetEconomyRow,
    kind: ProductionItemKind,
) -> Result<u32, GameStateMutationError> {
    let view = planet_build_view(game_data, row)?;
    let Some(unit) = build_unit_spec_by_kind(kind) else {
        return Ok(0);
    };
    let queue_capacity =
        game_data.planet_additional_build_points_capacity(row.planet_record_index_1_based, kind)?;
    let mut max_qty = max_quantity(view.points_left.min(queue_capacity), unit.cost);
    match kind {
        ProductionItemKind::Army => {
            let free = game_data.planet_free_army_capacity(row.planet_record_index_1_based)?;
            max_qty = max_qty.min(u32::from(free));
        }
        ProductionItemKind::GroundBattery => {
            let free =
                game_data.planet_free_ground_battery_capacity(row.planet_record_index_1_based)?;
            max_qty = max_qty.min(u32::from(free));
        }
        _ => {}
    }
    Ok(max_qty)
}

pub fn planet_has_any_buildable_unit(
    game_data: &CoreGameData,
    row: &EmpirePlanetEconomyRow,
) -> Result<bool, GameStateMutationError> {
    for unit in BUILD_UNITS {
        if planet_build_max_quantity(game_data, row, unit.kind)? > 0 {
            return Ok(true);
        }
    }
    Ok(false)
}

pub fn planet_build_unavailable_message(
    points_left: u32,
    kind: ProductionItemKind,
) -> &'static str {
    if points_left == 0 {
        return "No points are available to spend.";
    }
    match kind {
        ProductionItemKind::Army => "Planet already has the maximum 255 armies.",
        ProductionItemKind::GroundBattery => "Planet already has the maximum 255 ground batteries.",
        _ => "No points are available to spend.",
    }
}

pub fn planet_build_list_entries(planet: &PlanetRecord) -> Vec<PlanetBuildListEntry> {
    let queue_qty_by_kind = build_queue_unit_counts(planet);
    ordered_kind_raws(&queue_qty_by_kind)
        .into_iter()
        .filter_map(|kind_raw| {
            let queue_qty = queue_qty_by_kind.get(&kind_raw).copied().unwrap_or(0);
            if queue_qty == 0 {
                return None;
            }
            let kind = ProductionItemKind::from_raw(kind_raw);
            let points = build_unit_spec_by_kind(kind).map(|u| u.cost).unwrap_or(0);
            Some(PlanetBuildListEntry {
                kind,
                queue_qty,
                points,
            })
        })
        .collect()
}

pub fn planet_build_specify_entries(
    points_left: u32,
    orders: &[PlanetBuildOrderLine],
) -> Vec<PlanetBuildSpecifyEntry> {
    BUILD_UNITS
        .iter()
        .copied()
        .map(|unit| {
            let queued_qty = if unit.cost == 0 {
                0
            } else {
                orders
                    .iter()
                    .filter(|order| order.kind == unit.kind)
                    .map(|order| u32::from(order.points_remaining) / unit.cost)
                    .sum()
            };
            PlanetBuildSpecifyEntry {
                number: unit.number,
                kind: unit.kind,
                label: unit.label,
                cost: unit.cost,
                queued_qty,
                selectable: max_quantity(points_left, unit.cost) > 0,
            }
        })
        .collect()
}

pub fn planet_build_max_selectable_unit_number(entries: &[PlanetBuildSpecifyEntry]) -> u8 {
    entries
        .iter()
        .filter(|entry| entry.selectable)
        .map(|entry| entry.number)
        .max()
        .unwrap_or(0)
}

pub fn planet_commission_slot_entries(planet: &PlanetRecord) -> Vec<PlanetCommissionSlotEntry> {
    (0..STARDOCK_SLOT_COUNT)
        .filter_map(|slot| {
            let qty = u32::from(planet.stardock_count_raw(slot));
            let kind = ProductionItemKind::from_raw(planet.stardock_kind_raw(slot));
            (qty != 0 && kind.requires_stardock()).then_some(PlanetCommissionSlotEntry {
                slot_0_based: slot,
                kind,
                qty,
            })
        })
        .collect()
}

pub fn planet_commission_draft_state(
    rows: &[PlanetCommissionSlotEntry],
) -> PlanetCommissionDraftState {
    let mut totals = BTreeMap::<u8, (ProductionItemKind, u16)>::new();
    let mut starbase_rows = Vec::new();
    let mut draft_slots = Vec::new();

    for row in rows {
        if row.kind == ProductionItemKind::Starbase {
            starbase_rows.push(PlanetCommissionDraftEntry {
                direct_slot_0_based: Some(row.slot_0_based),
                kind: row.kind,
                remaining_qty: row.qty.min(u32::from(u16::MAX)) as u16,
                fleet_qty: 0,
            });
            continue;
        }
        if !is_commission_ship_kind(row.kind) {
            continue;
        }
        draft_slots.push(row.slot_0_based);
        let kind_raw = production_item_kind_raw(row.kind);
        let entry = totals.entry(kind_raw).or_insert((row.kind, 0));
        entry.1 = entry
            .1
            .saturating_add(row.qty.min(u32::from(u16::MAX)) as u16);
    }

    let mut draft_rows = Vec::new();
    for kind in [
        ProductionItemKind::Destroyer,
        ProductionItemKind::Cruiser,
        ProductionItemKind::Battleship,
        ProductionItemKind::Scout,
        ProductionItemKind::Transport,
        ProductionItemKind::Etac,
    ] {
        let kind_raw = production_item_kind_raw(kind);
        let Some((kind, qty)) = totals.remove(&kind_raw) else {
            continue;
        };
        draft_rows.push(PlanetCommissionDraftEntry {
            direct_slot_0_based: None,
            kind,
            remaining_qty: qty,
            fleet_qty: 0,
        });
    }
    draft_rows.extend(starbase_rows);

    PlanetCommissionDraftState {
        draft_slots,
        rows: draft_rows,
    }
}

pub fn commission_fleet_draft_from_entries(
    rows: &[PlanetCommissionDraftEntry],
) -> Result<CommissionFleetDraft, &'static str> {
    let mut draft = CommissionFleetDraft::default();
    for row in rows {
        if !accepts_commission_fleet_qty(*row) {
            continue;
        }
        match row.kind {
            ProductionItemKind::Destroyer => draft.destroyers = row.fleet_qty,
            ProductionItemKind::Cruiser => draft.cruisers = row.fleet_qty,
            ProductionItemKind::Battleship => draft.battleships = row.fleet_qty,
            ProductionItemKind::Scout => draft.scouts = row.fleet_qty,
            ProductionItemKind::Transport => draft.transports = row.fleet_qty,
            ProductionItemKind::Etac => draft.etacs = row.fleet_qty,
            _ => return Err("invalid ship kind in commission draft"),
        }
    }
    Ok(draft)
}

fn ordered_kind_raws(counts_by_kind: &BTreeMap<u8, u32>) -> Vec<u8> {
    let mut ordered_kind_raws = vec![1, 2, 3, 4, 5, 6, 9, 8, 7];
    for kind_raw in counts_by_kind.keys() {
        if !ordered_kind_raws.contains(kind_raw) {
            ordered_kind_raws.push(*kind_raw);
        }
    }
    ordered_kind_raws
}

fn is_commission_ship_kind(kind: ProductionItemKind) -> bool {
    matches!(
        kind,
        ProductionItemKind::Destroyer
            | ProductionItemKind::Cruiser
            | ProductionItemKind::Battleship
            | ProductionItemKind::Scout
            | ProductionItemKind::Transport
            | ProductionItemKind::Etac
    )
}

fn accepts_commission_fleet_qty(row: PlanetCommissionDraftEntry) -> bool {
    row.direct_slot_0_based.is_none() && is_commission_ship_kind(row.kind)
}
