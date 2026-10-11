use crate::{
    AssaultReportEvent, BombardEvent, CoreGameData, FleetDestroyedEvent, Mission, MissionEvent,
    MissionOutcome, Order, PlanetIntelEvent, PlanetIntelSource, PlanetOwnershipChangeEvent,
    ProductionItemKind, STARDOCK_SLOT_COUNT, ShipLosses,
};

use super::exchange::{
    COMBAT_GUARDRAIL_MAX_ROUNDS, COMBAT_KIND_BLITZ_COVER, COMBAT_KIND_BLITZ_GROUND,
    COMBAT_KIND_BLITZ_LANDING, COMBAT_KIND_BOMBARD, COMBAT_KIND_GROUND,
    COMBAT_KIND_INVASION_SOFTEN, COMBAT_KIND_INVASION_SUPPRESSION, ExchangeResolution,
    GROUND_AS_ARMY, GROUND_AS_BATTERY, GROUND_DS_ARMY, GROUND_DS_BATTERY, apply_hits_to_fleet,
    apply_landing_fire_to_fleet, resolve_ground_exchange, resolve_orbital_exchange,
    scalar_hits_with_critical,
};
use super::reporting::{preferred_reporting_fleet_number, push_planet_intel};
use super::retreat::set_fleet_to_hold_current_position;
use super::state::{
    FleetCombatState, IDX_BB, IDX_CA, IDX_DD, IDX_ET, IDX_SB, IDX_SC, IDX_TT, fleet_class_ds,
    fleet_state_from_records, planet_idx_at_coords, ship_counts_from_state,
    ship_losses_from_states,
};

#[derive(Debug, Default)]
pub(crate) struct AssaultEvents {
    pub bombard_events: Vec<BombardEvent>,
    pub fleet_destroyed_events: Vec<FleetDestroyedEvent>,
    pub assault_report_events: Vec<AssaultReportEvent>,
    pub planet_intel_events: Vec<PlanetIntelEvent>,
    pub ownership_change_events: Vec<PlanetOwnershipChangeEvent>,
    pub mission_events: Vec<MissionEvent>,
}

impl AssaultEvents {
    pub(crate) fn append(&mut self, mut other: Self) {
        self.bombard_events.append(&mut other.bombard_events);
        self.fleet_destroyed_events
            .append(&mut other.fleet_destroyed_events);
        self.assault_report_events
            .append(&mut other.assault_report_events);
        self.planet_intel_events
            .append(&mut other.planet_intel_events);
        self.ownership_change_events
            .append(&mut other.ownership_change_events);
        self.mission_events.append(&mut other.mission_events);
    }

    /// Date every event to the tick the strike was resolved in.
    pub(crate) fn stamp(&mut self, tick: u8) {
        for event in &mut self.bombard_events {
            event.stardate_week = Some(tick);
        }
        for event in &mut self.fleet_destroyed_events {
            event.stardate_week = Some(tick);
        }
        for event in &mut self.assault_report_events {
            event.stardate_week = Some(tick);
        }
        for event in &mut self.planet_intel_events {
            event.stardate_week = Some(tick);
        }
        for event in &mut self.ownership_change_events {
            event.stardate_week = Some(tick);
        }
        for event in &mut self.mission_events {
            event.stardate_week = Some(tick);
        }
    }
}

fn clear_arrival_and_hold(game_data: &mut CoreGameData, fleet_indices: &[usize]) {
    for &idx in fleet_indices {
        set_fleet_to_hold_current_position(&mut game_data.fleets[idx]);
    }
}

fn assault_force_destroyed(game_data: &CoreGameData, fleet_indices: &[usize]) -> bool {
    !fleet_indices.is_empty()
        && fleet_indices.iter().all(|&idx| {
            game_data
                .fleets
                .get(idx)
                .is_none_or(|fleet| !fleet.has_any_force())
        })
}

fn push_assault_destroyed_fleet_event(
    events: &mut AssaultEvents,
    game_data: &CoreGameData,
    fleet_indices: &[usize],
    coords: [u8; 2],
    friendly_initial: ShipLosses,
    friendly_loaded_armies_initial: u32,
    enemy_ground_batteries_initial: u8,
    enemy_ground_armies_initial: u8,
    enemy_ground_battery_losses: u8,
    enemy_ground_army_losses: u8,
    primary_enemy_empire_raw: u8,
) {
    events.fleet_destroyed_events.push(FleetDestroyedEvent {
        reporting_empire_raw: game_data.fleets[fleet_indices[0]].owner_empire_raw(),
        fleet_number: preferred_reporting_fleet_number(game_data, fleet_indices).unwrap_or(0),
        coords,
        was_intercepting: false,
        friendly_initial,
        friendly_loaded_armies_initial,
        enemy_initial: ShipLosses::default(),
        enemy_initial_starbases: 0,
        enemy_loaded_armies_initial: 0,
        enemy_ground_batteries_initial,
        enemy_ground_armies_initial,
        enemy_losses: ShipLosses::default(),
        enemy_starbases_destroyed: 0,
        enemy_ground_battery_losses,
        enemy_ground_army_losses,
        primary_enemy_empire_raw: (primary_enemy_empire_raw != 0)
            .then_some(primary_enemy_empire_raw),
        primary_enemy_fleet_number: None,
        stardate_week: None,
    });
}

