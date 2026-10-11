use crate::{
    AssaultReportEvent, BombardEvent, CoreGameData, FleetDestroyedEvent, Mission, MissionEvent,
    MissionOutcome, Order, PlanetIntelEvent, PlanetIntelSource, PlanetOwnershipChangeEvent,
    ProductionItemKind, STARDOCK_SLOT_COUNT, ShipLosses,
};

use super::exchange::{
    COMBAT_KIND_BLITZ_COVER, COMBAT_KIND_BLITZ_GROUND, COMBAT_KIND_BLITZ_LANDING,
    COMBAT_KIND_BOMBARD, COMBAT_KIND_GROUND, COMBAT_KIND_INVASION_SOFTEN,
    COMBAT_KIND_INVASION_SUPPRESSION, ExchangeResolution, GROUND_AS_ARMY, GROUND_AS_BATTERY,
    GROUND_DS_ARMY, GROUND_DS_BATTERY, apply_hits_to_fleet, apply_landing_fire_to_fleet,
    landing_volley, resolve_ground_exchange, resolve_orbital_exchange, scalar_hits_with_critical,
    seeded_combat_roll,
};
use super::reporting::{preferred_reporting_fleet_number, push_planet_intel};
use super::retreat::set_fleet_to_hold_current_position;
use super::state::{
    FleetCombatState, IDX_BB, IDX_CA, IDX_DD, IDX_ET, IDX_SB, IDX_SC, IDX_TT, fleet_class_ds,
    fleet_state_from_records, planet_idx_at_coords, ship_counts_from_state,
    ship_losses_from_states,
};
use nc_data::{
    InheritedAssets, Real48, has_any_force, present_production_points, set_armies_aboard,
    stardock_slot_kind, stored_production_points,
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
                .is_none_or(|fleet| !has_any_force(fleet))
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
    primary_enemy_empire: u8,
) {
    events.fleet_destroyed_events.push(FleetDestroyedEvent {
        reporting_empire: game_data.fleets[fleet_indices[0]].owner.0,
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
        primary_enemy_empire: (primary_enemy_empire != 0).then_some(primary_enemy_empire),
        primary_enemy_fleet_number: None,
        stardate_week: None,
    });
}

