use std::collections::{BTreeSet, HashMap, HashSet};

use crate::{
    CoreGameData, EncounterDispositionEvent, EncounterDispositionReason, FleetBattleEvent,
    FleetDestroyedEvent, FleetOrderValidationError, MissionAbortReason, MissionEvent,
    MissionOutcome, Order, PlanetIntelEvent, ScoutContactEvent, StarbaseDestroyedEvent,
    StarbaseSightingEvent,
};

use super::super::FleetBattlePerspective;
use super::damage::{Holder, apply_class_change, write_back_base, write_back_fleet};
use super::exchange::{
    BattleDice, COMBAT_GUARDRAIL_MAX_ROUNDS, COMBAT_KIND_ALLOCATION, COMBAT_KIND_FLEET,
    COMBAT_KIND_PURSUIT, COMBAT_KIND_PURSUIT_CATCH, SeededDice, apply_hits_to_fleet,
    fleet_die_modifier, fleet_volley, heavy_line_outweighs, pursuit_catches,
};
use super::reporting::{
    loaded_armies_for_fleet_indices, mission_kind_for_order, preferred_reporting_fleet_index,
    single_named_fleet_number,
};
use super::retreat::{
    dominant_empire_after_battle, escape_fleet, retreat_fleet, retreat_task_force,
};
use super::state::{
    BattleRole, FleetCombatState, IDX_SB, IDX_TT, Member, Record, TaskForce, Transports,
    Withdrawal, new_task_force, planet_idx_at_coords, ship_counts_from_state,
    ship_losses_from_states, tf_has_any_units,
};
use crate::maint::encounter::rules::roe_permits;
use nc_data::{GameRng, has_any_force, troop_transports};

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
    task_force.state.has_units() && task_force.state.total_combat_as() > 0
}

/// The live opponents `actor` is fighting: every other side with a unit
/// still in the battle. Fleets that broke off are out of it and are never
/// targeted.
fn engaged_opponents<'a>(actor: &TaskForce, task_forces: &'a [TaskForce]) -> Vec<&'a TaskForce> {
    task_forces
        .iter()
        .filter(|other| other.empire != actor.empire && other.state.has_units())
        .collect()
}

fn battle_has_live_hostile_targets(task_forces: &[TaskForce]) -> bool {
    task_forces
        .iter()
        .filter(|tf| task_force_can_fire(tf))
        .any(|actor| !engaged_opponents(actor, task_forces).is_empty())
}

/// One exchange of fire: every side still in the battle fires one volley
/// at its target, all from the same moment, and the hits land together
/// with what each side carried. `surprise` names the empire whose side
/// strikes by surprise in this exchange, if any. Returns whether anyone
/// fired.
fn fire_exchange(
    task_forces: &mut [TaskForce],
    dice: &dyn BattleDice,
    kind: u64,
    round: u32,
    planet_owner: Option<u8>,
    surprise: Option<u8>,
) -> bool {
    let mut pending_suppression: HashMap<u8, u32> = HashMap::new();
    let mut pending_execution: HashMap<u8, u32> = HashMap::new();
    let mut pending_criticals: HashMap<u8, u32> = HashMap::new();
    let mut engaged = HashSet::new();
    for tf in task_forces.iter() {
        if !task_force_can_fire(tf) {
            continue;
        }
        let opponents = engaged_opponents(tf, task_forces);
        let Some(target_empire) = hostile_target_priority(tf.empire, &opponents, planet_owner)
        else {
            continue;
        };
        let our_as = tf.state.total_combat_as();
        let enemy_heavy_cost: u32 = opponents
            .iter()
            .filter(|other| other.empire == target_empire)
            .map(|other| other.state.heavy_cost())
            .sum();
        let heavy_line = heavy_line_outweighs(tf.state.heavy_cost(), enemy_heavy_cost);
        let result = fleet_volley(
            dice.d10(kind, round, tf.empire, target_empire),
            fleet_die_modifier(&tf.state, surprise == Some(tf.empire), heavy_line),
            our_as,
        );
        let sup_hits = if result.hits > 0 {
            let sup_ratio = tf.state.suppression_as() as f64 / our_as as f64;
            (result.hits as f64 * sup_ratio).round() as u32
        } else {
            0
        };
        let exe_hits = result.hits.saturating_sub(sup_hits);
        *pending_suppression.entry(target_empire).or_default() += sup_hits;
        *pending_execution.entry(target_empire).or_default() += exe_hits;
        *pending_criticals.entry(target_empire).or_default() += u32::from(result.critical);
        engaged.insert(tf.empire);
        engaged.insert(target_empire);
    }
    for tf in task_forces.iter_mut() {
        if engaged.contains(&tf.empire) {
            tf.engaged_in_battle = true;
        }
    }
    apply_pending_hits(
        task_forces,
        &pending_suppression,
        &pending_execution,
        &pending_criticals,
        dice,
        COMBAT_KIND_ALLOCATION | (kind & !0xff),
        round,
    );
    !engaged.is_empty()
}

