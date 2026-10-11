use crate::{CoreGameData, production_item_kind_code};
use nc_data::{
    Coords, DiplomaticRelation, Order, ProductionItemKind, STARDOCK_SLOT_COUNT,
    build_queue_unit_counts, diplomatic_relation_toward, present_production_points,
    recompute_max_speed, stardock_slot_kind,
};

/// Autopilot orders for fleets, once a year before the planet pass and the
/// year's ticks, so movement acts on them this turn. Only human players with
/// autopilot enabled; rogue empires' fleet and starbase orders are t070.
pub fn process_autopilot_fleet_orders(
    game_data: &mut CoreGameData,
) -> Result<(), Box<dyn std::error::Error>> {
    for player_idx in 0..game_data.players.len() {
        let mode = game_data.players[player_idx].mode.code();
        let autopilot = u8::from(game_data.players[player_idx].autopilot);
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
    let player_count = game_data.conquest.player_count;
    for player_idx in 0..game_data.players.len() {
        let mode = game_data.players[player_idx].mode.code();
        let autopilot = u8::from(game_data.players[player_idx].autopilot);
        let ai_active = mode == 0xff || (mode == 0x01 && autopilot == 0x01);
        if !ai_active {
            continue;
        }

        let owner_slot = (player_idx + 1) as u8;
        let posture = assess_empire_posture(game_data, player_idx, owner_slot, player_count);

        game_data.players[player_idx].tax_rate = posture.tax_rate;

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
        .iter()
        .filter(|p| p.owner.0 == owner_slot)
        .fold((0u32, 0u32), |(pres, pot), p| {
            let present = present_production_points(p) as u32;
            let potential = u16::from(p.potential_production) as u32;
            (pres + present, pot + potential)
        });
    let dev_ratio = if total_potential == 0 {
        100
    } else {
        ((total_present * 100) / total_potential).min(100) as u8
    };

    // Count declared enemies.
    let player = &game_data.players[player_idx];
    let enemy_count = (1..=player_count)
        .filter(|&e| e != owner_slot)
        .filter(|&e| diplomatic_relation_toward(player, e) == Some(DiplomaticRelation::Enemy))
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

/// ECMAINT's orders pass for an autopilot empire (`24a3:3cc6`, `24a3:25f7`).
/// Each fleet on Hold gets the empire's best world as its target, picked by
/// travel distance per point of present production
/// ([`CoreGameData::nearest_productive_world`]), with target depth 1. A fleet
/// not yet in orbit there takes Seek Home at its maximum speed. One in orbit
/// with warships takes Guard/Blockade at rest, ROE 6 when it was 0, and then
/// guards an unescorted starbase at rest in the same orbit; one in orbit
/// without warships stays on Hold. Other orders are left alone.
fn autopilot_fleet_orders(game_data: &mut CoreGameData, owner_slot: u8) {
    for fleet_idx in 0..game_data.fleets.len() {
        let fleet = &game_data.fleets[fleet_idx];
        if fleet.owner.0 != owner_slot || fleet.order != Order::HoldPosition {
            continue;
        }
        let Some(world) = game_data.nearest_productive_world(fleet_idx + 1) else {
            continue;
        };

        let fleet = &mut game_data.fleets[fleet_idx];
        fleet.target = Coords::from_array(world);
        fleet.target_depth = 1;
        if fleet.sector.to_array() != world || fleet.depth.round() != 1 {
            fleet.order = Order::SeekHome;
            fleet.speed = fleet.max_speed;
            continue;
        }
        let ships = &fleet.ships;
        if ships.battleships + ships.cruisers + ships.destroyers == 0 {
            continue;
        }
        fleet.order = Order::GuardBlockadeWorld;
        fleet.speed = 0;
        recompute_max_speed(fleet);
        if fleet.rules_of_engagement == 0 {
            fleet.rules_of_engagement = 6;
        }
        guard_unescorted_starbase(game_data, fleet_idx);
    }
}

/// A fleet on Guard/Blockade takes the first starbase of its empire that
/// rests in orbit in its sector with no escort: the fleet's order becomes
/// Guard Starbase and the base's escort word the fleet's number.
fn guard_unescorted_starbase(game_data: &mut CoreGameData, fleet_idx: usize) {
    let fleet = &mut game_data.fleets[fleet_idx];
    let Some(base) = game_data.bases.iter_mut().find(|base| {
        base.owner == fleet.owner
            && base.base_id.0 != 0
            && base.escort.0 == 0
            && base.sector == fleet.sector
            && base.depth.round() == 1
            && !base.moving
            && base.target == base.sector
    }) else {
        return;
    };
    base.escort = fleet.fleet_number;
    fleet.order = Order::GuardStarbase;
    fleet.mission_target = base.base_id.0;
}

// ── Build queue ─────────────────────────────────────────────────────────

/// Commission completed starbases so they become active bases (and grant the
/// 5x build capacity) instead of sitting uncommissioned in stardock.
fn commission_stardock_starbases(
    game_data: &mut CoreGameData,
    owner_slot: u8,
) -> Result<(), Box<dyn std::error::Error>> {
    for planet_idx in 0..game_data.planets.len() {
        for slot in 0..STARDOCK_SLOT_COUNT {
            let planet = &game_data.planets[planet_idx];
            if planet.owner.0 == owner_slot
                && u16::try_from(planet.stardock[slot]).unwrap_or(u16::MAX) > 0
                && stardock_slot_kind(planet, slot) == ProductionItemKind::Starbase
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
        .iter()
        .filter(|b| b.owner.0 == owner_slot && b.base_id.0 != 0)
        .map(|b| b.sector.to_array())
        .collect();

    let threat_multiplier: u16 = if posture.enemy_count >= 3 {
        3
    } else if posture.enemy_count >= 2 {
        2
    } else {
        1
    };

    for planet_idx in 0..game_data.planets.len() {
        let planet = &game_data.planets[planet_idx];
        if planet.owner.0 != owner_slot {
            continue;
        }
        if planet.years_under_owner < 2 {
            continue;
        }

        let potential = u16::from(planet.potential_production);
        let present = present_production_points(planet);
        let coords = planet.sector.to_array();
        let current_armies = u8::try_from(planet.armies).unwrap_or(u8::MAX);
        let current_batteries = u8::try_from(planet.ground_batteries).unwrap_or(u8::MAX);
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
                .get(&production_item_kind_code(kind))
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
    }
    Ok(())
}
