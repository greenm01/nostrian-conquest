use std::collections::{BTreeSet, HashMap, HashSet};

use crate::{
    CoreGameData, EncounterDispositionEvent, EncounterDispositionReason, FleetBattleEvent,
    FleetDestroyedEvent, FleetOrderValidationError, MissionAbortReason, MissionEvent,
    MissionOutcome, Order, PlanetIntelEvent, ScoutContactEvent, ShipLosses, StarbaseDestroyedEvent,
    StarbaseSightingEvent,
};

use super::super::FleetBattlePerspective;
use super::exchange::{
    COMBAT_GUARDRAIL_MAX_ROUNDS, COMBAT_KIND_FLEET, COMBAT_KIND_WITHDRAWAL, RoundAction,
    RoundActionKind, apply_hits_to_fleet, fleet_die_modifier, fleet_state_changed, fleet_volley,
    rule_threshold_satisfied, seeded_combat_roll,
};
use super::reporting::{
    loaded_armies_for_fleet_indices, mission_kind_for_order, preferred_reporting_fleet_index,
    single_named_fleet_number,
};
use super::retreat::{
    dominant_empire_after_battle, retreat_fleet, retreat_task_force,
    set_fleet_to_hold_current_position,
};
use super::state::{
    BattleRole, FleetCombatState, IDX_SB, TaskForce, new_task_force, planet_idx_at_coords,
    ship_counts_from_state, ship_losses_from_states, tf_has_any_units,
};

fn hostile_target_priority(
    our_empire: u8,
    candidates: &[&TaskForce],
    planet_owner: Option<u8>,
) -> Option<u8> {
    candidates
        .iter()
        .copied()
        .filter(|tf| tf.empire != our_empire && tf.state.has_units())
        .min_by_key(|tf| {
            let guarding = matches!(
                tf.role,
                BattleRole::IncumbentDefender | BattleRole::GuardingDefender
            );
            let threatens_ours =
                planet_owner == Some(our_empire) && matches!(tf.role, BattleRole::Attacker);
            (
                !guarding,
                !threatens_ours,
                std::cmp::Reverse(tf.state.total_combat_as()),
                tf.empire,
            )
        })
        .map(|tf| tf.empire)
}

fn defending_starbase_holds_planet(task_force: &TaskForce) -> bool {
    matches!(task_force.role, BattleRole::IncumbentDefender) && task_force.state.counts[IDX_SB] > 0
}

fn task_force_can_fire(task_force: &TaskForce) -> bool {
    task_force.state.has_units()
        && task_force.state.total_combat_as() > 0
        && !task_force.withdrew_under_roe
}

/// The live opponents `actor` is fighting: every other side still in the
/// battle. Withdrawn forces are out of the battle and are never targeted.
fn engaged_opponents<'a>(actor: &TaskForce, task_forces: &'a [TaskForce]) -> Vec<&'a TaskForce> {
    task_forces
        .iter()
        .filter(|other| {
            other.empire != actor.empire && !other.withdrew_under_roe && other.state.has_units()
        })
        .collect()
}

fn battle_has_live_hostile_targets(task_forces: &[TaskForce]) -> bool {
    task_forces
        .iter()
        .filter(|tf| task_force_can_fire(tf))
        .any(|actor| !engaged_opponents(actor, task_forces).is_empty())
}

/// One side's d10 for an exchange, from the combat kind, the round, and the
/// acting and target empires. Maintenance rolls seeded dice.
type CombatRoll<'a> = &'a dyn Fn(u64, u32, u8, u8) -> u8;

/// The withdrawal exchange: both sides fire once on the flat table, without
/// die modifiers.
fn apply_withdrawal_exchanges(
    task_forces: &[TaskForce],
    exchange_pairs: &[(u8, u8)],
    roll: CombatRoll,
    withdrawal_kind: u64,
    round: u32,
    pending_suppression: &mut HashMap<u8, u32>,
    pending_execution: &mut HashMap<u8, u32>,
    pending_criticals: &mut HashMap<u8, u32>,
) {
    if exchange_pairs.is_empty() {
        return;
    }

    let round_start_states: HashMap<u8, FleetCombatState> = task_forces
        .iter()
        .map(|tf| (tf.empire, tf.state.clone()))
        .collect();
    let mut reciprocal_withdrawal_replies = HashSet::new();

    let mut ordered_pairs = exchange_pairs.to_vec();
    ordered_pairs.sort_unstable();
    for (empire, target_empire) in ordered_pairs {
        if reciprocal_withdrawal_replies.remove(&(empire, target_empire)) {
            continue;
        }

        let Some(actor_state) = round_start_states.get(&empire) else {
            continue;
        };
        let Some(target_state) = round_start_states.get(&target_empire) else {
            continue;
        };

        let our_as = actor_state.total_combat_as();
        if our_as == 0 {
            continue;
        }

        let outbound = fleet_volley(
            roll(withdrawal_kind, round, empire, target_empire),
            0,
            our_as,
        );

        let sup_hits;
        let exe_hits;
        if outbound.hits > 0 {
            let sup_ratio = actor_state.suppression_as() as f64 / our_as as f64;
            sup_hits = (outbound.hits as f64 * sup_ratio).round() as u32;
            exe_hits = outbound.hits.saturating_sub(sup_hits);
        } else {
            sup_hits = 0;
            exe_hits = 0;
        }
        *pending_suppression.entry(target_empire).or_default() += sup_hits;
        *pending_execution.entry(target_empire).or_default() += exe_hits;
        *pending_criticals.entry(target_empire).or_default() += u32::from(outbound.critical);

        let target_as = target_state.total_combat_as();
        if target_as > 0 {
            let reply = fleet_volley(
                roll(withdrawal_kind, round, target_empire, empire),
                0,
                target_as,
            );

            let sup_hits;
            let exe_hits;
            if reply.hits > 0 {
                let sup_ratio = target_state.suppression_as() as f64 / target_as as f64;
                sup_hits = (reply.hits as f64 * sup_ratio).round() as u32;
                exe_hits = reply.hits.saturating_sub(sup_hits);
            } else {
                sup_hits = 0;
                exe_hits = 0;
            }
            *pending_suppression.entry(empire).or_default() += sup_hits;
            *pending_execution.entry(empire).or_default() += exe_hits;
            *pending_criticals.entry(empire).or_default() += u32::from(reply.critical);
            reciprocal_withdrawal_replies.insert((target_empire, empire));
        }
    }
}