fn fleet_still_ready_for_assault(game_data: &CoreGameData, fleet_idx: usize, order: Order) -> bool {
    let Some(fleet) = game_data.fleets.get(fleet_idx) else {
        return false;
    };
    let target_coords = fleet.standing_order_target_coords_raw();
    if fleet.standing_order_kind() != order || fleet.current_location_coords_raw() != target_coords
    {
        return false;
    }
    game_data
        .validate_fleet_order_payload(fleet_idx + 1, order.to_raw(), target_coords, None, None)
        .is_ok()
}

/// Hits needed to destroy one stardock item: its combat defense strength.
fn stardock_item_ds(kind: ProductionItemKind) -> u32 {
    match kind {
        ProductionItemKind::Destroyer => fleet_class_ds(IDX_DD),
        ProductionItemKind::Cruiser => fleet_class_ds(IDX_CA),
        ProductionItemKind::Battleship => fleet_class_ds(IDX_BB),
        ProductionItemKind::Scout => fleet_class_ds(IDX_SC),
        ProductionItemKind::Transport => fleet_class_ds(IDX_TT),
        ProductionItemKind::Etac => fleet_class_ds(IDX_ET),
        ProductionItemKind::Starbase => fleet_class_ds(IDX_SB),
        ProductionItemKind::GroundBattery => GROUND_DS_BATTERY,
        ProductionItemKind::Army => GROUND_DS_ARMY,
        ProductionItemKind::Unknown(_) => 1,
    }
}

/// Each docked item costs its own DS in hits. Hits too few to destroy the
/// next item move on to the following slot; partial hits never carry over.
fn reduce_stardock(planet: &mut nc_data::Planet, mut hits: u32) -> u32 {
    for slot in 0..STARDOCK_SLOT_COUNT {
        if hits == 0 {
            break;
        }
        let count = planet.stardock_count_raw(slot) as u32;
        if count == 0 {
            continue;
        }
        let ds = stardock_item_ds(ProductionItemKind::from_raw(planet.stardock_kind_raw(slot)));
        let destroyed = (hits / ds).min(count) as u16;
        if destroyed == 0 {
            continue;
        }
        planet.set_stardock_count_raw(
            slot,
            planet.stardock_count_raw(slot).saturating_sub(destroyed),
        );
        if planet.stardock_count_raw(slot) == 0 {}
        hits -= destroyed as u32 * ds;
    }
    hits
}

/// Each ground battery destroyed costs its DS (20 hits). Returns the hits
/// left over.
fn apply_battery_hits(planet: &mut nc_data::Planet, hits: u32) -> u32 {
    let battery_loss = (hits / GROUND_DS_BATTERY).min(planet.ground_batteries_raw() as u32);
    planet.set_ground_batteries_raw(planet.ground_batteries_raw() - battery_loss as u8);
    hits - battery_loss * GROUND_DS_BATTERY
}

fn stardock_summary(planet: &nc_data::Planet) -> nc_data::EmpireUnitSummary {
    let mut summary = nc_data::EmpireUnitSummary::default();
    for slot in 0..STARDOCK_SLOT_COUNT {
        let count = u32::from(planet.stardock_count_raw(slot));
        if count == 0 {
            continue;
        }
        match ProductionItemKind::from_raw(planet.stardock_kind_raw(slot)) {
            ProductionItemKind::Destroyer => summary.destroyers += count,
            ProductionItemKind::Cruiser => summary.cruisers += count,
            ProductionItemKind::Battleship => summary.battleships += count,
            ProductionItemKind::Scout => summary.scouts += count,
            ProductionItemKind::Transport => summary.transports += count,
            ProductionItemKind::Etac => summary.etacs += count,
            ProductionItemKind::Starbase => summary.starbases += count,
            ProductionItemKind::Army => summary.armies += count,
            ProductionItemKind::GroundBattery => summary.ground_batteries += count,
            ProductionItemKind::Unknown(_) => {}
        }
    }
    summary
}

fn stardock_summary_diff(
    before: nc_data::EmpireUnitSummary,
    after: nc_data::EmpireUnitSummary,
) -> nc_data::EmpireUnitSummary {
    nc_data::EmpireUnitSummary {
        destroyers: before.destroyers.saturating_sub(after.destroyers),
        cruisers: before.cruisers.saturating_sub(after.cruisers),
        battleships: before.battleships.saturating_sub(after.battleships),
        scouts: before.scouts.saturating_sub(after.scouts),
        transports: before.transports.saturating_sub(after.transports),
        etacs: before.etacs.saturating_sub(after.etacs),
        starbases: before.starbases.saturating_sub(after.starbases),
        armies: before.armies.saturating_sub(after.armies),
        ground_batteries: before
            .ground_batteries
            .saturating_sub(after.ground_batteries),
    }
}

/// Suppression damage: hits only go through stardock and batteries.
/// Armies, stored goods, and factories are shielded.
fn apply_planet_suppression_damage(planet: &mut nc_data::Planet, hits: u32) {
    let hits = reduce_stardock(planet, hits);
    apply_battery_hits(planet, hits);
}