/// Apply one pass of exchanges to every side. A side's carried hits join the
/// new hits of the same fire pool, and what a pool cannot spend on a damage
/// step stays carried in that pool for the side's next exchange in this
/// battle. What the hits did to the side is then shared among its fleets.
pub(super) fn apply_pending_hits(
    task_forces: &mut [TaskForce],
    pending_suppression: &HashMap<u8, u32>,
    pending_execution: &HashMap<u8, u32>,
    pending_criticals: &HashMap<u8, u32>,
    dice: &dyn BattleDice,
    allocation_kind: u64,
    round: u32,
) {
    for tf in task_forces {
        let sup_hits =
            tf.carried_hits.suppression + pending_suppression.get(&tf.empire).copied().unwrap_or(0);
        let exe_hits =
            tf.carried_hits.execution + pending_execution.get(&tf.empire).copied().unwrap_or(0);
        let critical_hits = pending_criticals.get(&tf.empire).copied().unwrap_or(0);
        if sup_hits > 0 || exe_hits > 0 || critical_hits > 0 {
            let before = tf.state.clone();
            tf.carried_hits = apply_hits_to_fleet(&mut tf.state, sup_hits, exe_hits, critical_hits);
            let mut rng = dice.allocation(allocation_kind, round, tf.empire);
            share_losses(tf, &before, &mut rng);
        }
    }
}

/// Share what an exchange did to the side, `before` to its state now,
/// among the members still in the battle: class by class, cohort by
/// cohort, in proportion to each member's eligible hulls (combat decision
/// 6), with `rng` breaking ties at the cutoff.
fn share_losses(tf: &mut TaskForce, before: &FleetCombatState, rng: &mut GameRng) {
    let fighting: Vec<usize> = (0..tf.members.len())
        .filter(|&m| tf.members[m].fighting())
        .collect();
    for idx in 0..7 {
        let mut holders: Vec<Holder> = fighting
            .iter()
            .flat_map(|&m| holders_of(&tf.members[m], idx))
            .collect();
        apply_class_change(
            &mut holders,
            (before.counts[idx], &before.crippled[idx]),
            (tf.state.counts[idx], &tf.state.crippled[idx]),
            rng,
        );
        let mut holders = holders.iter();
        for &m in &fighting {
            let member = &mut tf.members[m];
            if idx == IDX_TT {
                let loaded = holders.next().expect("the loaded holder");
                let empty = holders.next().expect("the empty holder");
                member.transports = Transports::from_holders(loaded, empty);
                (member.state.counts[IDX_TT], member.state.crippled[IDX_TT]) =
                    member.transports.total();
            } else {
                let holder = holders.next().expect("the class holder");
                member.state.counts[idx] = holder.hulls;
                member.state.crippled[idx] = holder.crippled;
            }
        }
    }
    debug_assert_eq!(
        tf.state,
        {
            let mut rebuilt = tf.clone();
            rebuilt.rebuild();
            rebuilt.state
        },
        "the members' hulls add up to the side's"
    );
}

/// A member's holders of class `idx`: for transports its two cargo
/// subgroups, loaded first; for any other class its one holder.
fn holders_of(member: &Member, idx: usize) -> Vec<Holder> {
    if idx == IDX_TT {
        member.transports.holders().to_vec()
    } else {
        vec![Holder {
            hulls: member.state.counts[idx],
            crippled: member.state.crippled[idx],
            loaded: false,
        }]
    }
}