/// Apply one pass of exchanges to every side. A side's carried hits join the
/// new hits of the same fire pool, and what a pool cannot spend on a damage
/// step stays carried in that pool for the side's next exchange in this
/// battle.
fn apply_pending_hits(
    task_forces: &mut [TaskForce],
    pending_suppression: &HashMap<u8, u32>,
    pending_execution: &HashMap<u8, u32>,
    pending_criticals: &HashMap<u8, u32>,
) {
    for tf in task_forces {
        let sup_hits =
            tf.carried_hits.suppression + pending_suppression.get(&tf.empire).copied().unwrap_or(0);
        let exe_hits =
            tf.carried_hits.execution + pending_execution.get(&tf.empire).copied().unwrap_or(0);
        let critical_hits = pending_criticals.get(&tf.empire).copied().unwrap_or(0);
        if sup_hits > 0 || exe_hits > 0 || critical_hits > 0 {
            tf.carried_hits = apply_hits_to_fleet(&mut tf.state, sup_hits, exe_hits, critical_hits);
        }
    }
}

pub(super) fn distribute_fleet_losses(
    game_data: &mut CoreGameData,
    fleet_indices: &[usize],
    before: &FleetCombatState,
    after: &FleetCombatState,
) {
    let losses = [
        before.counts[super::state::IDX_DD].saturating_sub(after.counts[super::state::IDX_DD]),
        before.counts[super::state::IDX_CA].saturating_sub(after.counts[super::state::IDX_CA]),
        before.counts[super::state::IDX_BB].saturating_sub(after.counts[super::state::IDX_BB]),
        before.counts[super::state::IDX_SC].saturating_sub(after.counts[super::state::IDX_SC]),
        before.counts[super::state::IDX_TT].saturating_sub(after.counts[super::state::IDX_TT]),
        before.counts[super::state::IDX_ET].saturating_sub(after.counts[super::state::IDX_ET]),
    ];

    let mut remaining = losses;
    for &idx in fleet_indices {
        let fleet = &mut game_data.fleets[idx];

        let dd_loss = remaining[0].min(fleet.destroyer_count() as u32) as u16;
        fleet.set_destroyer_count(fleet.destroyer_count().saturating_sub(dd_loss));
        remaining[0] -= dd_loss as u32;

        let ca_loss = remaining[1].min(fleet.cruiser_count() as u32) as u16;
        fleet.set_cruiser_count(fleet.cruiser_count().saturating_sub(ca_loss));
        remaining[1] -= ca_loss as u32;

        let bb_loss = remaining[2].min(fleet.battleship_count() as u32) as u16;
        fleet.set_battleship_count(fleet.battleship_count().saturating_sub(bb_loss));
        remaining[2] -= bb_loss as u32;

        let sc_loss = remaining[3].min(fleet.scout_count() as u32) as u16;
        fleet.set_scout_count(fleet.scout_count().saturating_sub(sc_loss));
        remaining[3] -= sc_loss as u32;

        // Transport losses fall on loaded transports first, each taking its
        // army with it, then on empty ones.
        let tt_loss = remaining[4].min(fleet.troop_transport_count() as u32) as u16;
        let loaded_loss = tt_loss.min(fleet.loaded_transport_count());
        fleet.set_loaded_transport_count(fleet.loaded_transport_count() - loaded_loss);
        fleet.set_empty_transport_count(fleet.empty_transport_count() - (tt_loss - loaded_loss));
        remaining[4] -= tt_loss as u32;

        let et_loss = remaining[5].min(fleet.etac_count() as u32) as u16;
        fleet.set_etac_count(fleet.etac_count().saturating_sub(et_loss));
        remaining[5] -= et_loss as u32;

        // A survivor moves at the speed of the ships it has left (classic
        // recomputes it after the battle too, `24a3:036b`). A fleet with
        // none left is culled, not re-rated.
        let lost_ships = dd_loss + ca_loss + bb_loss + sc_loss + tt_loss + et_loss > 0;
        if lost_ships && fleet.has_any_force() {
            fleet.recompute_max_speed_from_composition();
        }
    }
}

/// Unlink and zero the first `destroyed` of a side's starbase records and
/// return their ids.
fn remove_destroyed_starbases(
    game_data: &mut CoreGameData,
    bases: &[usize],
    destroyed: u32,
) -> Vec<u16> {
    let mut destroyed_ids = Vec::new();
    for &idx in bases.iter().take(destroyed as usize) {
        destroyed_ids.push(game_data.bases[idx].base_id_raw());
        game_data.unlink_base(idx);
        game_data.bases[idx] = nc_data::Starbase::default();
    }
    destroyed_ids
}

