use crate::{CoreGameData, production_item_kind_raw};
use nc_data::fleet_motion_state::reset_motion_state_for_new_orders;
use nc_data::{
    DiplomaticRelation, Order, ProductionItemKind, STARDOCK_SLOT_COUNT, build_queue_unit_counts,
};

/// Autopilot fleet recall — runs BEFORE movement so idle fleets get SeekHome
/// orders that movement can act on this turn. Only applies to human players
/// with autopilot enabled (not rogue empires).
pub fn process_autopilot_fleet_orders(
    game_data: &mut CoreGameData,
) -> Result<(), Box<dyn std::error::Error>> {
    for player_idx in 0..game_data.player.records.len() {
        let mode = game_data.player.records[player_idx].owner_mode_raw();
        let autopilot = game_data.player.records[player_idx].autopilot_flag();
        if !(mode == 0x01 && autopilot == 0x01) {
            continue;
        }
        let owner_slot = (player_idx + 1) as u8;
        autopilot_fleet_orders(game_data, owner_slot);
    }
    Ok(())
}

/// Autopilot economic AI — runs AFTER economics for build queue and tax
/// management. Applies to rogue empires and human autopilot players.
pub fn process_autopilot_ai(
    game_data: &mut CoreGameData,
) -> Result<(), Box<dyn std::error::Error>> {
    let player_count = game_data.conquest.player_count();
    for player_idx in 0..game_data.player.records.len() {
        let mode = game_data.player.records[player_idx].owner_mode_raw();
        let autopilot = game_data.player.records[player_idx].autopilot_flag();
        let ai_active = mode == 0xff || (mode == 0x01 && autopilot == 0x01);
        if !ai_active {
            continue;
        }

        let owner_slot = (player_idx + 1) as u8;
        let posture = assess_empire_posture(game_data, player_idx, owner_slot, player_count);

        game_data.player.records[player_idx].set_tax_rate_raw(posture.tax_rate);

        commission_stardock_starbases(game_data, owner_slot)?;
        autopilot_build_queue(game_data, owner_slot, &posture)?;
    }

    Ok(())
}

// ── Empire assessment ───────────────────────────────────────────────────

struct EmpirePosture {
    tax_rate: u8,
    enemy_count: usize,
    #[allow(dead_code)]
    dev_ratio: u8,
}

fn assess_empire_posture(
    game_data: &CoreGameData,
    player_idx: usize,
    owner_slot: u8,
    player_count: u8,
) -> EmpirePosture {
    // Empire-wide development ratio: sum(present) / sum(potential).
    let (total_present, total_potential) = game_data
        .planets
        .records
        .iter()
        .filter(|p| p.owner_empire_slot_raw() == owner_slot)
        .fold((0u32, 0u32), |(pres, pot), p| {
            let present = p.present_production_points().unwrap_or(0) as u32;
            let potential = p.potential_production_points() as u32;
            (pres + present, pot + potential)
        });
    let dev_ratio = if total_potential == 0 {
        100
    } else {
        ((total_present * 100) / total_potential).min(100) as u8
    };

    // Count declared enemies.
    let player = &game_data.player.records[player_idx];
    let enemy_count = (1..=player_count)
        .filter(|&e| e != owner_slot)
        .filter(|&e| player.diplomatic_relation_toward(e) == Some(DiplomaticRelation::Enemy))
        .count();

    let high_threat = enemy_count >= 2;

    // Adaptive tax rate: low taxes grow fast, high taxes fund defense.
    // Never exceed 65 — the penalty destroys production.
    let tax_rate = match (dev_ratio, high_threat) {
        (0..=29, false) => 25,  // Heavy growth, starbase bonus maxed
        (0..=29, true) => 35,   // Growth priority but need some revenue
        (30..=59, false) => 40, // Balanced, good starbase bonus
        (30..=59, true) => 50,  // More revenue for defense
        (60..=84, _) => 55,     // Mostly developed, shift to revenue
        _ => 60,                // Near mature, maximize below penalty
    };

    EmpirePosture {
        tax_rate,
        enemy_count,
        dev_ratio,
    }
}

// ── Fleet orders ────────────────────────────────────────────────────────

fn autopilot_fleet_orders(game_data: &mut CoreGameData, owner_slot: u8) {
    let owned_planet_coords: Vec<[u8; 2]> = game_data
        .planets
        .records
        .iter()
        .filter(|p| p.owner_empire_slot_raw() == owner_slot)
        .map(|p| p.coords_raw())
        .collect();

    for fleet_idx in 0..game_data.fleets.records.len() {
        let fleet = &game_data.fleets.records[fleet_idx];
        if fleet.owner_empire_raw() != owner_slot || !fleet.has_any_force() {
            continue;
        }

        // Only recall truly idle fleets in deep space.
        if fleet.standing_order_kind() != Order::HoldPosition {
            continue;
        }
        let coords = fleet.current_location_coords_raw();
        if owned_planet_coords.iter().any(|pc| *pc == coords) {
            continue;
        }

        let fleet = &mut game_data.fleets.records[fleet_idx];
        let max_speed = fleet.max_speed();
        let target =
            nearest_owned_planet_coords(fleet.current_location_coords_raw(), &owned_planet_coords)
                .unwrap_or(fleet.current_location_coords_raw());
        reset_motion_state_for_new_orders(fleet);
        fleet.set_current_speed(max_speed);
        fleet.set_standing_order_kind(Order::SeekHome);
        fleet.set_standing_order_target_coords_raw(target);
    }
}

fn nearest_owned_planet_coords(from: [u8; 2], candidates: &[[u8; 2]]) -> Option<[u8; 2]> {
    candidates
        .iter()
        .min_by_key(|coords| {
            let dx = i16::from(coords[0]) - i16::from(from[0]);
            let dy = i16::from(coords[1]) - i16::from(from[1]);
            dx * dx + dy * dy
        })
        .copied()
}

