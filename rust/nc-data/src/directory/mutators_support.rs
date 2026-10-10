use super::*;

pub(super) fn next_available_owned_fleet_local_slot(
    records: &[FleetRecord],
    owner_empire: u8,
) -> u16 {
    let mut owned_slots = records
        .iter()
        .filter(|fleet| fleet.owner_empire_raw() == owner_empire)
        .map(FleetRecord::local_slot_word_raw)
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

pub(super) fn next_available_global_fleet_id(records: &[FleetRecord]) -> u16 {
    let mut fleet_ids = records
        .iter()
        .map(FleetRecord::fleet_id_word_raw)
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
    record: &FleetRecord,
) -> Option<FleetOrderValidationError> {
    match record.standing_order_kind() {
        Order::GuardStarbase | Order::GuardBlockadeWorld | Order::BombardWorld
            if !record.has_any_combat_ships() =>
        {
            Some(FleetOrderValidationError::MissingCombatShips)
        }
        Order::InvadeWorld if !record.has_any_combat_ships() => {
            Some(FleetOrderValidationError::MissingCombatShips)
        }
        Order::InvadeWorld if record.troop_transport_count() == 0 || record.army_count() == 0 => {
            Some(FleetOrderValidationError::MissingLoadedTroopTransports)
        }
        Order::BlitzWorld if record.troop_transport_count() == 0 || record.army_count() == 0 => {
            Some(FleetOrderValidationError::MissingLoadedTroopTransports)
        }
        Order::ScoutSector | Order::ScoutSolarSystem if record.scout_count() == 0 => {
            Some(FleetOrderValidationError::MissingScoutShip)
        }
        Order::ColonizeWorld if record.etac_count() == 0 => {
            Some(FleetOrderValidationError::MissingEtac)
        }
        _ => None,
    }
}

pub(super) fn set_fleet_to_local_hold(record: &mut FleetRecord) {
    let coords = record.current_location_coords_raw();
    record.set_current_speed(0);
    record.set_standing_order_kind(Order::HoldPosition);
    record.set_standing_order_target_coords_raw(coords);
    record.set_mission_target(0, 0);
    normalize_fleet_roe_for_composition(record);
}

pub(super) fn normalize_fleet_roe_for_composition(record: &mut FleetRecord) {
    if record.is_support_only() {
        record.set_rules_of_engagement(0);
    } else if record.has_any_combat_ships() && record.rules_of_engagement() == 0 {
        // Combat fleets should not default to Pacifist (ROE 0) upon commissioning.
        // Default to ROE 6 (Engage equal or inferior strength).
        record.set_rules_of_engagement(6);
    }
}

pub(super) fn normalize_post_composition_fleet_state(
    record: &mut FleetRecord,
) -> Option<FleetOrderValidationError> {
    let invalidated = composition_invalidates_current_mission(record);
    if invalidated.is_some() {
        set_fleet_to_local_hold(record);
    } else {
        normalize_fleet_roe_for_composition(record);
    }
    invalidated
}

pub(super) fn rebuild_owner_fleet_chain(
    records: &mut [FleetRecord],
    player: &mut PlayerRecord,
    owner_empire: u8,
) {
    let mut owned = records
        .iter()
        .enumerate()
        .filter(|(_, fleet)| fleet.owner_empire_raw() == owner_empire)
        .map(|(idx, fleet)| (idx, fleet.local_slot_word_raw(), fleet.fleet_id_word_raw()))
        .collect::<Vec<_>>();
    owned.sort_unstable_by_key(|(_, local_slot, _)| *local_slot);

    player.set_fleet_chain_head_raw(owned.first().map(|(_, _, fleet_id)| *fleet_id).unwrap_or(0));

    for (position, (idx, _, _)) in owned.iter().enumerate() {
        let previous_id = position
            .checked_sub(1)
            .and_then(|prev| owned.get(prev))
            .map(|(_, _, fleet_id)| *fleet_id)
            .unwrap_or(0);
        let next_id = owned
            .get(position + 1)
            .map(|(_, _, fleet_id)| *fleet_id)
            .unwrap_or(0);
        records[*idx].set_previous_fleet_id(previous_id);
        records[*idx].set_next_fleet_link_word_raw(next_id);
    }
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
) -> BaseRecord {
    let mut record = BaseRecord::new_zeroed();
    record.set_escort_fleet_slot_raw(escort_fleet_slot);
    record.set_base_id_raw(base_id);
    record.set_owner_empire_raw(owner_empire);
    record.set_record_number_raw(record_number);
    record.set_coords_raw(coords);
    record.set_x_fraction(x_fraction);
    record.set_y_fraction(y_fraction);
    record.set_depth(depth);
    record.set_target_coords_raw(coords);
    record.set_target_depth_raw(1);
    record
}