fn is_ship_loss_abort_reason(order: Order, reason: FleetOrderValidationError) -> bool {
    matches!(
        (order, reason),
        (
            Order::BombardWorld,
            FleetOrderValidationError::MissingCombatShips
        ) | (
            Order::InvadeWorld,
            FleetOrderValidationError::MissingCombatShips
                | FleetOrderValidationError::MissingLoadedTroopTransports,
        ) | (
            Order::BlitzWorld,
            FleetOrderValidationError::MissingLoadedTroopTransports,
        ) | (
            Order::ScoutSector | Order::ScoutSolarSystem,
            FleetOrderValidationError::MissingScoutShip,
        )
    )
}

fn abort_invalid_dominant_missions_after_battle(
    game_data: &mut CoreGameData,
    events: &mut FleetBattlePhaseEvents,
    task_forces: &[TaskForce],
    dominant_empire: Option<u8>,
    pre_retreat_orders: &HashMap<usize, Order>,
    coords: [u8; 2],
) {
    let Some(dominant_empire) = dominant_empire else {
        return;
    };

    for task_force in task_forces {
        if task_force.empire != dominant_empire || task_force.withdrew_under_roe {
            continue;
        }
        for &fleet_idx in &task_force.fleet_indices {
            let Some(order) = pre_retreat_orders.get(&fleet_idx).copied() else {
                continue;
            };
            let Some(kind) = mission_kind_for_order(Some(order)) else {
                continue;
            };
            let target_coords = game_data.fleets[fleet_idx].standing_order_target_coords_raw();
            let validation = game_data.validate_fleet_order_payload(
                fleet_idx + 1,
                order.to_raw(),
                target_coords,
                None,
                None,
            );
            let Err(reason) = validation else {
                continue;
            };
            if !is_ship_loss_abort_reason(order, reason) {
                continue;
            }

            let owner_empire_raw = game_data.fleets[fleet_idx].owner_empire_raw();
            retreat_fleet(game_data, fleet_idx);
            events.mission_events.push(MissionEvent {
                fleet_idx,
                owner_empire_raw,
                kind,
                outcome: MissionOutcome::Aborted,
                abort_reason: Some(MissionAbortReason::CapabilityLost),
                planet_idx: planet_idx_at_coords(game_data, coords),
                location_coords: Some(coords),
                target_coords: Some(game_data.fleets[fleet_idx].standing_order_target_coords_raw()),
                stardate_week: None,
            });
        }
    }
}

fn emit_aborted_mission_events_for_task_force(
    game_data: &CoreGameData,
    events: &mut FleetBattlePhaseEvents,
    fleet_indices: &[usize],
    pre_orders: &HashMap<usize, Order>,
    coords: [u8; 2],
    abort_reason: Option<MissionAbortReason>,
) {
    for &fleet_idx in fleet_indices {
        let Some(kind) = mission_kind_for_order(pre_orders.get(&fleet_idx).copied()) else {
            continue;
        };
        let fleet = &game_data.fleets[fleet_idx];
        events.mission_events.push(MissionEvent {
            fleet_idx,
            owner_empire_raw: fleet.owner_empire_raw(),
            kind,
            outcome: MissionOutcome::Aborted,
            abort_reason,
            planet_idx: planet_idx_at_coords(game_data, coords),
            location_coords: Some(coords),
            target_coords: Some(fleet.standing_order_target_coords_raw()),
            stardate_week: None,
        });
    }
}

fn task_force_has_tt_only_blitz_assault(
    game_data: &CoreGameData,
    task_force: &TaskForce,
    pre_orders: &HashMap<usize, Order>,
) -> bool {
    task_force.state.total_combat_as() == 0
        && task_force.state.has_units()
        && task_force.fleet_indices.iter().any(|&fleet_idx| {
            pre_orders.get(&fleet_idx) == Some(&Order::BlitzWorld) && {
                let fleet = &game_data.fleets[fleet_idx];
                fleet.troop_transport_count() > 0 && fleet.army_count() > 0
            }
        })
}

fn tt_only_blitz_followthrough_empire(
    game_data: &CoreGameData,
    task_forces: &[TaskForce],
    pre_orders: &HashMap<usize, Order>,
) -> Option<u8> {
    if task_forces.is_empty()
        || task_forces
            .iter()
            .any(|tf| tf.withdrew_under_roe || tf.state.total_combat_as() > 0)
        || task_forces.iter().any(defending_starbase_holds_planet)
    {
        return None;
    }

    let mut candidate_empires = task_forces
        .iter()
        .filter(|tf| tf.state.has_units())
        .filter_map(|tf| {
            tf.fleet_indices
                .iter()
                .any(|&fleet_idx| {
                    pre_orders.get(&fleet_idx) == Some(&Order::BlitzWorld) && {
                        let fleet = &game_data.fleets[fleet_idx];
                        fleet.troop_transport_count() > 0 && fleet.army_count() > 0
                    }
                })
                .then_some(tf.empire)
        })
        .collect::<Vec<_>>();
    candidate_empires.sort_unstable();
    candidate_empires.dedup();
    match candidate_empires.as_slice() {
        [empire] => Some(*empire),
        _ => None,
    }
}

#[derive(Debug, Default)]
pub(crate) struct FleetBattlePhaseEvents {
    pub fleet_battle_events: Vec<FleetBattleEvent>,
    pub fleet_destroyed_events: Vec<FleetDestroyedEvent>,
    pub starbase_destroyed_events: Vec<StarbaseDestroyedEvent>,
    pub scout_contact_events: Vec<ScoutContactEvent>,
    pub starbase_sighting_events: Vec<StarbaseSightingEvent>,
    pub planet_intel_events: Vec<PlanetIntelEvent>,
    pub encounter_disposition_events: Vec<EncounterDispositionEvent>,
    pub mission_events: Vec<MissionEvent>,
}

