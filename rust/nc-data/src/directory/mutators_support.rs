use super::*;
use crate::{Coords, has_combat_ships, is_support_only, set_mission_target, troop_transports};

pub(super) fn next_available_owned_fleet_local_slot(records: &[Fleet], owner_empire: u8) -> u16 {
    let mut owned_slots = records
        .iter()
        .filter(|fleet| fleet.owner.0 == owner_empire)
        .map(|fleet| fleet.fleet_number.0)
        .filter(|slot| *slot != 0)
        .collect::<Vec<_>>();
    owned_slots.sort_unstable();
    owned_slots.dedup();

    let mut next = 1u16;
    for slot in owned_slots {
        if slot == next {
            next = next.saturating_add(1);
        } else if slot > next {
            break;
        }
    }
    next
}

pub(super) fn next_available_global_fleet_id(records: &[Fleet]) -> u16 {
    let mut fleet_ids = records
        .iter()
        .map(|fleet| fleet.record.0)
        .filter(|fleet_id| *fleet_id != 0)
        .collect::<Vec<_>>();
    fleet_ids.sort_unstable();
    fleet_ids.dedup();

    let mut next = 1u16;
    for fleet_id in fleet_ids {
        if fleet_id == next {
            next = next.saturating_add(1);
        } else if fleet_id > next {
            break;
        }
    }
    next
}

pub(super) fn composition_invalidates_current_mission(
    record: &Fleet,
) -> Option<FleetOrderValidationError> {
    match record.order {
        Order::GuardStarbase | Order::GuardBlockadeWorld | Order::BombardWorld
            if !has_combat_ships(record) =>
        {
            Some(FleetOrderValidationError::MissingCombatShips)
        }
        Order::InvadeWorld if !has_combat_ships(record) => {
            Some(FleetOrderValidationError::MissingCombatShips)
        }
        Order::InvadeWorld
            if troop_transports(record) == 0 || record.ships.loaded_transports == 0 =>
        {
            Some(FleetOrderValidationError::MissingLoadedTroopTransports)
        }
        Order::BlitzWorld
            if troop_transports(record) == 0 || record.ships.loaded_transports == 0 =>
        {
            Some(FleetOrderValidationError::MissingLoadedTroopTransports)
        }
        Order::ScoutSector | Order::ScoutSolarSystem if record.ships.scouts == 0 => {
            Some(FleetOrderValidationError::MissingScoutShip)
        }
        Order::ColonizeWorld if record.ships.etacs == 0 => {
            Some(FleetOrderValidationError::MissingEtac)
        }
        _ => None,
    }
}

pub(super) fn set_fleet_to_local_hold(record: &mut Fleet) {
    let coords = record.sector.to_array();
    record.speed = 0;
    record.order = Order::HoldPosition;
    record.target = Coords::from_array(coords);
    set_mission_target(record, 0, 0);
}

/// ECGAME's donor recompute after a detach or transfer (`0x18333`): a fleet
/// left without a BB, CA, or DD gets ROE 0. Any other ROE stays.
pub(super) fn clear_roe_without_warships(record: &mut Fleet) {
    if !has_combat_ships(record) {
        record.rules_of_engagement = 0;
    }
}

pub(super) fn normalize_fleet_roe_for_composition(record: &mut Fleet) {
    if is_support_only(record) {
        record.rules_of_engagement = 0;
    } else if has_combat_ships(record) && record.rules_of_engagement == 0 {
        // Combat fleets should not default to Pacifist (ROE 0) upon commissioning.
        // Default to ROE 6 (Engage equal or inferior strength).
        record.rules_of_engagement = crate::NEW_FLEET_DEFAULT_ROE;
    }
}

pub(super) fn normalize_post_composition_fleet_state(
    record: &mut Fleet,
) -> Option<FleetOrderValidationError> {
    let invalidated = composition_invalidates_current_mission(record);
    if invalidated.is_some() {
        set_fleet_to_local_hold(record);
    }
    invalidated
}

/// ECGAME's new-fleet path (`0xeac1`, then the sort at `0x15d89`), shared by
/// detach and commission. The new fleet joins the end of its owner's chain:
/// its previous link names the last fleet, whose next link names it. ECGAME
/// then sorts the chain by fleet number, moving each record's contents while
/// the chain words (next, id, previous) stay at their positions. The
/// PLAYER.DAT head is set only for an empty chain; the tail is left as it
/// was. Returns the 1-based record index that holds the new fleet.
pub(super) fn append_fleet_to_owner_chain(
    records: &mut Vec<Fleet>,
    player: &mut Player,
    mut fleet: Fleet,
) -> usize {
    let owner_empire = fleet.owner.0;
    let mut chain = Vec::new();
    let mut fleet_id = player.fleets_first.0;
    while fleet_id != 0 && chain.len() < records.len() {
        let Some(idx) = records
            .iter()
            .position(|record| record.owner.0 == owner_empire && record.record.0 == fleet_id)
        else {
            break;
        };
        chain.push(idx);
        fleet_id = records[idx].next.0;
    }

    let new_fleet_id = fleet.record.0;
    let new_fleet_number = fleet.fleet_number.0;
    fleet.next.0 = 0;
    if let Some(&last) = chain.last() {
        fleet.prev.0 = records[last].record.0;
        records[last].next.0 = new_fleet_id;
    } else {
        fleet.prev.0 = 0;
        player.fleets_first.0 = new_fleet_id;
    }
    records.push(fleet);
    chain.push(records.len() - 1);

    let mut contents = chain
        .iter()
        .map(|&idx| records[idx].clone())
        .collect::<Vec<_>>();
    contents.sort_by_key(|fleet| fleet.fleet_number.0);
    for (&idx, mut content) in chain.iter().zip(contents) {
        content.next.0 = records[idx].next.0;
        content.record.0 = records[idx].record.0;
        content.prev.0 = records[idx].prev.0;
        records[idx] = content;
    }
    chain
        .into_iter()
        .find(|&idx| records[idx].fleet_number.0 == new_fleet_number)
        .map_or(records.len(), |idx| idx + 1)
}

/// A base at rest at `coords`, as ECGAME and ECMAINT create one: not moving,
/// target sector = current sector, target depth 1 (orbit). Chain links are
/// left at 0.
pub(super) fn build_guard_starbase_base_record(
    coords: [u8; 2],
    base_id: u16,
    escort_fleet_slot: u16,
    record_number: u16,
    owner_empire: u8,
    x_fraction: Real48,
    y_fraction: Real48,
    depth: Real48,
) -> Starbase {
    let mut record = Starbase::default();
    record.escort.0 = escort_fleet_slot;
    record.base_id.0 = base_id;
    record.owner.0 = owner_empire;
    record.record.0 = record_number;
    record.sector = Coords::from_array(coords);
    record.x_fraction = x_fraction;
    record.y_fraction = y_fraction;
    record.depth = depth;
    record.target = Coords::from_array(coords);
    record.target_depth = 1;
    record
}