/// After an exchange, both sides drop every fleet whose ROE no longer
/// accepts the odds of the sides as they stand, and every fleet with no
/// attack strength left, all at once, recomputing until none fails. A
/// dropped fleet is out for good, even if the odds later favour it; a
/// starbase never leaves. Nobody leaves a battle whose opponents are all
/// gone. Returns the fleets dropped, as (side, member, reason).
fn prune(task_forces: &mut [TaskForce]) -> Vec<(usize, usize, EncounterDispositionReason)> {
    let mut dropped = Vec::new();
    loop {
        let power: Vec<u32> = task_forces
            .iter()
            .map(|tf| tf.state.total_combat_as())
            .collect();
        let mut batch = Vec::new();
        for (k, tf) in task_forces.iter().enumerate() {
            let enemy = task_forces
                .iter()
                .enumerate()
                .filter(|(j, other)| {
                    *j != k && other.empire != tf.empire && other.state.has_units()
                })
                .map(|(j, _)| power[j])
                .max();
            let Some(enemy) = enemy else {
                continue;
            };
            for (m, member) in tf.members.iter().enumerate() {
                if !member.fighting() || member.fleet_idx().is_none() || !member.state.has_units() {
                    continue;
                }
                let reason = if member.state.total_combat_as() == 0 {
                    EncounterDispositionReason::UnarmedEscape
                } else if !roe_permits(member.roe, power[k], enemy) {
                    EncounterDispositionReason::RoeWithdrawal
                } else {
                    continue;
                };
                batch.push((k, m, reason));
            }
        }
        if batch.is_empty() {
            return dropped;
        }
        for &(k, m, reason) in &batch {
            task_forces[k].members[m].left = Some(Withdrawal {
                reason,
                pursued: false,
            });
            task_forces[k].rebuild();
        }
        dropped.extend(batch);
    }
}

/// Side `pursuer` chases the fleets of side `fleeing` that broke off in
/// `round`, `dropped`: it needs armed ships (a starbase cannot chase), and
/// it catches each fleeing fleet on its own, by speed and the catch draw,
/// unless it has already had its one pursuit exchange of that fleet's
/// escape. The fleets caught stand for one more exchange, carried hits
/// included.
#[allow(clippy::too_many_arguments)]
fn pursue(
    game_data: &CoreGameData,
    task_forces: &mut [TaskForce],
    dice: &dyn BattleDice,
    catch_kind: u64,
    pursuit_kind: u64,
    round: u32,
    planet_owner: Option<u8>,
    pursuer: usize,
    fleeing: usize,
    dropped: &[(usize, usize, EncounterDispositionReason)],
) {
    let chaser = &task_forces[pursuer];
    if !chaser.state.has_ships() || chaser.state.total_combat_as() == 0 {
        return;
    }
    let (pursuer_speed, pursuer_empire) = (chaser.state.ship_speed(), chaser.empire);
    let fleeing_empire = task_forces[fleeing].empire;
    let caught: Vec<(usize, EncounterDispositionReason)> = dropped
        .iter()
        .filter(|&&(side, ..)| side == fleeing)
        .filter(|&&(_, m, _)| {
            let member = &task_forces[fleeing].members[m];
            let got_away = member.fleet_idx().is_some_and(|idx| {
                game_data.fleets[idx]
                    .escape
                    .is_some_and(|escape| escape.pursued_by(pursuer_empire))
            });
            !got_away
                && pursuit_catches(
                    pursuer_speed,
                    member.state.ship_speed(),
                    dice.pursuit(
                        catch_kind | (m as u64) << 48,
                        round,
                        pursuer_empire,
                        fleeing_empire,
                    ),
                )
        })
        .map(|&(_, m, reason)| (m, reason))
        .collect();
    if caught.is_empty() {
        return;
    }
    for &(m, _) in &caught {
        task_forces[fleeing].members[m].left = None;
    }
    task_forces[fleeing].rebuild();
    fire_exchange(task_forces, dice, pursuit_kind, round, planet_owner, None);
    for &(m, reason) in &caught {
        task_forces[fleeing].members[m].left = Some(Withdrawal {
            reason,
            pursued: true,
        });
    }
    task_forces[fleeing].rebuild();
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

/// The fleet's losses left it without the ships its order needs. Checked
/// while the fleet still has that order and its target.
fn mission_lost_its_ships(game_data: &CoreGameData, fleet_idx: usize, order: Order) -> bool {
    if mission_kind_for_order(Some(order)).is_none() {
        return false;
    }
    let target_coords = game_data.fleets[fleet_idx].target.to_array();
    match game_data.validate_fleet_order_payload(
        fleet_idx + 1,
        order.code(),
        target_coords,
        None,
        None,
    ) {
        Err(reason) => is_ship_loss_abort_reason(order, reason),
        Ok(_) => false,
    }
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
        if task_force.empire != dominant_empire {
            continue;
        }
        for fleet_idx in fighting_fleets(task_force) {
            let Some(order) = pre_retreat_orders.get(&fleet_idx).copied() else {
                continue;
            };
            if !mission_lost_its_ships(game_data, fleet_idx, order) {
                continue;
            }
            retreat_fleet(game_data, fleet_idx);
            emit_aborted_mission_event(
                game_data,
                events,
                fleet_idx,
                Some(order),
                coords,
                Some(MissionAbortReason::CapabilityLost),
            );
        }
    }
}

