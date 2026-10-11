use super::*;

pub(super) fn next_available_owned_fleet_local_slot(records: &[Fleet], owner_empire: u8) -> u16 {
    let mut owned_slots = records
        .iter()
        .filter(|fleet| fleet.owner_empire_raw() == owner_empire)
        .map(Fleet::local_slot_word_raw)
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
        .map(Fleet::fleet_id_word_raw)
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

pub(super) fn set_fleet_to_local_hold(record: &mut Fleet) {
    let coords = record.current_location_coords_raw();
    record.set_current_speed(0);
    record.set_standing_order_kind(Order::HoldPosition);
    record.set_standing_order_target_coords_raw(coords);
    record.set_mission_target(0, 0);
}

/// ECGAME's donor recompute after a detach or transfer (`0x18333`): a fleet
/// left without a BB, CA, or DD gets ROE 0. Any other ROE stays.
pub(super) fn clear_roe_without_warships(record: &mut Fleet) {
    if !record.has_any_combat_ships() {
        record.set_rules_of_engagement(0);
    }
}

pub(super) fn normalize_fleet_roe_for_composition(record: &mut Fleet) {
    if record.is_support_only() {
        record.set_rules_of_engagement(0);
    } else if record.has_any_combat_ships() && record.rules_of_engagement() == 0 {
        // Combat fleets should not default to Pacifist (ROE 0) upon commissioning.
        // Default to ROE 6 (Engage equal or inferior strength).
        record.set_rules_of_engagement(crate::NEW_FLEET_DEFAULT_ROE);
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
    let owner_empire = fleet.owner_empire_raw();
    let mut chain = Vec::new();
    let mut fleet_id = player.fleet_chain_head_raw();
    while fleet_id != 0 && chain.len() < records.len() {
        let Some(idx) = records.iter().position(|record| {
            record.owner_empire_raw() == owner_empire && record.fleet_id_word_raw() == fleet_id
        }) else {
            break;
        };
        chain.push(idx);
        fleet_id = records[idx].next_fleet_link_word_raw();
    }

    let new_fleet_id = fleet.fleet_id_word_raw();
    let new_fleet_number = fleet.local_slot_word_raw();
    fleet.set_next_fleet_link_word_raw(0);
    if let Some(&last) = chain.last() {
        fleet.set_previous_fleet_id(records[last].fleet_id_word_raw());
        records[last].set_next_fleet_link_word_raw(new_fleet_id);
    } else {
        fleet.set_previous_fleet_id(0);
        player.set_fleet_chain_head_raw(new_fleet_id);
    }
    records.push(fleet);
    chain.push(records.len() - 1);

    let mut contents = chain
        .iter()
        .map(|&idx| records[idx].clone())
        .collect::<Vec<_>>();
    contents.sort_by_key(Fleet::local_slot_word_raw);
    for (&idx, mut content) in chain.iter().zip(contents) {
        content.set_next_fleet_link_word_raw(records[idx].next_fleet_link_word_raw());
        content.set_fleet_id_word_raw(records[idx].fleet_id_word_raw());
        content.set_previous_fleet_id(records[idx].previous_fleet_id());
        records[idx] = content;
    }
    chain
        .into_iter()
        .find(|&idx| records[idx].local_slot_word_raw() == new_fleet_number)
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