fn apply_capped_invasion_soften_damage(
    planet: &mut nc_data::Planet,
    hits: u32,
    initial_armies: u8,
) -> u8 {
    let max_soften_losses = u32::from(initial_armies / 2);
    let army_loss = (hits / GROUND_DS_ARMY)
        .min(planet.army_count_raw() as u32)
        .min(max_soften_losses) as u8;
    planet.set_army_count_raw(planet.army_count_raw().saturating_sub(army_loss));
    army_loss
}

/// Full bombardment cascade: stardock -> batteries -> armies -> stored goods -> factories.
/// Units cost their DS in hits; hits reaching goods and factories are scaled
/// down from the 10x combat scale before the per-unit conversion.
fn apply_planet_bombardment_damage(planet: &mut nc_data::Planet, hits: u32) {
    let hits = reduce_stardock(planet, hits);
    let mut hits = apply_battery_hits(planet, hits);

    let army_loss = (hits / GROUND_DS_ARMY).min(planet.army_count_raw() as u32);
    planet.set_army_count_raw(planet.army_count_raw() - army_loss as u8);
    hits -= army_loss * GROUND_DS_ARMY;

    let hits = hits / 10;
    if hits > 0 {
        let goods_loss = hits.saturating_mul(100);
        planet.set_stored_production_points(
            planet.stored_production_points().saturating_sub(goods_loss),
        );
    }
    if hits > 0 {
        let factories = planet.present_production_points().unwrap_or(0);
        let loss = hits.min(u32::from(factories)) as u16;
        planet.set_present_production_points(factories.saturating_sub(loss));
    }
}

fn bombard_attack_as(state: &FleetCombatState) -> u32 {
    let mut as_total = 0;
    // Healthy ships (AS_DD: 10, AS_CA: 30, AS_BB: 90)
    // Bombard modifiers (0.5x, 1.0x, 1.5x)
    as_total += state.nominal_count(IDX_DD) * 5 + state.crippled[IDX_DD] * 2;
    as_total += state.nominal_count(IDX_CA) * 30 + state.crippled[IDX_CA] * 15;
    as_total += state.nominal_count(IDX_BB) * 135 + state.crippled[IDX_BB] * 67;
    as_total
}

fn blitz_cover_exchange(
    campaign_seed: u64,
    game_year: u16,
    coords: [u8; 2],
    acting_empire: u8,
    target_empire: u8,
    state: &FleetCombatState,
    battery_as: u32,
) -> ExchangeResolution {
    let combat_ships = state.counts[IDX_DD] + state.counts[IDX_CA] + state.counts[IDX_BB];
    if combat_ships == 0 {
        return ExchangeResolution {
            roll: 0,
            critical: false,
            hits: 0,
        };
    }

    resolve_ground_exchange(
        campaign_seed,
        game_year,
        coords,
        COMBAT_KIND_BLITZ_COVER,
        1,
        acting_empire,
        target_empire,
        bombard_attack_as(state).max(1),
        battery_as.max(1),
        0,
    )
}

/// Surviving batteries fire on the descending transports: one CRT exchange of
/// battery AS against the escorts' bombardment AS, applied as focus fire.
#[allow(clippy::too_many_arguments)]
fn apply_blitz_landing_fire(
    game_data: &mut CoreGameData,
    campaign_seed: u64,
    game_year: u16,
    coords: [u8; 2],
    defender_empire: u8,
    attacker_empire: u8,
    fleet_indices: &[usize],
    surviving_batteries: u8,
) -> (u32, ShipLosses) {
    if surviving_batteries == 0 {
        return (0, ShipLosses::default());
    }
    let before = fleet_state_from_records(game_data, fleet_indices, 0);
    let landing_exchange = resolve_orbital_exchange(
        campaign_seed,
        game_year,
        coords,
        COMBAT_KIND_BLITZ_LANDING,
        1,
        defender_empire,
        attacker_empire,
        u32::from(surviving_batteries) * GROUND_AS_BATTERY,
        bombard_attack_as(&before).max(1),
        false,
    );
    let mut after = before.clone();
    apply_landing_fire_to_fleet(&mut after, landing_exchange.hits, landing_exchange.critical);

    let loaded_armies = |game_data: &CoreGameData| -> u32 {
        fleet_indices
            .iter()
            .map(|&idx| u32::from(game_data.fleets[idx].army_count()))
            .sum()
    };
    let armies_before = loaded_armies(game_data);
    super::fleet_battle::distribute_fleet_losses(game_data, fleet_indices, &before, &after);
    let armies_after = loaded_armies(game_data);

    (
        armies_before.saturating_sub(armies_after),
        ship_losses_from_states(&before, &after),
    )
}