impl FleetBattlePhaseEvents {
    pub(crate) fn append(&mut self, mut other: Self) {
        self.fleet_battle_events
            .append(&mut other.fleet_battle_events);
        self.fleet_destroyed_events
            .append(&mut other.fleet_destroyed_events);
        self.starbase_destroyed_events
            .append(&mut other.starbase_destroyed_events);
        self.scout_contact_events
            .append(&mut other.scout_contact_events);
        self.starbase_sighting_events
            .append(&mut other.starbase_sighting_events);
        self.planet_intel_events
            .append(&mut other.planet_intel_events);
        self.encounter_disposition_events
            .append(&mut other.encounter_disposition_events);
        self.mission_events.append(&mut other.mission_events);
    }
}

/// One side of an encounter battle: one empire's fleets and starbases.
#[derive(Debug, Clone, Default)]
pub(crate) struct BattleSide {
    pub empire: u8,
    pub fleets: Vec<usize>,
    pub bases: Vec<usize>,
}

/// Fight one encounter battle between exactly two sides at `coords`. The
/// attacker is the fleet whose encounter check started it; nobody else in
/// the sector joins. `battle_key` tells apart battles fought at the same
/// place in one year.
///
/// This is Rust's battle (seeded flat effectiveness table, carried hits,
/// round-4 withdrawals, post-battle retreat), not classic's, which is not
/// decoded yet; it is the seam where classic's battle will go.
pub(crate) fn fight_encounter(
    game_data: &mut CoreGameData,
    campaign_seed: u64,
    battle_key: u64,
    coords: [u8; 2],
    attacker: &BattleSide,
    defender: &BattleSide,
) -> FleetBattlePhaseEvents {
    let battle_year = game_data.conquest.game_year();
    let roll = |combat_kind, round, acting_empire, target_empire| {
        seeded_combat_roll(
            campaign_seed,
            battle_year,
            coords,
            combat_kind,
            round,
            acting_empire,
            target_empire,
        )
    };
    fight_encounter_with_rolls(game_data, &roll, battle_key, coords, attacker, defender)
}