// ── Build queue ─────────────────────────────────────────────────────────

/// Commission completed starbases so they become active bases (and grant the
/// 5x build capacity) instead of sitting uncommissioned in stardock.
fn commission_stardock_starbases(
    game_data: &mut CoreGameData,
    owner_slot: u8,
) -> Result<(), Box<dyn std::error::Error>> {
    for planet_idx in 0..game_data.planets.records.len() {
        for slot in 0..STARDOCK_SLOT_COUNT {
            let planet = &game_data.planets.records[planet_idx];
            if planet.owner_empire_slot_raw() == owner_slot
                && planet.stardock_count_raw(slot) > 0
                && planet.stardock_item_kind_current_known(slot) == ProductionItemKind::Starbase
            {
                game_data.commission_planet_stardock_slot(
                    usize::from(owner_slot),
                    planet_idx + 1,
                    slot,
                )?;
            }
        }
    }
    Ok(())
}

fn autopilot_build_queue(
    game_data: &mut CoreGameData,
    owner_slot: u8,
    posture: &EmpirePosture,
) -> Result<(), Box<dyn std::error::Error>> {
    let has_starbase_at: std::collections::HashSet<[u8; 2]> = game_data
        .bases
        .records
        .iter()
        .filter(|b| b.owner_empire_raw() == owner_slot && b.active_flag_raw() != 0)
        .map(|b| b.coords_raw())
        .collect();

    let threat_multiplier: u16 = if posture.enemy_count >= 3 {
        3
    } else if posture.enemy_count >= 2 {
        2
    } else {
        1
    };

    for planet_idx in 0..game_data.planets.records.len() {
        let planet = &game_data.planets.records[planet_idx];
        if planet.owner_empire_slot_raw() != owner_slot {
            continue;
        }
        if planet.conversion_countdown_raw() > 0 {
            continue;
        }

        let potential = planet.potential_production_points();
        let present = planet.present_production_points().unwrap_or(0);
        let coords = planet.coords_raw();
        let current_armies = planet.army_count_raw();
        let current_batteries = planet.ground_batteries_raw();
        let has_starbase = has_starbase_at.contains(&coords);

        // Per-planet development ratio.
        let planet_dev = if potential == 0 {
            100u8
        } else {
            ((present as u32 * 100) / potential as u32).min(100) as u8
        };

        // Targets scale with planet development and threat level.
        let (army_target, battery_target, want_starbase) = if planet_dev < 40 {
            // Low development: minimal garrison, save for growth.
            (2u8, 0u8, false)
        } else if planet_dev < 70 {
            // Medium: start building defenses.
            let armies = ((potential / 10).max(2) as u16 * threat_multiplier).min(255) as u8;
            let batteries = (1u16 * threat_multiplier).min(255) as u8;
            (armies, batteries, present >= 30)
        } else {
            // High development: full defense.
            let armies = ((potential / 10).max(2) as u16 * threat_multiplier).min(255) as u8;
            let batteries = ((potential / 20).max(1) as u16 * threat_multiplier).min(255) as u8;
            (armies, batteries, present >= 30)
        };

        // Keep existing orders (partial progress and the player's own orders)
        // and top up only what they do not already cover.
        let queued = build_queue_unit_counts(planet);
        let queued_qty = |kind: ProductionItemKind| {
            queued
                .get(&production_item_kind_raw(kind))
                .copied()
                .unwrap_or(0)
        };
        let has_starbase = has_starbase || queued_qty(ProductionItemKind::Starbase) > 0;
        let queued_armies = queued_qty(ProductionItemKind::Army);
        let queued_batteries = queued_qty(ProductionItemKind::GroundBattery);

        // Budget: min(treasury, build capacity) less points already queued.
        // Economics has already credited this year's revenue to the treasury.
        let budget = game_data.planet_build_points_left(planet_idx + 1)?;
        let mut spent = 0u32;

        // Priority 1: Armies to target.
        if current_armies < army_target {
            let needed = u32::from(army_target - current_armies).saturating_sub(queued_armies);
            let cost_per = ProductionItemKind::Army.build_cost().unwrap_or(2);
            let affordable = budget.saturating_sub(spent) / cost_per;
            let to_build = needed.min(affordable);
            if to_build > 0
                && game_data
                    .append_planet_build_order(planet_idx + 1, to_build * cost_per, 8)
                    .is_ok()
            {
                spent += to_build * cost_per;
            }
        }

        // Priority 2: Ground batteries to target.
        if battery_target > 0 && current_batteries < battery_target {
            let needed =
                u32::from(battery_target - current_batteries).saturating_sub(queued_batteries);
            let cost_per = ProductionItemKind::GroundBattery.build_cost().unwrap_or(20);
            let affordable = budget.saturating_sub(spent) / cost_per;
            let to_build = needed.min(affordable);
            if to_build > 0
                && game_data
                    .append_planet_build_order(planet_idx + 1, to_build * cost_per, 7)
                    .is_ok()
            {
                spent += to_build * cost_per;
            }
        }

        // Priority 3: Starbase if planet qualifies and has none built or queued.
        if want_starbase && !has_starbase {
            let cost = ProductionItemKind::Starbase.build_cost().unwrap_or(50);
            if budget.saturating_sub(spent) >= cost {
                let _ = game_data.append_planet_build_order(planet_idx + 1, cost, 9);
            }
        }

        // Economy marker tracks the AI's chosen tax rate.
        game_data.planets.records[planet_idx].set_economy_marker_raw(posture.tax_rate);
    }
    Ok(())
}