fn fleet_still_ready_for_assault(game_data: &CoreGameData, fleet_idx: usize, order: Order) -> bool {
    let Some(fleet) = game_data.fleets.get(fleet_idx) else {
        return false;
    };
    let target_coords = fleet.target.to_array();
    if fleet.order != order || fleet.sector.to_array() != target_coords {
        return false;
    }
    game_data
        .validate_fleet_order_payload(fleet_idx + 1, order.code(), target_coords, None, None)
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

/// Stored goods one point of bombardment reaching industry destroys, beside
/// the one point of production it destroys.
const GOODS_DESTROYED_PER_POINT: u32 = 25;

/// Each docked item costs its own DS in hits. Hits too few to destroy the
/// next item move on to the following slot; partial hits never carry over.
fn reduce_stardock(planet: &mut nc_data::Planet, mut hits: u32) -> u32 {
    for slot in 0..STARDOCK_SLOT_COUNT {
        if hits == 0 {
            break;
        }
        let count = u16::try_from(planet.stardock[slot]).unwrap_or(u16::MAX) as u32;
        if count == 0 {
            continue;
        }
        let ds = stardock_item_ds(stardock_slot_kind(planet, slot));
        let destroyed = (hits / ds).min(count) as u16;
        if destroyed == 0 {
            continue;
        }
        planet.stardock[slot] = u32::from(
            u16::try_from(planet.stardock[slot])
                .unwrap_or(u16::MAX)
                .saturating_sub(destroyed),
        );
        hits -= destroyed as u32 * ds;
    }
    hits
}

/// Each ground battery destroyed costs its DS (20 hits). Returns the hits
/// left over.
fn apply_battery_hits(planet: &mut nc_data::Planet, hits: u32) -> u32 {
    let battery_loss = (hits / GROUND_DS_BATTERY)
        .min(u8::try_from(planet.ground_batteries).unwrap_or(u8::MAX) as u32);
    planet.ground_batteries =
        u16::from(u8::try_from(planet.ground_batteries).unwrap_or(u8::MAX) - battery_loss as u8);
    hits - battery_loss * GROUND_DS_BATTERY
}

fn stardock_summary(planet: &nc_data::Planet) -> nc_data::EmpireUnitSummary {
    let mut summary = nc_data::EmpireUnitSummary::default();
    for slot in 0..STARDOCK_SLOT_COUNT {
        let count = u32::from(u16::try_from(planet.stardock[slot]).unwrap_or(u16::MAX));
        if count == 0 {
            continue;
        }
        match stardock_slot_kind(planet, slot) {
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

/// Suppression damage. Batteries shield the stardock (combat decision 5): an
/// exchange that begins with batteries standing hits them only, and the hits
/// beyond what destroys them expire. Once none stand, the exchange hits the
/// stardock. Armies, stored goods, and factories are shielded either way.
pub(super) fn apply_planet_suppression_damage(planet: &mut nc_data::Planet, hits: u32) {
    if planet.ground_batteries > 0 {
        apply_battery_hits(planet, hits);
    } else {
        reduce_stardock(planet, hits);
    }
}

fn apply_capped_invasion_soften_damage(
    planet: &mut nc_data::Planet,
    hits: u32,
    initial_armies: u8,
) -> u8 {
    let max_soften_losses = u32::from(initial_armies / 2);
    let army_loss = (hits / GROUND_DS_ARMY)
        .min(u8::try_from(planet.armies).unwrap_or(u8::MAX) as u32)
        .min(max_soften_losses) as u8;
    planet.armies = u16::from(
        u8::try_from(planet.armies)
            .unwrap_or(u8::MAX)
            .saturating_sub(army_loss),
    );
    army_loss
}

/// The breakthrough exchange, which only an exchange that begins with no
/// batteries standing can be: stardock -> armies -> stored goods -> factories.
/// Units cost their DS in hits; hits reaching goods and factories are scaled
/// down from the 10x combat scale before the per-unit conversion.
pub(super) fn apply_planet_bombardment_damage(planet: &mut nc_data::Planet, hits: u32) {
    let mut hits = reduce_stardock(planet, hits);

    let army_loss =
        (hits / GROUND_DS_ARMY).min(u8::try_from(planet.armies).unwrap_or(u8::MAX) as u32);
    planet.armies = u16::from(u8::try_from(planet.armies).unwrap_or(u8::MAX) - army_loss as u8);
    hits -= army_loss * GROUND_DS_ARMY;

    let hits = hits / 10;
    if hits > 0 {
        // Only positive goods burn, fraction and all; a debt is left alone.
        let goods = planet.stored_goods.to_f64();
        if goods > 0.0 {
            let goods_loss = f64::from(hits.saturating_mul(GOODS_DESTROYED_PER_POINT));
            planet.stored_goods = Real48::from_f64((goods - goods_loss).max(0.0));
        }
        let production = planet.production.to_f64();
        planet.production = Real48::from_f64((production - f64::from(hits)).max(0.0));
    }
}

/// A scorched world that falls (ECMAINT `110a:6e6a..6f02`): the defenders
/// halve its production and its positive goods, empty the dock, and lift the
/// order, and production never ends below 1.0. Debt is untouched, and a world
/// with no potential stays at production 0.
fn apply_scorch_on_capture(planet: &mut nc_data::Planet) {
    if !planet.scorch_earth {
        return;
    }
    let goods = planet.stored_goods.to_f64();
    if goods > 0.0 {
        planet.stored_goods = Real48::from_f64(goods / 2.0);
    }
    let production = planet.production.to_f64() / 2.0;
    planet.production = if planet.potential_production > 0 {
        Real48::from_f64(production.max(1.0))
    } else {
        Real48::from_f64(production)
    };
    planet.stardock = nc_data::Stardock::default();
    planet.scorch_earth = false;
}

/// The world passes to `new_owner` with `garrison` armies and `batteries`.
/// Everything else the fight left stands: goods, production, docked ships,
/// and the build queue pass intact, with no capture bonus and no forgiven
/// debt, unless the defenders had scorched it. Returns what the new owner
/// finds, for the capture report.
fn capture_planet(
    planet: &mut nc_data::Planet,
    new_owner: u8,
    garrison: u32,
    batteries: u16,
) -> InheritedAssets {
    let scorched = planet.scorch_earth;
    planet.owner.0 = new_owner;
    // Just captured: no income until the yearly pass has stepped the status
    // to 2, as ECMAINT does.
    planet.years_under_owner = 0;
    planet.armies = u16::from(garrison.min(255) as u8);
    planet.ground_batteries = batteries;
    apply_scorch_on_capture(planet);
    InheritedAssets {
        stored_goods: stored_production_points(planet),
        docked: stardock_summary(planet),
        queued_work: planet.build_queue.iter().map(|slot| slot.points_owed).sum(),
        scorched,
    }
}

pub(super) fn bombard_attack_as(state: &FleetCombatState) -> u32 {
    let mut as_total = 0;
    // Healthy ships (AS_DD: 10, AS_CA: 30, AS_BB: 90)
    // Bombard modifiers (0.5x, 1.0x, 1.5x)
    as_total += state.nominal_count(IDX_DD) * 5 + state.crippled_count(IDX_DD) * 2;
    as_total += state.nominal_count(IDX_CA) * 30 + state.crippled_count(IDX_CA) * 15;
    as_total += state.nominal_count(IDX_BB) * 135 + state.crippled_count(IDX_BB) * 67;
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
    let before = fleet_state_from_records(game_data, fleet_indices, &[]);
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
            .map(|&idx| u32::from(game_data.fleets[idx].ships.loaded_transports))
            .sum()
    };
    let armies_before = loaded_armies(game_data);
    super::damage::write_back_strike(game_data, fleet_indices[0], &after);
    let armies_after = loaded_armies(game_data);

    (
        armies_before.saturating_sub(armies_after),
        ship_losses_from_states(&before, &after),
    )
}

/// The landing battle (combat decision 4), fought to the last army of one
/// side. Each round the defenders fire first and the attackers they leave
/// reply. Every volley rolls once on the shared flat table, unmodified: the
/// roll `roll(round, acting_empire, target_empire)` gives the d10, and the
/// side's armies deal `armies x effectiveness` damage in quarter armies.
/// Damage short of an army carries, in quarters, into the next round of this
/// landing and is dropped when it ends. There is no defender modifier and no
/// critical, and the attackers fire only with what survives, so both sides
/// cannot be destroyed. Each round costs the attackers at least a quarter
/// army per defending army, so the battle ends within `4 x attackers` rounds.
/// Returns the armies each side has left; at most one is not zero.
pub(super) fn resolve_landing_battle(
    attacker_empire: u8,
    defender_empire: u8,
    mut attackers: u32,
    mut defenders: u32,
    roll: &mut dyn FnMut(u32, u8, u8) -> u8,
) -> (u32, u32) {
    let mut attackers_carry = 0;
    let mut defenders_carry = 0;
    for round in 1.. {
        if attackers == 0 || defenders == 0 {
            break;
        }
        let damage = attackers_carry
            + landing_volley(roll(round, defender_empire, attacker_empire), defenders);
        attackers = attackers.saturating_sub(damage / 4);
        attackers_carry = damage % 4;
        if attackers == 0 {
            break;
        }
        let damage = defenders_carry
            + landing_volley(roll(round, attacker_empire, defender_empire), attackers);
        defenders = defenders.saturating_sub(damage / 4);
        defenders_carry = damage % 4;
    }
    (attackers, defenders)
}

/// The landing battle with the campaign's seeded dice: the roll depends only
/// on the year, the world, the battle kind (an invasion's or a blitz's), the
/// round, and the two empires.
#[allow(clippy::too_many_arguments)]
pub(super) fn resolve_seeded_landing(
    campaign_seed: u64,
    game_year: u16,
    coords: [u8; 2],
    combat_kind: u64,
    attacker_empire: u8,
    defender_empire: u8,
    attackers: u32,
    defenders: u32,
) -> (u32, u32) {
    let mut roll = |round, acting_empire, target_empire| {
        seeded_combat_roll(
            campaign_seed,
            game_year,
            coords,
            combat_kind,
            round,
            acting_empire,
            target_empire,
        )
    };
    resolve_landing_battle(
        attacker_empire,
        defender_empire,
        attackers,
        defenders,
        &mut roll,
    )
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
    let order = game_data.fleets[fleet_idx].order;
    let kind = match order {
        Order::BombardWorld => Mission::BombardWorld,
        Order::InvadeWorld => Mission::InvadeWorld,
        Order::BlitzWorld => Mission::BlitzWorld,
        _ => return events,
    };
    if !fleet_still_ready_for_assault(game_data, fleet_idx, order) {
        return events;
    }
    let coords = game_data.fleets[fleet_idx].target.to_array();
    let Some(planet_idx) = planet_idx_at_coords(game_data, coords) else {
        return events;
    };
    let empire = game_data.fleets[fleet_idx].owner.0;
    let fleets = [fleet_idx];
    let mission_event = move |outcome, abort_reason| MissionEvent {
        fleet_idx,
        owner_empire: empire,
        kind,
        outcome,
        abort_reason,
        planet_idx: Some(planet_idx),
        location_coords: Some(coords),
        target_coords: Some(coords),
        stardate_week: None,
    };

    let battle_year = game_data.conquest.game_year;
    match kind {
        Mission::BombardWorld => {
            let defender_empire = game_data.planets[planet_idx].owner.0;
            let pre_armies = u8::try_from(game_data.planets[planet_idx].armies).unwrap_or(u8::MAX);
            let pre_batteries =
                u8::try_from(game_data.planets[planet_idx].ground_batteries).unwrap_or(u8::MAX);
            let pre_stored_goods = stored_production_points(&game_data.planets[planet_idx]);
            let pre_factories = present_production_points(&game_data.planets[planet_idx]);
            let pre_stardock_items: u32 = (0..STARDOCK_SLOT_COUNT)
                .map(|s| {
                    u16::try_from(game_data.planets[planet_idx].stardock[s]).unwrap_or(u16::MAX)
                        as u32
                })
                .sum();
            let pre_docked_summary = stardock_summary(&game_data.planets[planet_idx]);

            let before = fleet_state_from_records(game_data, &fleets, &[]);
            let initial_attacking_armies: u32 = fleets
                .iter()
                .map(|idx| game_data.fleets[*idx].ships.loaded_transports as u32)
                .sum();
            let mut fleet_state = before.clone();
            let mut breakthrough = false;

            // Three-round bombardment: rounds 1-2 suppression, round 3 breakthrough if batteries cleared.
            for round in 1..=3u32 {
                let attack_as = bombard_attack_as(&fleet_state);
                if attack_as == 0 {
                    break; // fleet has no bombardment capability left
                }
                let batteries_at_round_start =
                    u8::try_from(game_data.planets[planet_idx].ground_batteries).unwrap_or(u8::MAX);
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
                    super::damage::write_back_strike(game_data, fleet_idx, &next_state);
                    fleet_state = fleet_state_from_records(game_data, &fleets, &[]);
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
                attacker_empire: empire,
                attacker_fleet_number: preferred_reporting_fleet_number(game_data, &fleets),
                defender_empire: post_planet.owner.0,
                attacker_initial: ship_counts_from_state(&before),
                attacker_loaded_armies_initial: initial_attacking_armies,
                defender_batteries_initial: pre_batteries,
                defender_armies_initial: pre_armies,
                attacker_losses: ship_losses_from_states(&before, &fleet_state),
                defender_battery_losses: pre_batteries
                    .saturating_sub(u8::try_from(post_planet.ground_batteries).unwrap_or(u8::MAX)),
                defender_army_losses: pre_armies
                    .saturating_sub(u8::try_from(post_planet.armies).unwrap_or(u8::MAX)),
                breakthrough,
                docked_losses: stardock_summary_diff(pre_docked_summary, post_docked_summary),
                stardock_items_destroyed: pre_stardock_items.saturating_sub(
                    (0..STARDOCK_SLOT_COUNT)
                        .map(|s| u16::try_from(post_planet.stardock[s]).unwrap_or(u16::MAX) as u32)
                        .sum::<u32>(),
                ),
                stored_goods_destroyed: pre_stored_goods
                    .saturating_sub(stored_production_points(post_planet)),
                factories_destroyed: pre_factories
                    .saturating_sub(present_production_points(post_planet)),
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
                    pre_batteries.saturating_sub(
                        u8::try_from(post_planet.ground_batteries).unwrap_or(u8::MAX),
                    ),
                    pre_armies.saturating_sub(u8::try_from(post_planet.armies).unwrap_or(u8::MAX)),
                    defender_empire,
                );
            }
            events
                .mission_events
                .push(mission_event(MissionOutcome::Succeeded, None));
        }
        Mission::InvadeWorld => {
            let state = fleet_state_from_records(game_data, &fleets, &[]);
            let bombard_as = bombard_attack_as(&state);
            let initial_attacking_armies: u32 = fleets
                .iter()
                .map(|idx| game_data.fleets[*idx].ships.loaded_transports as u32)
                .sum();
            let previous_owner = game_data.planets[planet_idx].owner.0;
            let planet = &game_data.planets[planet_idx];
            let pre_batteries = u8::try_from(planet.ground_batteries).unwrap_or(u8::MAX);
            let pre_armies = u8::try_from(planet.armies).unwrap_or(u8::MAX);
            let battery_as =
                u8::try_from(planet.ground_batteries).unwrap_or(u8::MAX) as u32 * GROUND_AS_BATTERY;
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
            super::damage::write_back_strike(game_data, fleet_idx, &after);

            apply_battery_hits(
                &mut game_data.planets[planet_idx],
                scalar_hits_with_critical(suppression_exchange),
            );

            let batteries_cleared = game_data.planets[planet_idx].ground_batteries == 0;
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
                    u8::try_from(game_data.planets[planet_idx].armies).unwrap_or(u8::MAX) as u32
                        * GROUND_AS_ARMY,
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
                    .map(|idx| game_data.fleets[*idx].ships.loaded_transports as u32)
                    .sum();
                let defender_armies =
                    u8::try_from(game_data.planets[planet_idx].armies).unwrap_or(u8::MAX) as u32;
                let (attacker_survivors, defender_survivors) = resolve_seeded_landing(
                    campaign_seed,
                    battle_year,
                    coords,
                    COMBAT_KIND_GROUND,
                    empire,
                    previous_owner,
                    attacking_armies,
                    defender_armies,
                );
                let defender_battery_losses = pre_batteries.saturating_sub(
                    u8::try_from(game_data.planets[planet_idx].ground_batteries).unwrap_or(u8::MAX),
                );
                let defender_army_losses =
                    pre_armies.saturating_sub(defender_survivors.min(255) as u8);

                for &idx in &fleets {
                    set_armies_aboard(&mut game_data.fleets[idx], 0);
                }
                if attacker_survivors > 0 && defender_survivors == 0 {
                    let inherited = capture_planet(
                        &mut game_data.planets[planet_idx],
                        empire,
                        attacker_survivors,
                        0,
                    );
                    events
                        .ownership_change_events
                        .push(PlanetOwnershipChangeEvent {
                            planet_idx,
                            reporting_empire: previous_owner,
                            previous_owner_empire: previous_owner,
                            new_owner_empire: empire,
                            stardate_week: None,
                        });
                    events
                        .mission_events
                        .push(mission_event(MissionOutcome::Succeeded, None));
                    events.assault_report_events.push(AssaultReportEvent {
                        kind: Mission::InvadeWorld,
                        attacker_fleet_number: preferred_reporting_fleet_number(game_data, &fleets),
                        planet_idx,
                        attacker_empire: empire,
                        defender_empire: previous_owner,
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
                        inherited: Some(inherited),
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
                    game_data.planets[planet_idx].armies =
                        u16::from(defender_survivors.min(255) as u8);
                    events
                        .mission_events
                        .push(mission_event(MissionOutcome::Failed, None));
                    events.assault_report_events.push(AssaultReportEvent {
                        kind: Mission::InvadeWorld,
                        attacker_fleet_number: preferred_reporting_fleet_number(game_data, &fleets),
                        planet_idx,
                        attacker_empire: empire,
                        defender_empire: previous_owner,
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
                        inherited: None,
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
                    attacker_empire: empire,
                    defender_empire: previous_owner,
                    attacker_initial: ship_counts_from_state(&before),
                    attacker_loaded_armies_initial: initial_attacking_armies,
                    defender_batteries_initial: pre_batteries,
                    defender_armies_initial: pre_armies,
                    attacker_ship_losses: ship_losses_from_states(&before, &after),
                    attacker_army_losses: 0,
                    transport_army_losses: 0,
                    defender_battery_losses: pre_batteries.saturating_sub(
                        u8::try_from(game_data.planets[planet_idx].ground_batteries)
                            .unwrap_or(u8::MAX),
                    ),
                    defender_army_losses_softening: 0,
                    defender_army_losses: 0,
                    inherited: None,
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
                        pre_batteries.saturating_sub(
                            u8::try_from(game_data.planets[planet_idx].ground_batteries)
                                .unwrap_or(u8::MAX),
                        ),
                        0,
                        previous_owner,
                    );
                }
            }

            let intel_source = if game_data.planets[planet_idx].owner.0 == empire {
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
            let owner_after = game_data.planets[planet_idx].owner.0;
            push_planet_intel(
                &mut events.planet_intel_events,
                planet_idx,
                owner_after,
                intel_source,
            );
        }
        Mission::BlitzWorld => {
            let previous_owner = game_data.planets[planet_idx].owner.0;
            let cover_state = fleet_state_from_records(game_data, &fleets, &[]);
            let pre_batteries =
                u8::try_from(game_data.planets[planet_idx].ground_batteries).unwrap_or(u8::MAX);
            let pre_armies = u8::try_from(game_data.planets[planet_idx].armies).unwrap_or(u8::MAX);
            let attacking_armies: u32 = fleets
                .iter()
                .map(|idx| game_data.fleets[*idx].ships.loaded_transports as u32)
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
            let surviving_batteries =
                u8::try_from(game_data.planets[planet_idx].ground_batteries).unwrap_or(u8::MAX);
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
            let defender_armies =
                u8::try_from(game_data.planets[planet_idx].armies).unwrap_or(u8::MAX) as u32;
            let (attacker_survivors, defender_survivors) = resolve_seeded_landing(
                campaign_seed,
                battle_year,
                coords,
                COMBAT_KIND_BLITZ_GROUND,
                empire,
                previous_owner,
                armies_after_landing,
                defender_armies,
            );
            let defender_battery_losses = pre_batteries.saturating_sub(
                u8::try_from(game_data.planets[planet_idx].ground_batteries).unwrap_or(u8::MAX),
            );
            let defender_army_losses = pre_armies.saturating_sub(defender_survivors.min(255) as u8);

            for &idx in &fleets {
                set_armies_aboard(&mut game_data.fleets[idx], 0);
            }
            if attacker_survivors > 0 && defender_survivors == 0 {
                let batteries = game_data.planets[planet_idx].ground_batteries;
                let inherited = capture_planet(
                    &mut game_data.planets[planet_idx],
                    empire,
                    attacker_survivors,
                    batteries,
                );
                events
                    .ownership_change_events
                    .push(PlanetOwnershipChangeEvent {
                        planet_idx,
                        reporting_empire: previous_owner,
                        previous_owner_empire: previous_owner,
                        new_owner_empire: empire,
                        stardate_week: None,
                    });
                events
                    .mission_events
                    .push(mission_event(MissionOutcome::Succeeded, None));
                events.assault_report_events.push(AssaultReportEvent {
                    kind: Mission::BlitzWorld,
                    attacker_fleet_number: preferred_reporting_fleet_number(game_data, &fleets),
                    planet_idx,
                    attacker_empire: empire,
                    defender_empire: previous_owner,
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
                    inherited: Some(inherited),
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
                game_data.planets[planet_idx].armies = u16::from(defender_survivors.min(255) as u8);
                events
                    .mission_events
                    .push(mission_event(MissionOutcome::Failed, None));
                events.assault_report_events.push(AssaultReportEvent {
                    kind: Mission::BlitzWorld,
                    attacker_fleet_number: preferred_reporting_fleet_number(game_data, &fleets),
                    planet_idx,
                    attacker_empire: empire,
                    defender_empire: previous_owner,
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
                    inherited: None,
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

            let intel_source = if game_data.planets[planet_idx].owner.0 == empire {
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
            let owner_after = game_data.planets[planet_idx].owner.0;
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