fn resolve_ground_battle_to_destruction(
    campaign_seed: u64,
    game_year: u16,
    coords: [u8; 2],
    combat_kind: u64,
    attacker_empire: u8,
    defender_empire: u8,
    attacker_armies: u32,
    defender_armies: u32,
    defender_bonus_shift: i32,
) -> (u32, u32) {
    let mut attacker_survivors = attacker_armies;
    let mut defender_survivors = defender_armies;

    for round in 1..=COMBAT_GUARDRAIL_MAX_ROUNDS {
        if attacker_survivors == 0 || defender_survivors == 0 {
            return (attacker_survivors, defender_survivors);
        }

        let attacker_before_round = attacker_survivors;
        let defender_before_round = defender_survivors;

        let attacker_ground = resolve_ground_exchange(
            campaign_seed,
            game_year,
            coords,
            combat_kind,
            round,
            attacker_empire,
            defender_empire,
            attacker_survivors,
            defender_survivors,
            0,
        );
        let defender_ground = resolve_ground_exchange(
            campaign_seed,
            game_year,
            coords,
            combat_kind,
            round,
            defender_empire,
            attacker_empire,
            defender_survivors,
            attacker_survivors,
            defender_bonus_shift,
        );

        let attacker_losses = scalar_hits_with_critical(defender_ground).min(attacker_survivors);
        let defender_losses = scalar_hits_with_critical(attacker_ground).min(defender_survivors);

        attacker_survivors = attacker_survivors.saturating_sub(attacker_losses);
        defender_survivors = defender_survivors.saturating_sub(defender_losses);

        // Ground combat must resolve with one side broken and the other still
        // holding the field. If a simultaneous exchange would annihilate both
        // sides, the larger force entering that exchange survives with a
        // remnant army; exact equality favors the defender.
        if attacker_survivors == 0 && defender_survivors == 0 {
            if attacker_before_round > defender_before_round {
                attacker_survivors = 1;
            } else {
                defender_survivors = 1;
            }
            return (attacker_survivors, defender_survivors);
        }
    }

    settle_ground_battle_at_cap(attacker_survivors, defender_survivors)
}

/// Ground combat cannot end in a draw. If the round cap is reached with both
/// sides standing, the larger remaining force holds the field and the other is
/// broken; exact equality favors the defender.
fn settle_ground_battle_at_cap(attacker_survivors: u32, defender_survivors: u32) -> (u32, u32) {
    if attacker_survivors > defender_survivors {
        (attacker_survivors, 0)
    } else {
        (0, defender_survivors)
    }
}