/// `fight_encounter` with the dice supplied by the caller.
pub(super) fn fight_encounter_with_rolls(
    game_data: &mut CoreGameData,
    roll: CombatRoll,
    battle_key: u64,
    coords: [u8; 2],
    attacker: &BattleSide,
    defender: &BattleSide,
) -> FleetBattlePhaseEvents {
    let mut events = FleetBattlePhaseEvents::default();
    let combat_kind = COMBAT_KIND_FLEET | battle_key << 8;
    let withdrawal_kind = COMBAT_KIND_WITHDRAWAL | battle_key << 8;
    let mut sides = [attacker, defender];
    sides.sort_by_key(|side| side.empire);
    let mut task_forces: Vec<TaskForce> = sides
        .iter()
        .map(|side| {
            new_task_force(
                game_data,
                side.empire,
                coords,
                side.fleets.clone(),
                side.bases.len() as u32,
            )
        })
        .collect();
    let bases_by_empire: HashMap<u8, Vec<usize>> = sides
        .iter()
        .map(|side| (side.empire, side.bases.clone()))
        .collect();
    let pre_encounter_orders: HashMap<usize, Order> = task_forces
        .iter()
        .flat_map(|tf| tf.fleet_indices.iter().copied())
        .map(|idx| (idx, game_data.fleets[idx].standing_order_kind()))
        .collect();
    let participants: Vec<u8> = task_forces
        .iter()
        .filter(|tf| tf.state.has_units())
        .map(|tf| tf.empire)
        .collect();
    if participants.len() < 2 {
        return events;
    }

    let original_states: HashMap<u8, FleetCombatState> = task_forces
        .iter()
        .map(|tf| (tf.empire, tf.state.clone()))
        .collect();
    let original_loaded_armies: HashMap<u8, u32> = task_forces
        .iter()
        .map(|tf| {
            (
                tf.empire,
                loaded_armies_for_fleet_indices(game_data, &tf.fleet_indices),
            )
        })
        .collect();
    let original_fleet_states: HashMap<usize, ShipLosses> = task_forces
        .iter()
        .flat_map(|tf| tf.fleet_indices.iter().copied())
        .map(|idx| {
            let fleet = &game_data.fleets[idx];
            (
                idx,
                ShipLosses {
                    destroyers: u32::from(fleet.destroyer_count()),
                    cruisers: u32::from(fleet.cruiser_count()),
                    battleships: u32::from(fleet.battleship_count()),
                    scouts: u32::from(fleet.scout_count()),
                    transports: u32::from(fleet.troop_transport_count()),
                    etacs: u32::from(fleet.etac_count()),
                },
            )
        })
        .collect();
    let original_fleet_loaded_armies: HashMap<usize, u32> = task_forces
        .iter()
        .flat_map(|tf| tf.fleet_indices.iter().copied())
        .map(|idx| (idx, u32::from(game_data.fleets[idx].army_count())))
        .collect();

    let planet_owner = game_data
        .planets
        .iter()
        .find(|p| p.coords_raw() == coords)
        .map(|p| p.owner_empire_slot_raw());
    let hostile_pairs: BTreeSet<(u8, u8)> = [
        (attacker.empire, defender.empire),
        (defender.empire, attacker.empire),
    ]
    .into_iter()
    .collect();

    let mut combat_occurred = false;

    // A battle still undecided at the round cap ends as a stalemate: the
    // survivors keep their ships and the post-battle rules below apply.
    for round in 1..=COMBAT_GUARDRAIL_MAX_ROUNDS {
        if !battle_has_live_hostile_targets(&task_forces) {
            break;
        }

        let combat_as_map: HashMap<u8, u32> = task_forces
            .iter()
            .map(|tf| (tf.empire, tf.state.total_combat_as()))
            .collect();
        let mut actions = Vec::new();
        for tf in &task_forces {
            let our_as = *combat_as_map.get(&tf.empire).unwrap_or(&0);
            if our_as == 0 || tf.withdrew_under_roe || !tf.state.has_units() {
                continue;
            }
            let hostile_opponents = engaged_opponents(tf, &task_forces);
            let enemy_as = hostile_opponents
                .iter()
                .map(|other| other.state.total_combat_as())
                .max()
                .unwrap_or(0);
            let Some(target_empire) =
                hostile_target_priority(tf.empire, &hostile_opponents, planet_owner)
            else {
                continue;
            };
            let roe = tf
                .fleet_indices
                .iter()
                .filter_map(|idx| {
                    let fleet = &game_data.fleets[*idx];
                    (fleet.destroyer_count() > 0
                        || fleet.cruiser_count() > 0
                        || fleet.battleship_count() > 0)
                        .then_some(fleet.rules_of_engagement())
                })
                .max()
                .unwrap_or(0);
            let kind = if defending_starbase_holds_planet(tf) {
                RoundActionKind::Fight
            } else if round > 3 && !rule_threshold_satisfied(roe, our_as, enemy_as) {
                RoundActionKind::Withdraw
            } else {
                RoundActionKind::Fight
            };
            actions.push(RoundAction {
                empire: tf.empire,
                target_empire,
                kind,
            });
        }

        if actions.is_empty() {
            break;
        }

        let before_round_states: HashMap<u8, FleetCombatState> = task_forces
            .iter()
            .map(|tf| (tf.empire, tf.state.clone()))
            .collect();
        let mut engaged_empires = HashSet::new();
        let mut pre_round_withdrawals = HashSet::new();
        let mut pending_suppression: HashMap<u8, u32> = HashMap::new();
        let mut pending_execution: HashMap<u8, u32> = HashMap::new();
        let mut pending_criticals: HashMap<u8, u32> = HashMap::new();
        let mut pre_round_withdrawal_pairs = Vec::new();
        let mut suppressed_reciprocal_actions = HashSet::new();

        // Withdrawals are declared before any volley resolves, so a force
        // breaking off takes only the withdrawal exchange from the
        // opponent it leaves, whatever the empire slot order.
        let mut ordered_actions = actions;
        ordered_actions.sort_by_key(|action| action.empire);
        for action in &ordered_actions {
            if action.kind == RoundActionKind::Withdraw {
                engaged_empires.insert(action.empire);
                engaged_empires.insert(action.target_empire);
                pre_round_withdrawals.insert(action.empire);
                pre_round_withdrawal_pairs.push((action.empire, action.target_empire));
                suppressed_reciprocal_actions.insert((action.target_empire, action.empire));
            }
        }
        for action in ordered_actions {
            if action.kind != RoundActionKind::Fight
                || suppressed_reciprocal_actions.contains(&(action.empire, action.target_empire))
            {
                continue;
            }
            let Some(actor_tf) = task_forces.iter().find(|tf| tf.empire == action.empire) else {
                continue;
            };
            let our_as = actor_tf.state.total_combat_as();
            if our_as == 0 {
                continue;
            }
            engaged_empires.insert(action.empire);
            engaged_empires.insert(action.target_empire);

            let result = fleet_volley(
                roll(combat_kind, round, action.empire, action.target_empire),
                fleet_die_modifier(&actor_tf.state),
                our_as,
            );

            let sup_hits;
            let exe_hits;
            if result.hits > 0 {
                let sup_ratio = actor_tf.state.suppression_as() as f64 / our_as as f64;
                sup_hits = (result.hits as f64 * sup_ratio).round() as u32;
                exe_hits = result.hits.saturating_sub(sup_hits);
            } else {
                sup_hits = 0;
                exe_hits = 0;
            }
            *pending_suppression.entry(action.target_empire).or_default() += sup_hits;
            *pending_execution.entry(action.target_empire).or_default() += exe_hits;
            *pending_criticals.entry(action.target_empire).or_default() +=
                u32::from(result.critical);
        }

        apply_withdrawal_exchanges(
            &task_forces,
            &pre_round_withdrawal_pairs,
            roll,
            withdrawal_kind,
            round,
            &mut pending_suppression,
            &mut pending_execution,
            &mut pending_criticals,
        );

        for empire in engaged_empires {
            if let Some(task_force) = task_forces.iter_mut().find(|tf| tf.empire == empire) {
                task_force.engaged_in_battle = true;
            }
        }

        apply_pending_hits(
            &mut task_forces,
            &pending_suppression,
            &pending_execution,
            &pending_criticals,
        );

        // A withdrawn force gets its retreat order after the battle, once
        // its losses are on the records and its speed is known.
        let mut any_withdrawal = false;
        for empire in pre_round_withdrawals {
            if let Some(task_force) = task_forces.iter_mut().find(|tf| tf.empire == empire) {
                task_force.withdrew_under_roe = true;
                any_withdrawal = true;
            }
        }

        let any_round_state_change = task_forces.iter().any(|tf| {
            before_round_states
                .get(&tf.empire)
                .is_some_and(|before| fleet_state_changed(before, &tf.state))
        });
        combat_occurred |= any_round_state_change || any_withdrawal;

        if !battle_has_live_hostile_targets(&task_forces) {
            break;
        }

        let mut post_round_retreats = Vec::new();
        let mut free_holds_to_consume = Vec::new();

        // Evaluate post-round retreats sequentially by ROE level (ascending).
        let mut evaluation_order: Vec<(u8, u8)> = task_forces
            .iter()
            .filter(|tf| tf.engaged_in_battle && !tf.withdrew_under_roe && tf.state.has_units())
            .map(|tf| {
                let max_roe = tf
                    .fleet_indices
                    .iter()
                    .filter_map(|&idx| {
                        let fleet = &game_data.fleets[idx];
                        (fleet.destroyer_count() > 0
                            || fleet.cruiser_count() > 0
                            || fleet.battleship_count() > 0)
                            .then_some(fleet.rules_of_engagement())
                    })
                    .max()
                    .unwrap_or(0);
                (tf.empire, max_roe)
            })
            .collect();
        evaluation_order.sort_by_key(|&(empire, roe)| (roe, empire));

        for (empire_raw, roe) in evaluation_order {
            let Some(tf_idx) = task_forces.iter().position(|tf| tf.empire == empire_raw) else {
                continue;
            };

            let our_as = task_forces[tf_idx].state.total_combat_as();
            if our_as == 0 {
                continue;
            }

            let hostile_opponents = engaged_opponents(&task_forces[tf_idx], &task_forces);

            let enemy_as = hostile_opponents
                .iter()
                .map(|other| other.state.total_combat_as())
                .max()
                .unwrap_or(0);

            if enemy_as == 0 {
                continue;
            }

            if defending_starbase_holds_planet(&task_forces[tf_idx]) {
                continue;
            }

            if round > 3 && !rule_threshold_satisfied(roe, our_as, enemy_as) {
                let is_guard = matches!(
                    task_forces[tf_idx].role,
                    BattleRole::GuardingDefender | BattleRole::IncumbentDefender
                );
                if is_guard && !task_forces[tf_idx].free_hold_used {
                    free_holds_to_consume.push(empire_raw);
                    continue;
                }
                let Some(target_empire) =
                    hostile_target_priority(empire_raw, &hostile_opponents, planet_owner)
                else {
                    continue;
                };
                // Mark as withdrawn immediately for the benefit of subsequent fleets in the evaluation loop.
                task_forces[tf_idx].withdrew_under_roe = true;
                post_round_retreats.push((empire_raw, target_empire));
            }
        }

        for empire in free_holds_to_consume {
            if let Some(task_force) = task_forces.iter_mut().find(|tf| tf.empire == empire) {
                task_force.free_hold_used = true;
            }
        }

        // Carried hits are spent only in an exchange, so this pass runs only
        // when someone withdraws.
        if !post_round_retreats.is_empty() {
            let mut pending_suppression: HashMap<u8, u32> = HashMap::new();
            let mut pending_execution: HashMap<u8, u32> = HashMap::new();
            let mut pending_criticals: HashMap<u8, u32> = HashMap::new();

            apply_withdrawal_exchanges(
                &task_forces,
                &post_round_retreats,
                roll,
                withdrawal_kind,
                round,
                &mut pending_suppression,
                &mut pending_execution,
                &mut pending_criticals,
            );

            apply_pending_hits(
                &mut task_forces,
                &pending_suppression,
                &pending_execution,
                &pending_criticals,
            );
        }

        combat_occurred |= !post_round_retreats.is_empty();

        if !battle_has_live_hostile_targets(&task_forces) {
            break;
        }
    }

    // Only forces that actually exchanged fire can win, lose, or be
    // reported in this battle. Bystanders and forces whose contact never
    // led to an exchange (for example, unarmed fleets meeting) keep their
    // missions.
    let winner_empire = {
        let mut survivors: Vec<&TaskForce> = task_forces
            .iter()
            .filter(|tf| {
                tf.engaged_in_battle
                    && tf.state.has_units()
                    && tf.state.total_combat_as() > 0
                    && !tf.withdrew_under_roe
            })
            .collect();
        if survivors.len() == 1 {
            Some(survivors[0].empire)
        } else if survivors.is_empty() {
            None
        } else {
            survivors.sort_by_key(|tf| {
                (
                    match tf.role {
                        BattleRole::IncumbentDefender => 0u8,
                        BattleRole::GuardingDefender => 1u8,
                        BattleRole::Attacker => 2u8,
                        BattleRole::Neutral => 3u8,
                    },
                    std::cmp::Reverse(tf.state.total_combat_as()),
                    tf.empire,
                )
            });
            Some(survivors[0].empire)
        }
    };
    let dominant_empire = task_forces
        .iter()
        .find(|tf| {
            tf.engaged_in_battle && defending_starbase_holds_planet(tf) && tf.state.has_units()
        })
        .map(|tf| tf.empire)
        .or_else(|| dominant_empire_after_battle(&task_forces, winner_empire));

    let mut destroyed_starbases_by_empire: HashMap<u8, Vec<u16>> = HashMap::new();
    for tf in &task_forces {
        if let Some(before) = original_states.get(&tf.empire) {
            distribute_fleet_losses(game_data, &tf.fleet_indices, before, &tf.state);
            let destroyed_starbases = before.counts[IDX_SB].saturating_sub(tf.state.counts[IDX_SB]);
            if destroyed_starbases > 0 {
                destroyed_starbases_by_empire.insert(
                    tf.empire,
                    remove_destroyed_starbases(
                        game_data,
                        &bases_by_empire[&tf.empire],
                        destroyed_starbases,
                    ),
                );
            }
        }
    }

    let pre_retreat_orders: HashMap<usize, Order> = task_forces
        .iter()
        .flat_map(|tf| tf.fleet_indices.iter().copied())
        .map(|idx| (idx, game_data.fleets[idx].standing_order_kind()))
        .collect();
    let tt_only_blitz_followthrough = (dominant_empire.is_none())
        .then(|| tt_only_blitz_followthrough_empire(game_data, &task_forces, &pre_retreat_orders))
        .flatten();
    // The world's owner held its own orbit: a beaten side was repelled by
    // the orbital defence.
    let repelled_from_orbit =
        dominant_empire.is_some_and(|dominant| dominant != 0 && planet_owner == Some(dominant));

    for tf in &task_forces {
        if tt_only_blitz_followthrough.is_some() {
            continue;
        }
        if tf.engaged_in_battle && Some(tf.empire) != dominant_empire && !tf.withdrew_under_roe {
            let blocked_tt_only_blitz =
                task_force_has_tt_only_blitz_assault(game_data, tf, &pre_retreat_orders);
            if blocked_tt_only_blitz {
                for &fleet_idx in &tf.fleet_indices {
                    set_fleet_to_hold_current_position(&mut game_data.fleets[fleet_idx]);
                }
            } else {
                retreat_task_force(game_data, tf);
            }
            emit_aborted_mission_events_for_task_force(
                game_data,
                &mut events,
                &tf.fleet_indices,
                &pre_retreat_orders,
                coords,
                (blocked_tt_only_blitz || repelled_from_orbit)
                    .then_some(MissionAbortReason::OrbitBlocked),
            );
        }
    }
    abort_invalid_dominant_missions_after_battle(
        game_data,
        &mut events,
        &task_forces,
        dominant_empire,
        &pre_retreat_orders,
        coords,
    );
    for tf in &task_forces {
        if !tf.withdrew_under_roe {
            continue;
        }
        retreat_task_force(game_data, tf);
        emit_aborted_mission_events_for_task_force(
            game_data,
            &mut events,
            &tf.fleet_indices,
            &pre_encounter_orders,
            coords,
            None,
        );
        if !tf_has_any_units(tf) {
            continue;
        }
        let Some(before) = original_states.get(&tf.empire) else {
            continue;
        };
        let losses = ship_losses_from_states(before, &tf.state);
        let mut enemy_before = FleetCombatState::default();
        let mut enemy_after = FleetCombatState::default();
        for other in &task_forces {
            if !hostile_pairs.contains(&(tf.empire, other.empire)) {
                continue;
            }
            if let Some(orig) = original_states.get(&other.empire) {
                for idx in 0..7 {
                    enemy_before.counts[idx] += orig.counts[idx];
                    enemy_after.counts[idx] += other.state.counts[idx];
                }
            }
        }
        let enemy_losses_inflicted = ship_losses_from_states(&enemy_before, &enemy_after);
        let target_empire_raw = task_forces
            .iter()
            .filter(|other| hostile_pairs.contains(&(tf.empire, other.empire)))
            .max_by_key(|other| other.state.total_combat_as())
            .map(|other| other.empire)
            .unwrap_or(0);
        let target_fleet_number = task_forces
            .iter()
            .find(|other| other.empire == target_empire_raw)
            .and_then(|other| single_named_fleet_number(game_data, &other.fleet_indices));
        for &idx in &tf.fleet_indices {
            events
                .encounter_disposition_events
                .push(EncounterDispositionEvent::Retreated {
                    fleet_idx: idx,
                    owner_empire_raw: tf.empire,
                    mission: mission_kind_for_order(pre_encounter_orders.get(&idx).copied()),
                    coords,
                    friendly_initial: original_fleet_states.get(&idx).copied().unwrap_or_default(),
                    friendly_loaded_armies_initial: original_fleet_loaded_armies
                        .get(&idx)
                        .copied()
                        .unwrap_or(0),
                    target_empire_raw,
                    target_fleet_number,
                    enemy_initial: ship_counts_from_state(&enemy_before),
                    retreat_target_coords: game_data.fleets[idx].standing_order_target_coords_raw(),
                    losses_sustained: losses,
                    enemy_losses_inflicted,
                    reason: EncounterDispositionReason::RoeWithdrawal,
                    stardate_week: None,
                });
        }
    }
    let fought_against = |empire: u8, other: &TaskForce| {
        hostile_pairs.contains(&(empire, other.empire))
            && (other.engaged_in_battle || other.withdrew_under_roe)
    };
    for empire in participants {
        if !combat_occurred {
            continue;
        }
        let Some(before) = original_states.get(&empire) else {
            continue;
        };
        let Some(after_tf) = task_forces.iter().find(|tf| tf.empire == empire) else {
            continue;
        };
        if !after_tf.engaged_in_battle {
            continue;
        }
        let after = &after_tf.state;
        let friendly_losses = ship_losses_from_states(before, after);
        let mut enemy_before = FleetCombatState::default();
        let mut enemy_after = FleetCombatState::default();
        for tf in &task_forces {
            if !fought_against(empire, tf) {
                continue;
            }
            if let Some(orig) = original_states.get(&tf.empire) {
                for idx in 0..7 {
                    enemy_before.counts[idx] += orig.counts[idx];
                    enemy_after.counts[idx] += tf.state.counts[idx];
                }
            }
        }
        let enemy_losses = ship_losses_from_states(&enemy_before, &enemy_after);
        let enemy_loaded_armies_initial = task_forces
            .iter()
            .filter(|tf| fought_against(empire, tf))
            .map(|tf| original_loaded_armies.get(&tf.empire).copied().unwrap_or(0))
            .sum();
        let enemy_empires_raw = task_forces
            .iter()
            .filter(|tf| {
                fought_against(empire, tf)
                    && original_states
                        .get(&tf.empire)
                        .is_some_and(FleetCombatState::has_units)
            })
            .map(|tf| tf.empire)
            .collect();
        let reporting_fleet_idx =
            preferred_reporting_fleet_index(game_data, &after_tf.fleet_indices);
        let reporting_fleet_number = reporting_fleet_idx
            .map(|idx| game_data.fleets[idx].local_slot_word_raw() as u8)
            .filter(|fleet_number| *fleet_number != 0);
        let reporting_mission = reporting_fleet_idx
            .and_then(|idx| mission_kind_for_order(pre_encounter_orders.get(&idx).copied()));
        let primary_enemy_fleet_number = task_forces
            .iter()
            .filter(|tf| fought_against(empire, tf) && tf.state.has_units())
            .max_by_key(|tf| tf.state.total_combat_as())
            .and_then(|tf| single_named_fleet_number(game_data, &tf.fleet_indices));
        events.fleet_battle_events.push(FleetBattleEvent {
            reporting_empire_raw: empire,
            reporting_fleet_number,
            reporting_mission,
            perspective: if empire == attacker.empire {
                FleetBattlePerspective::Intercepted
            } else {
                FleetBattlePerspective::Attacked
            },
            coords,
            enemy_empires_raw,
            primary_enemy_fleet_number,
            held_field: dominant_empire == Some(empire),
            friendly_initial: ship_counts_from_state(before),
            friendly_initial_starbases: before.counts[IDX_SB],
            friendly_loaded_armies_initial: original_loaded_armies
                .get(&empire)
                .copied()
                .unwrap_or(0),
            friendly_losses,
            friendly_starbases_lost: before.counts[IDX_SB]
                .saturating_sub(after_tf.state.counts[IDX_SB]),
            enemy_initial: ship_counts_from_state(&enemy_before),
            enemy_initial_starbases: enemy_before.counts[IDX_SB],
            enemy_loaded_armies_initial,
            enemy_losses,
            enemy_starbases_destroyed: enemy_before.counts[IDX_SB]
                .saturating_sub(enemy_after.counts[IDX_SB]),
            stardate_week: None,
        });

        if !tf_has_any_units(after_tf) && !after_tf.fleet_indices.is_empty() {
            let fleet_number = reporting_fleet_number.unwrap_or(0);
            let primary_enemy_empire_raw = task_forces
                .iter()
                .filter(|tf| fought_against(empire, tf))
                .max_by_key(|tf| tf.state.total_combat_as())
                .map(|tf| tf.empire);
            let primary_enemy_fleet_number = task_forces
                .iter()
                .filter(|tf| fought_against(empire, tf))
                .max_by_key(|tf| tf.state.total_combat_as())
                .and_then(|tf| single_named_fleet_number(game_data, &tf.fleet_indices));
            events.fleet_destroyed_events.push(FleetDestroyedEvent {
                reporting_empire_raw: empire,
                fleet_number,
                coords,
                was_intercepting: empire == attacker.empire,
                friendly_initial: ship_counts_from_state(before),
                friendly_loaded_armies_initial: original_loaded_armies
                    .get(&empire)
                    .copied()
                    .unwrap_or(0),
                enemy_initial: ship_counts_from_state(&enemy_before),
                enemy_initial_starbases: enemy_before.counts[IDX_SB],
                enemy_loaded_armies_initial,
                enemy_ground_batteries_initial: 0,
                enemy_ground_armies_initial: 0,
                enemy_losses,
                enemy_starbases_destroyed: enemy_before.counts[IDX_SB]
                    .saturating_sub(enemy_after.counts[IDX_SB]),
                enemy_ground_battery_losses: 0,
                enemy_ground_army_losses: 0,
                primary_enemy_empire_raw,
                primary_enemy_fleet_number,
                stardate_week: None,
            });
        }

        let destroyed_starbases = before.counts[IDX_SB].saturating_sub(after.counts[IDX_SB]);
        if destroyed_starbases > 0 {
            let primary_enemy_empire_raw = task_forces
                .iter()
                .filter(|tf| fought_against(empire, tf))
                .max_by_key(|tf| tf.state.total_combat_as())
                .map(|tf| tf.empire);
            let primary_enemy_fleet_number = task_forces
                .iter()
                .filter(|tf| fought_against(empire, tf))
                .max_by_key(|tf| tf.state.total_combat_as())
                .and_then(|tf| single_named_fleet_number(game_data, &tf.fleet_indices));
            if let Some(lost_ids) = destroyed_starbases_by_empire.get(&empire) {
                for &starbase_id in lost_ids {
                    events
                        .starbase_destroyed_events
                        .push(StarbaseDestroyedEvent {
                            reporting_empire_raw: empire,
                            starbase_id,
                            coords,
                            enemy_initial: ship_counts_from_state(&enemy_before),
                            enemy_loaded_armies_initial,
                            enemy_losses,
                            primary_enemy_empire_raw,
                            primary_enemy_fleet_number,
                            stardate_week: None,
                        });
                }
            }
        }
    }

    events
}