/// The fleets of a side still in the battle.
fn fighting_fleets(task_force: &TaskForce) -> impl Iterator<Item = usize> + '_ {
    task_force
        .members
        .iter()
        .filter(|member| member.fighting())
        .filter_map(Member::fleet_idx)
}

/// Report fleet `fleet_idx`'s mission aborted at `coords`, if `order` was a
/// mission, with the fleet's new target as where it goes.
fn emit_aborted_mission_event(
    game_data: &CoreGameData,
    events: &mut FleetBattlePhaseEvents,
    fleet_idx: usize,
    order: Option<Order>,
    coords: [u8; 2],
    abort_reason: Option<MissionAbortReason>,
) {
    let Some(kind) = mission_kind_for_order(order) else {
        return;
    };
    let fleet = &game_data.fleets[fleet_idx];
    events.mission_events.push(MissionEvent {
        fleet_idx,
        owner_empire: fleet.owner.0,
        kind,
        outcome: MissionOutcome::Aborted,
        abort_reason,
        planet_idx: planet_idx_at_coords(game_data, coords),
        location_coords: Some(coords),
        target_coords: Some(fleet.target.to_array()),
        stardate_week: None,
    });
}

fn tt_only_blitz_followthrough_empire(
    game_data: &CoreGameData,
    task_forces: &[TaskForce],
    pre_orders: &HashMap<usize, Order>,
) -> Option<u8> {
    if task_forces.is_empty()
        || task_forces
            .iter()
            .any(|tf| tf.any_left() || tf.state.total_combat_as() > 0)
        || task_forces.iter().any(defending_starbase_holds_planet)
    {
        return None;
    }
    let mut candidate_empires = task_forces
        .iter()
        .filter(|tf| tf.state.has_units())
        .filter_map(|tf| {
            fighting_fleets(tf)
                .any(|fleet_idx| {
                    pre_orders.get(&fleet_idx) == Some(&Order::BlitzWorld) && {
                        let fleet = &game_data.fleets[fleet_idx];
                        troop_transports(fleet) > 0 && fleet.ships.loaded_transports > 0
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

/// What a battle produced: its events, and the empire that held the field,
/// if any did. A battle the round cap ended, or that nobody could win,
/// leaves the field to nobody.
#[derive(Debug, Default)]
pub(crate) struct FleetBattle {
    pub events: FleetBattlePhaseEvents,
    pub held_by: Option<u8>,
}

/// Fight one encounter battle between two empires at `coords`: the
/// attacker's side, its fleets that volunteered, and the defender's roster,
/// the units the encounter assembled. `battle_key` tells apart battles
/// fought at the same place in one year. The attacker strikes by `surprise`
/// when the defending empire had not sighted it there: its first exchange
/// gets +1 on the die, within the modifier cap.
///
/// This is Rust's battle (seeded flat effectiveness table, carried hits,
/// ROE checks after every exchange by every fleet, pursuit of fleets that
/// break off, losses shared among a side's fleets, post-battle retreat),
/// not classic's, which is not decoded yet; it is the seam where classic's
/// battle will go.
pub(crate) fn fight_encounter(
    game_data: &mut CoreGameData,
    campaign_seed: u64,
    battle_key: u64,
    coords: [u8; 2],
    attacker: &BattleSide,
    defender: &BattleSide,
    surprise: bool,
) -> FleetBattle {
    let dice = SeededDice {
        campaign_seed,
        game_year: game_data.conquest.game_year,
        coords,
    };
    fight_encounter_with_dice(
        game_data,
        &dice,
        COMBAT_GUARDRAIL_MAX_ROUNDS,
        battle_key,
        coords,
        attacker,
        defender,
        surprise,
    )
}

/// `fight_encounter` with the dice and the round cap supplied by the caller.
#[allow(clippy::too_many_arguments)]
pub(super) fn fight_encounter_with_dice(
    game_data: &mut CoreGameData,
    dice: &dyn BattleDice,
    round_cap: u32,
    battle_key: u64,
    coords: [u8; 2],
    attacker: &BattleSide,
    defender: &BattleSide,
    surprise: bool,
) -> FleetBattle {
    let mut events = FleetBattlePhaseEvents::default();
    let combat_kind = COMBAT_KIND_FLEET | battle_key << 8;
    let pursuit_kind = COMBAT_KIND_PURSUIT | battle_key << 8;
    let catch_kind = COMBAT_KIND_PURSUIT_CATCH | battle_key << 8;
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
                &side.bases,
            )
        })
        .collect();
    let pre_encounter_orders: HashMap<usize, Order> = task_forces
        .iter()
        .flat_map(|tf| tf.fleet_indices())
        .map(|idx| (idx, game_data.fleets[idx].order))
        .collect();
    let participants: Vec<u8> = task_forces
        .iter()
        .filter(|tf| tf.state.has_units())
        .map(|tf| tf.empire)
        .collect();
    if participants.len() < 2 {
        return FleetBattle {
            events,
            held_by: None,
        };
    }

    // A fleet still escaping from an earlier battle here is attacked as a
    // pursuit: this encounter is the attacker's one pursuit of it, and no
    // pursuit exchange follows if it breaks off again.
    for &idx in &defender.fleets {
        if let Some(escape) = &mut game_data.fleets[idx].escape {
            escape.record_pursuit(attacker.empire);
        }
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
                loaded_armies_for_fleet_indices(game_data, &tf.fleet_indices()),
            )
        })
        .collect();
    let original_member_states: HashMap<usize, FleetCombatState> = task_forces
        .iter()
        .flat_map(|tf| tf.members.iter())
        .filter_map(|member| member.fleet_idx().map(|idx| (idx, member.state.clone())))
        .collect();
    let original_fleet_loaded_armies: HashMap<usize, u32> = task_forces
        .iter()
        .flat_map(|tf| tf.fleet_indices())
        .map(|idx| {
            (
                idx,
                u32::from(game_data.fleets[idx].ships.loaded_transports),
            )
        })
        .collect();

    let planet_owner = game_data
        .planets
        .iter()
        .find(|p| p.sector.to_array() == coords)
        .map(|p| p.owner.0);
    let hostile_pairs: BTreeSet<(u8, u8)> = [
        (attacker.empire, defender.empire),
        (defender.empire, attacker.empire),
    ]
    .into_iter()
    .collect();

    let mut combat_occurred = false;
    let mut capped = false;
    let mut round = 0;

    // The first exchange is compulsory; after every exchange each fleet
    // checks whether it stays. Fleets that break off while their side
    // fights on leave unpursued; when a whole side breaks off and the other
    // stays, the other may catch each fleeing fleet for one more exchange,
    // carried hits included, and when both break off nobody pursues. A
    // battle still undecided at the round cap ends unresolved.
    loop {
        if !battle_has_live_hostile_targets(&task_forces) {
            break;
        }
        if round == round_cap {
            capped = true;
            break;
        }
        round += 1;
        // Only the attacker's first exchange can be a surprise; a pursuit
        // exchange never is.
        let surprise_for = (surprise && round == 1).then_some(attacker.empire);
        combat_occurred |= fire_exchange(
            &mut task_forces,
            dice,
            combat_kind,
            round,
            planet_owner,
            surprise_for,
        );

        let dropped = prune(&mut task_forces);
        if dropped.is_empty() {
            continue;
        }
        combat_occurred = true;
        let broke_off: Vec<usize> = (0..task_forces.len())
            .filter(|&k| {
                !task_forces[k].state.has_units() && dropped.iter().any(|&(side, ..)| side == k)
            })
            .collect();
        match broke_off.as_slice() {
            [] => {}
            &[fleeing] => {
                if let Some(pursuer) = (0..task_forces.len())
                    .find(|&k| k != fleeing && task_forces[k].state.has_units())
                {
                    pursue(
                        game_data,
                        &mut task_forces,
                        dice,
                        catch_kind,
                        pursuit_kind,
                        round,
                        planet_owner,
                        pursuer,
                        fleeing,
                        &dropped,
                    );
                }
                break;
            }
            _ => break,
        }
    }

    // Only forces that actually exchanged fire can win, lose, or be
    // reported in this battle. Bystanders and forces whose contact never
    // led to an exchange (for example, unarmed fleets meeting) keep their
    // missions. At the round cap nobody wins and nobody holds the field.
    let winner_empire = if capped {
        None
    } else {
        let survivors: Vec<&TaskForce> = task_forces
            .iter()
            .filter(|tf| {
                tf.engaged_in_battle && tf.state.has_units() && tf.state.total_combat_as() > 0
            })
            .collect();
        match survivors.as_slice() {
            [survivor] => Some(survivor.empire),
            _ => None,
        }
    };
    let dominant_empire = if capped {
        None
    } else {
        task_forces
            .iter()
            .find(|tf| {
                tf.engaged_in_battle && defending_starbase_holds_planet(tf) && tf.state.has_units()
            })
            .map(|tf| tf.empire)
            .or_else(|| dominant_empire_after_battle(&task_forces, winner_empire))
    };

    // Every record, departed fleets included, as the battle left it.
    let mut destroyed_starbases_by_empire: HashMap<u8, Vec<u16>> = HashMap::new();
    for tf in &task_forces {
        for member in &tf.members {
            match member.record {
                Record::Fleet(idx) => {
                    write_back_fleet(game_data, idx, &member.state, &member.transports);
                    // ECMAINT deletes a fleet where it dies, and its base
                    // lets go of it first (`2895:3d46`).
                    if !has_any_force(&game_data.fleets[idx]) {
                        game_data.release_starbase_escort(idx);
                    }
                }
                Record::Base(idx) => {
                    if let Some(destroyed) = write_back_base(game_data, idx, &member.state) {
                        destroyed_starbases_by_empire
                            .entry(tf.empire)
                            .or_default()
                            .push(destroyed);
                    }
                }
            }
        }
    }
    let final_states: HashMap<u8, FleetCombatState> = task_forces
        .iter()
        .map(|tf| (tf.empire, tf.full_state()))
        .collect();

    let pre_retreat_orders: HashMap<usize, Order> = task_forces
        .iter()
        .flat_map(|tf| tf.fleet_indices())
        .map(|idx| (idx, game_data.fleets[idx].order))
        .collect();
    let tt_only_blitz_followthrough = (dominant_empire.is_none())
        .then(|| tt_only_blitz_followthrough_empire(game_data, &task_forces, &pre_retreat_orders))
        .flatten();
    // The world's owner held its own orbit: a beaten side was repelled by
    // the orbital defence.
    let repelled_from_orbit =
        dominant_empire.is_some_and(|dominant| dominant != 0 && planet_owner == Some(dominant));

    for tf in &task_forces {
        if capped || tt_only_blitz_followthrough.is_some() {
            continue;
        }
        if tf.engaged_in_battle && Some(tf.empire) != dominant_empire {
            retreat_task_force(game_data, tf);
            for fleet_idx in fighting_fleets(tf) {
                emit_aborted_mission_event(
                    game_data,
                    &mut events,
                    fleet_idx,
                    pre_retreat_orders.get(&fleet_idx).copied(),
                    coords,
                    repelled_from_orbit.then_some(MissionAbortReason::OrbitBlocked),
                );
            }
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
    let hostile_state = |empire: u8| {
        let mut before = FleetCombatState::default();
        let mut after = FleetCombatState::default();
        for other in &task_forces {
            if !hostile_pairs.contains(&(empire, other.empire)) {
                continue;
            }
            if let Some(orig) = original_states.get(&other.empire) {
                for idx in 0..7 {
                    before.counts[idx] += orig.counts[idx];
                    after.counts[idx] += final_states[&other.empire].counts[idx];
                }
            }
        }
        (before, after)
    };
    for tf in &task_forces {
        let target_empire_code = task_forces
            .iter()
            .filter(|other| hostile_pairs.contains(&(tf.empire, other.empire)))
            .max_by_key(|other| final_states[&other.empire].total_combat_as())
            .map(|other| other.empire)
            .unwrap_or(0);
        let target_fleet_number = task_forces
            .iter()
            .find(|other| other.empire == target_empire_code)
            .and_then(|other| single_named_fleet_number(game_data, &other.fleet_indices()));
        let (enemy_before, enemy_after) = hostile_state(tf.empire);
        let enemy_losses_inflicted = ship_losses_from_states(&enemy_before, &enemy_after);
        for member in &tf.members {
            let (Some(withdrawal), Some(fleet_idx)) = (member.left, member.fleet_idx()) else {
                continue;
            };
            // What the mission report says: repelled by the orbital defence
            // when the world's owner held the field, or left without the
            // ships the mission needs; checked before the retreat order
            // replaces it.
            let order = pre_encounter_orders.get(&fleet_idx).copied();
            let abort_reason = if repelled_from_orbit {
                Some(MissionAbortReason::OrbitBlocked)
            } else if order.is_some_and(|order| mission_lost_its_ships(game_data, fleet_idx, order))
            {
                Some(MissionAbortReason::CapabilityLost)
            } else {
                None
            };
            let pursuer = withdrawal.pursued.then_some(target_empire_code);
            escape_fleet(game_data, fleet_idx, coords, pursuer);
            emit_aborted_mission_event(
                game_data,
                &mut events,
                fleet_idx,
                order,
                coords,
                abort_reason,
            );
            if !member.state.has_units() {
                continue;
            }
            let losses =
                ship_losses_from_states(&original_member_states[&fleet_idx], &member.state);
            let owner_empire = tf.empire;
            let mission = mission_kind_for_order(order);
            let friendly_initial = ship_counts_from_state(&original_member_states[&fleet_idx]);
            let friendly_loaded_armies_initial = original_fleet_loaded_armies
                .get(&fleet_idx)
                .copied()
                .unwrap_or(0);
            let enemy_initial = ship_counts_from_state(&enemy_before);
            let retreat_target_coords = game_data.fleets[fleet_idx].target.to_array();
            let event = if withdrawal.pursued {
                EncounterDispositionEvent::PursuitFire {
                    fleet_idx,
                    owner_empire,
                    mission,
                    coords,
                    friendly_initial,
                    friendly_loaded_armies_initial,
                    target_empire: target_empire_code,
                    target_fleet_number,
                    enemy_initial,
                    retreat_target_coords,
                    losses_sustained: losses,
                    enemy_losses_inflicted,
                    reason: withdrawal.reason,
                    stardate_week: None,
                }
            } else {
                EncounterDispositionEvent::Retreated {
                    fleet_idx,
                    owner_empire,
                    mission,
                    coords,
                    friendly_initial,
                    friendly_loaded_armies_initial,
                    target_empire: target_empire_code,
                    target_fleet_number,
                    enemy_initial,
                    retreat_target_coords,
                    losses_sustained: losses,
                    enemy_losses_inflicted,
                    reason: withdrawal.reason,
                    stardate_week: None,
                }
            };
            events.encounter_disposition_events.push(event);
        }
    }
    let fought_against = |empire: u8, other: &TaskForce| {
        hostile_pairs.contains(&(empire, other.empire))
            && (other.engaged_in_battle || other.any_left())
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
        let after = &final_states[&empire];
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
                    enemy_after.counts[idx] += final_states[&tf.empire].counts[idx];
                }
            }
        }
        let enemy_losses = ship_losses_from_states(&enemy_before, &enemy_after);
        let enemy_loaded_armies_initial = task_forces
            .iter()
            .filter(|tf| fought_against(empire, tf))
            .map(|tf| original_loaded_armies.get(&tf.empire).copied().unwrap_or(0))
            .sum();
        let enemy_empires = task_forces
            .iter()
            .filter(|tf| {
                fought_against(empire, tf)
                    && original_states
                        .get(&tf.empire)
                        .is_some_and(FleetCombatState::has_units)
            })
            .map(|tf| tf.empire)
            .collect();
        let fleet_indices = after_tf.fleet_indices();
        let reporting_fleet_idx = preferred_reporting_fleet_index(game_data, &fleet_indices);
        let reporting_fleet_number = reporting_fleet_idx
            .map(|idx| game_data.fleets[idx].fleet_number.0 as u8)
            .filter(|fleet_number| *fleet_number != 0);
        let reporting_mission = reporting_fleet_idx
            .and_then(|idx| mission_kind_for_order(pre_encounter_orders.get(&idx).copied()));
        let primary_enemy = || {
            task_forces
                .iter()
                .filter(|tf| fought_against(empire, tf) && final_states[&tf.empire].has_units())
                .max_by_key(|tf| final_states[&tf.empire].total_combat_as())
        };
        let primary_enemy_fleet_number = primary_enemy()
            .and_then(|tf| single_named_fleet_number(game_data, &tf.fleet_indices()));
        events.fleet_battle_events.push(FleetBattleEvent {
            reporting_empire: empire,
            reporting_fleet_number,
            reporting_mission,
            perspective: if empire == attacker.empire {
                FleetBattlePerspective::Intercepted
            } else {
                FleetBattlePerspective::Attacked
            },
            coords,
            enemy_empires,
            primary_enemy_fleet_number,
            held_field: dominant_empire == Some(empire),
            friendly_initial: ship_counts_from_state(before),
            friendly_initial_starbases: before.counts[IDX_SB],
            friendly_loaded_armies_initial: original_loaded_armies
                .get(&empire)
                .copied()
                .unwrap_or(0),
            friendly_losses,
            friendly_starbases_lost: before.counts[IDX_SB].saturating_sub(after.counts[IDX_SB]),
            enemy_initial: ship_counts_from_state(&enemy_before),
            enemy_initial_starbases: enemy_before.counts[IDX_SB],
            enemy_loaded_armies_initial,
            enemy_losses,
            enemy_starbases_destroyed: enemy_before.counts[IDX_SB]
                .saturating_sub(enemy_after.counts[IDX_SB]),
            surprise,
            stardate_week: None,
        });

        let primary_enemy_empire = || {
            task_forces
                .iter()
                .filter(|tf| fought_against(empire, tf))
                .max_by_key(|tf| final_states[&tf.empire].total_combat_as())
                .map(|tf| tf.empire)
        };
        let primary_enemy_fleet_number = || {
            task_forces
                .iter()
                .filter(|tf| fought_against(empire, tf))
                .max_by_key(|tf| final_states[&tf.empire].total_combat_as())
                .and_then(|tf| single_named_fleet_number(game_data, &tf.fleet_indices()))
        };
        if !tf_has_any_units(after_tf) && !fleet_indices.is_empty() {
            let fleet_number = reporting_fleet_number.unwrap_or(0);
            events.fleet_destroyed_events.push(FleetDestroyedEvent {
                reporting_empire: empire,
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
                primary_enemy_empire: primary_enemy_empire(),
                primary_enemy_fleet_number: primary_enemy_fleet_number(),
                stardate_week: None,
            });
        }

        let destroyed_starbases = before.counts[IDX_SB].saturating_sub(after.counts[IDX_SB]);
        if destroyed_starbases > 0
            && let Some(lost_ids) = destroyed_starbases_by_empire.get(&empire)
        {
            for &starbase_id in lost_ids {
                events
                    .starbase_destroyed_events
                    .push(StarbaseDestroyedEvent {
                        reporting_empire: empire,
                        starbase_id,
                        coords,
                        enemy_initial: ship_counts_from_state(&enemy_before),
                        enemy_loaded_armies_initial,
                        enemy_losses,
                        primary_enemy_empire: primary_enemy_empire(),
                        primary_enemy_fleet_number: primary_enemy_fleet_number(),
                        stardate_week: None,
                    });
            }
        }
    }

    FleetBattle {
        events,
        held_by: dominant_empire,
    }
}