/// Resolve the planetary attack of fleet `fleet_idx`, whose strike test
/// passed this tick: one fleet against its target world, both as they stand
/// now. The fleet is in orbit at depth 1.0 with speed 0. What the attack
/// leaves on its order:
/// - Bombard: the order stands, so the fleet bombards again each year until
///   the player changes it.
/// - Invade and Blitz: the assault resolves, win or lose (an invasion whose
///   batteries survive suppression aborts with its armies still aboard), and
///   the surviving fleet holds in orbit. There is no retry.
///
/// The orbital gate is the encounter handler's: by the time the strike
/// reaches here the fleet has beaten, or found empty, the owner's orbit. A
/// fleet whose order no longer validates (its own empire took the world
/// earlier this year, or it lost the ships the order needs) does nothing;
/// the next year's input sanitizer clears the order.
pub(crate) fn resolve_strike(
    game_data: &mut CoreGameData,
    campaign_seed: u64,
    fleet_idx: usize,
) -> AssaultEvents {
    let mut events = AssaultEvents::default();
    let order = game_data.fleets[fleet_idx].standing_order_kind();
    let kind = match order {
        Order::BombardWorld => Mission::BombardWorld,
        Order::InvadeWorld => Mission::InvadeWorld,
        Order::BlitzWorld => Mission::BlitzWorld,
        _ => return events,
    };
    if !fleet_still_ready_for_assault(game_data, fleet_idx, order) {
        return events;
    }
    let coords = game_data.fleets[fleet_idx].standing_order_target_coords_raw();
    let Some(planet_idx) = planet_idx_at_coords(game_data, coords) else {
        return events;
    };
    let empire = game_data.fleets[fleet_idx].owner_empire_raw();
    let fleets = [fleet_idx];
    let mission_event = move |outcome, abort_reason| MissionEvent {
        fleet_idx,
        owner_empire_raw: empire,
        kind,
        outcome,
        abort_reason,
        planet_idx: Some(planet_idx),
        location_coords: Some(coords),
        target_coords: Some(coords),
        stardate_week: None,
    };

    let battle_year = game_data.conquest.game_year();
    match kind {
        Mission::BombardWorld => {
            let defender_empire = game_data.planets[planet_idx].owner_empire_slot_raw();
            let pre_armies = game_data.planets[planet_idx].army_count_raw();
            let pre_batteries = game_data.planets[planet_idx].ground_batteries_raw();
            let pre_stored_goods = game_data.planets[planet_idx].stored_production_points();
            let pre_factories = game_data.planets[planet_idx]
                .present_production_points()
                .unwrap_or(0);
            let pre_stardock_items: u32 = (0..STARDOCK_SLOT_COUNT)
                .map(|s| game_data.planets[planet_idx].stardock_count_raw(s) as u32)
                .sum();
            let pre_docked_summary = stardock_summary(&game_data.planets[planet_idx]);

            let before = fleet_state_from_records(game_data, &fleets, 0);
            let initial_attacking_armies: u32 = fleets
                .iter()
                .map(|idx| game_data.fleets[*idx].army_count() as u32)
                .sum();
            let mut fleet_state = before.clone();
            let mut breakthrough = false;

            // Three-round bombardment: rounds 1-2 suppression, round 3 breakthrough if batteries cleared.
            for round in 1..=3u32 {
                let attack_as = bombard_attack_as(&fleet_state);
                if attack_as == 0 {
                    break; // fleet has no bombardment capability left
                }
                let batteries_at_round_start = game_data.planets[planet_idx].ground_batteries_raw();
                let is_breakthrough = round == 3 && batteries_at_round_start == 0;
                if is_breakthrough {
                    breakthrough = true;
                }

                let defense_as = batteries_at_round_start as u32 * GROUND_AS_BATTERY;
                let attacker_exchange = resolve_orbital_exchange(
                    campaign_seed,
                    battle_year,
                    coords,
                    COMBAT_KIND_BOMBARD,
                    round,
                    empire,
                    defender_empire,
                    attack_as,
                    defense_as.max(1),
                    fleet_state.is_mixed(),
                );

                // Batteries fire back each round they exist.
                if defense_as > 0 {
                    let defender_exchange = resolve_orbital_exchange(
                        campaign_seed,
                        battle_year,
                        coords,
                        COMBAT_KIND_BOMBARD,
                        round,
                        defender_empire,
                        empire,
                        defense_as,
                        attack_as.max(1),
                        false,
                    );
                    let mut next_state = fleet_state.clone();
                    apply_hits_to_fleet(
                        &mut next_state,
                        0,
                        defender_exchange.hits,
                        u32::from(defender_exchange.critical),
                    );
                    super::fleet_battle::distribute_fleet_losses(
                        game_data,
                        &fleets,
                        &fleet_state,
                        &next_state,
                    );
                    fleet_state = fleet_state_from_records(game_data, &fleets, 0);
                }

                // Apply planet damage: suppression or breakthrough.
                let hits = scalar_hits_with_critical(attacker_exchange);
                if is_breakthrough {
                    apply_planet_bombardment_damage(&mut game_data.planets[planet_idx], hits);
                } else {
                    apply_planet_suppression_damage(&mut game_data.planets[planet_idx], hits);
                }
            }

            let post_planet = &game_data.planets[planet_idx];
            let post_docked_summary = stardock_summary(post_planet);
            events.bombard_events.push(BombardEvent {
                planet_idx,
                attacker_empire_raw: empire,
                attacker_fleet_number: preferred_reporting_fleet_number(game_data, &fleets),
                defender_empire_raw: post_planet.owner_empire_slot_raw(),
                attacker_initial: ship_counts_from_state(&before),
                attacker_loaded_armies_initial: initial_attacking_armies,
                defender_batteries_initial: pre_batteries,
                defender_armies_initial: pre_armies,
                attacker_losses: ship_losses_from_states(&before, &fleet_state),
                defender_battery_losses: pre_batteries
                    .saturating_sub(post_planet.ground_batteries_raw()),
                defender_army_losses: pre_armies.saturating_sub(post_planet.army_count_raw()),
                breakthrough,
                docked_losses: stardock_summary_diff(pre_docked_summary, post_docked_summary),
                stardock_items_destroyed: pre_stardock_items.saturating_sub(
                    (0..STARDOCK_SLOT_COUNT)
                        .map(|s| post_planet.stardock_count_raw(s) as u32)
                        .sum::<u32>(),
                ),
                stored_goods_destroyed: pre_stored_goods
                    .saturating_sub(post_planet.stored_production_points()),
                factories_destroyed: pre_factories
                    .saturating_sub(post_planet.present_production_points().unwrap_or(0)),
                stardate_week: None,
            });
            if assault_force_destroyed(game_data, &fleets) {
                push_assault_destroyed_fleet_event(
                    &mut events,
                    game_data,
                    &fleets,
                    coords,
                    ship_counts_from_state(&before),
                    0,
                    pre_batteries,
                    pre_armies,
                    pre_batteries.saturating_sub(post_planet.ground_batteries_raw()),
                    pre_armies.saturating_sub(post_planet.army_count_raw()),
                    defender_empire,
                );
            }
            events
                .mission_events
                .push(mission_event(MissionOutcome::Succeeded, None));
        }
        Mission::InvadeWorld => {
            let state = fleet_state_from_records(game_data, &fleets, 0);
            let bombard_as = bombard_attack_as(&state);
            let initial_attacking_armies: u32 = fleets
                .iter()
                .map(|idx| game_data.fleets[*idx].army_count() as u32)
                .sum();
            let previous_owner = game_data.planets[planet_idx].owner_empire_slot_raw();
            let planet = &game_data.planets[planet_idx];
            let pre_batteries = planet.ground_batteries_raw();
            let pre_armies = planet.army_count_raw();
            let battery_as = planet.ground_batteries_raw() as u32 * GROUND_AS_BATTERY;
            let suppression_exchange = resolve_orbital_exchange(
                campaign_seed,
                battle_year,
                coords,
                COMBAT_KIND_INVASION_SUPPRESSION,
                1,
                empire,
                previous_owner,
                bombard_as,
                battery_as.max(1),
                state.is_mixed(),
            );
            let return_exchange = resolve_orbital_exchange(
                campaign_seed,
                battle_year,
                coords,
                COMBAT_KIND_INVASION_SUPPRESSION,
                1,
                previous_owner,
                empire,
                battery_as,
                bombard_as.max(1),
                false,
            );

            let before = state.clone();
            let mut after = state.clone();
            apply_hits_to_fleet(
                &mut after,
                0,
                return_exchange.hits,
                u32::from(return_exchange.critical),
            );
            super::fleet_battle::distribute_fleet_losses(game_data, &fleets, &before, &after);

            apply_battery_hits(
                &mut game_data.planets[planet_idx],
                scalar_hits_with_critical(suppression_exchange),
            );

            let batteries_cleared = game_data.planets[planet_idx].ground_batteries_raw() == 0;
            if batteries_cleared {
                let soft_exchange = resolve_ground_exchange(
                    campaign_seed,
                    battle_year,
                    coords,
                    COMBAT_KIND_INVASION_SOFTEN,
                    1,
                    empire,
                    previous_owner,
                    bombard_attack_as(&after),
                    game_data.planets[planet_idx].army_count_raw() as u32 * GROUND_AS_ARMY,
                    0,
                );
                // Soften targets armies only — factories and stored goods are
                // preserved so the captured planet retains its production value.
                let soften_army_losses = apply_capped_invasion_soften_damage(
                    &mut game_data.planets[planet_idx],
                    scalar_hits_with_critical(soft_exchange),
                    pre_armies,
                );

                let attacking_armies: u32 = fleets
                    .iter()
                    .map(|idx| game_data.fleets[*idx].army_count() as u32)
                    .sum();
                let defender_armies = game_data.planets[planet_idx].army_count_raw() as u32;
                let (attacker_survivors, defender_survivors) = resolve_ground_battle_to_destruction(
                    campaign_seed,
                    battle_year,
                    coords,
                    COMBAT_KIND_GROUND,
                    empire,
                    previous_owner,
                    attacking_armies,
                    defender_armies,
                    1,
                );
                let defender_battery_losses = pre_batteries
                    .saturating_sub(game_data.planets[planet_idx].ground_batteries_raw());
                let defender_army_losses =
                    pre_armies.saturating_sub(defender_survivors.min(255) as u8);

                for &idx in &fleets {
                    game_data.fleets[idx].set_army_count(0);
                }
                if attacker_survivors > 0 && defender_survivors == 0 {
                    let planet = &mut game_data.planets[planet_idx];
                    planet.set_owner_empire_slot_raw(empire);
                    // Just captured: no income until the yearly pass has
                    // stepped the status to 2, as ECMAINT does.
                    planet.set_ownership_status_raw(0);
                    planet.set_army_count_raw(attacker_survivors.min(255) as u8);
                    planet.set_ground_batteries_raw(0);
                    events
                        .ownership_change_events
                        .push(PlanetOwnershipChangeEvent {
                            planet_idx,
                            reporting_empire_raw: previous_owner,
                            previous_owner_empire_raw: previous_owner,
                            new_owner_empire_raw: empire,
                            stardate_week: None,
                        });
                    events
                        .mission_events
                        .push(mission_event(MissionOutcome::Succeeded, None));
                    events.assault_report_events.push(AssaultReportEvent {
                        kind: Mission::InvadeWorld,
                        attacker_fleet_number: preferred_reporting_fleet_number(game_data, &fleets),
                        planet_idx,
                        attacker_empire_raw: empire,
                        defender_empire_raw: previous_owner,
                        attacker_initial: ship_counts_from_state(&before),
                        attacker_loaded_armies_initial: initial_attacking_armies,
                        defender_batteries_initial: pre_batteries,
                        defender_armies_initial: pre_armies,
                        attacker_ship_losses: ship_losses_from_states(&before, &after),
                        attacker_army_losses: attacking_armies.saturating_sub(attacker_survivors),
                        transport_army_losses: 0,
                        defender_battery_losses,
                        defender_army_losses_softening: soften_army_losses,
                        defender_army_losses,
                        outcome: MissionOutcome::Succeeded,
                        stardate_week: None,
                    });
                    if assault_force_destroyed(game_data, &fleets) {
                        push_assault_destroyed_fleet_event(
                            &mut events,
                            game_data,
                            &fleets,
                            coords,
                            ship_counts_from_state(&before),
                            initial_attacking_armies,
                            pre_batteries,
                            pre_armies,
                            defender_battery_losses,
                            defender_army_losses,
                            previous_owner,
                        );
                    }
                } else {
                    game_data.planets[planet_idx]
                        .set_army_count_raw(defender_survivors.min(255) as u8);
                    events
                        .mission_events
                        .push(mission_event(MissionOutcome::Failed, None));
                    events.assault_report_events.push(AssaultReportEvent {
                        kind: Mission::InvadeWorld,
                        attacker_fleet_number: preferred_reporting_fleet_number(game_data, &fleets),
                        planet_idx,
                        attacker_empire_raw: empire,
                        defender_empire_raw: previous_owner,
                        attacker_initial: ship_counts_from_state(&before),
                        attacker_loaded_armies_initial: initial_attacking_armies,
                        defender_batteries_initial: pre_batteries,
                        defender_armies_initial: pre_armies,
                        attacker_ship_losses: ship_losses_from_states(&before, &after),
                        attacker_army_losses: attacking_armies,
                        transport_army_losses: 0,
                        defender_battery_losses,
                        defender_army_losses_softening: soften_army_losses,
                        defender_army_losses,
                        outcome: MissionOutcome::Failed,
                        stardate_week: None,
                    });
                    if assault_force_destroyed(game_data, &fleets) {
                        push_assault_destroyed_fleet_event(
                            &mut events,
                            game_data,
                            &fleets,
                            coords,
                            ship_counts_from_state(&before),
                            initial_attacking_armies,
                            pre_batteries,
                            pre_armies,
                            defender_battery_losses,
                            defender_army_losses,
                            previous_owner,
                        );
                    }
                }
            } else {
                // The batteries held: no troops land and the fleets
                // keep their armies.
                events
                    .mission_events
                    .push(mission_event(MissionOutcome::Aborted, None));
                events.assault_report_events.push(AssaultReportEvent {
                    kind: Mission::InvadeWorld,
                    attacker_fleet_number: preferred_reporting_fleet_number(game_data, &fleets),
                    planet_idx,
                    attacker_empire_raw: empire,
                    defender_empire_raw: previous_owner,
                    attacker_initial: ship_counts_from_state(&before),
                    attacker_loaded_armies_initial: initial_attacking_armies,
                    defender_batteries_initial: pre_batteries,
                    defender_armies_initial: pre_armies,
                    attacker_ship_losses: ship_losses_from_states(&before, &after),
                    attacker_army_losses: 0,
                    transport_army_losses: 0,
                    defender_battery_losses: pre_batteries
                        .saturating_sub(game_data.planets[planet_idx].ground_batteries_raw()),
                    defender_army_losses_softening: 0,
                    defender_army_losses: 0,
                    outcome: MissionOutcome::Aborted,
                    stardate_week: None,
                });
                if assault_force_destroyed(game_data, &fleets) {
                    push_assault_destroyed_fleet_event(
                        &mut events,
                        game_data,
                        &fleets,
                        coords,
                        ship_counts_from_state(&before),
                        initial_attacking_armies,
                        pre_batteries,
                        pre_armies,
                        pre_batteries
                            .saturating_sub(game_data.planets[planet_idx].ground_batteries_raw()),
                        0,
                        previous_owner,
                    );
                }
            }

            let intel_source = if game_data.planets[planet_idx].owner_empire_slot_raw() == empire {
                PlanetIntelSource::AssaultSuccess
            } else {
                PlanetIntelSource::AssaultFailure
            };

            clear_arrival_and_hold(game_data, &fleets);
            push_planet_intel(
                &mut events.planet_intel_events,
                planet_idx,
                empire,
                intel_source,
            );
            push_planet_intel(
                &mut events.planet_intel_events,
                planet_idx,
                previous_owner,
                intel_source,
            );
            let owner_after = game_data.planets[planet_idx].owner_empire_slot_raw();
            push_planet_intel(
                &mut events.planet_intel_events,
                planet_idx,
                owner_after,
                intel_source,
            );
        }
        Mission::BlitzWorld => {
            let previous_owner = game_data.planets[planet_idx].owner_empire_slot_raw();
            let cover_state = fleet_state_from_records(game_data, &fleets, 0);
            let pre_batteries = game_data.planets[planet_idx].ground_batteries_raw();
            let pre_armies = game_data.planets[planet_idx].army_count_raw();
            let attacking_armies: u32 = fleets
                .iter()
                .map(|idx| game_data.fleets[*idx].army_count() as u32)
                .sum();
            let cover_exchange = blitz_cover_exchange(
                campaign_seed,
                battle_year,
                coords,
                empire,
                previous_owner,
                &cover_state,
                pre_batteries as u32 * GROUND_AS_BATTERY,
            );
            apply_battery_hits(
                &mut game_data.planets[planet_idx],
                scalar_hits_with_critical(cover_exchange),
            );
            let surviving_batteries = game_data.planets[planet_idx].ground_batteries_raw();
            let (landing_army_losses, ship_losses) = apply_blitz_landing_fire(
                game_data,
                campaign_seed,
                battle_year,
                coords,
                previous_owner,
                empire,
                &fleets,
                surviving_batteries,
            );
            let armies_after_landing = attacking_armies.saturating_sub(landing_army_losses);
            let defender_armies = game_data.planets[planet_idx].army_count_raw() as u32;
            let (attacker_survivors, defender_survivors) = resolve_ground_battle_to_destruction(
                campaign_seed,
                battle_year,
                coords,
                COMBAT_KIND_BLITZ_GROUND,
                empire,
                previous_owner,
                armies_after_landing,
                defender_armies,
                1,
            );
            let defender_battery_losses =
                pre_batteries.saturating_sub(game_data.planets[planet_idx].ground_batteries_raw());
            let defender_army_losses = pre_armies.saturating_sub(defender_survivors.min(255) as u8);

            for &idx in &fleets {
                game_data.fleets[idx].set_army_count(0);
            }
            if attacker_survivors > 0 && defender_survivors == 0 {
                let batteries = game_data.planets[planet_idx].ground_batteries_raw();
                let planet = &mut game_data.planets[planet_idx];
                planet.set_owner_empire_slot_raw(empire);
                // Just captured: no income until the yearly pass has
                // stepped the status to 2, as ECMAINT does.
                planet.set_ownership_status_raw(0);
                planet.set_army_count_raw(attacker_survivors.min(255) as u8);
                planet.set_ground_batteries_raw(batteries);
                events
                    .ownership_change_events
                    .push(PlanetOwnershipChangeEvent {
                        planet_idx,
                        reporting_empire_raw: previous_owner,
                        previous_owner_empire_raw: previous_owner,
                        new_owner_empire_raw: empire,
                        stardate_week: None,
                    });
                events
                    .mission_events
                    .push(mission_event(MissionOutcome::Succeeded, None));
                events.assault_report_events.push(AssaultReportEvent {
                    kind: Mission::BlitzWorld,
                    attacker_fleet_number: preferred_reporting_fleet_number(game_data, &fleets),
                    planet_idx,
                    attacker_empire_raw: empire,
                    defender_empire_raw: previous_owner,
                    attacker_initial: ship_counts_from_state(&cover_state),
                    attacker_loaded_armies_initial: attacking_armies,
                    defender_batteries_initial: pre_batteries,
                    defender_armies_initial: pre_armies,
                    attacker_ship_losses: ship_losses,
                    attacker_army_losses: armies_after_landing.saturating_sub(attacker_survivors),
                    transport_army_losses: landing_army_losses,
                    defender_battery_losses,
                    defender_army_losses_softening: 0,
                    defender_army_losses,
                    outcome: MissionOutcome::Succeeded,
                    stardate_week: None,
                });
                if assault_force_destroyed(game_data, &fleets) {
                    push_assault_destroyed_fleet_event(
                        &mut events,
                        game_data,
                        &fleets,
                        coords,
                        ship_counts_from_state(&cover_state),
                        attacking_armies,
                        pre_batteries,
                        pre_armies,
                        defender_battery_losses,
                        defender_army_losses,
                        previous_owner,
                    );
                }
            } else {
                game_data.planets[planet_idx].set_army_count_raw(defender_survivors.min(255) as u8);
                events
                    .mission_events
                    .push(mission_event(MissionOutcome::Failed, None));
                events.assault_report_events.push(AssaultReportEvent {
                    kind: Mission::BlitzWorld,
                    attacker_fleet_number: preferred_reporting_fleet_number(game_data, &fleets),
                    planet_idx,
                    attacker_empire_raw: empire,
                    defender_empire_raw: previous_owner,
                    attacker_initial: ship_counts_from_state(&cover_state),
                    attacker_loaded_armies_initial: attacking_armies,
                    defender_batteries_initial: pre_batteries,
                    defender_armies_initial: pre_armies,
                    attacker_ship_losses: ship_losses,
                    attacker_army_losses: armies_after_landing,
                    transport_army_losses: landing_army_losses,
                    defender_battery_losses,
                    defender_army_losses_softening: 0,
                    defender_army_losses,
                    outcome: MissionOutcome::Failed,
                    stardate_week: None,
                });
                if assault_force_destroyed(game_data, &fleets) {
                    push_assault_destroyed_fleet_event(
                        &mut events,
                        game_data,
                        &fleets,
                        coords,
                        ship_counts_from_state(&cover_state),
                        attacking_armies,
                        pre_batteries,
                        pre_armies,
                        defender_battery_losses,
                        defender_army_losses,
                        previous_owner,
                    );
                }
            }

            let intel_source = if game_data.planets[planet_idx].owner_empire_slot_raw() == empire {
                PlanetIntelSource::AssaultSuccess
            } else {
                PlanetIntelSource::AssaultFailure
            };

            clear_arrival_and_hold(game_data, &fleets);
            push_planet_intel(
                &mut events.planet_intel_events,
                planet_idx,
                empire,
                intel_source,
            );
            push_planet_intel(
                &mut events.planet_intel_events,
                planet_idx,
                previous_owner,
                intel_source,
            );
            let owner_after = game_data.planets[planet_idx].owner_empire_slot_raw();
            push_planet_intel(
                &mut events.planet_intel_events,
                planet_idx,
                owner_after,
                intel_source,
            );
        }
        _ => {}
    }

    events
}

// The 64-round ground-combat cap is not reachable through the public API
// with real dice, so its tie-break is pinned here on the private helper.
#[cfg(test)]
mod tests {
    use super::settle_ground_battle_at_cap;

    #[test]
    fn ground_battle_cap_never_ends_in_a_draw() {
        assert_eq!(settle_ground_battle_at_cap(7, 3), (7, 0));
        assert_eq!(settle_ground_battle_at_cap(3, 7), (0, 7));
        assert_eq!(settle_ground_battle_at_cap(5, 5), (0, 5));
    }
}
