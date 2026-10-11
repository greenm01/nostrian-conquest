//! One unit's turn in a tick: step, encounter check (`18ea:13cf`), order
//! dispatch, and the strike a hostile order makes when its test passes.

use nc_data::{
    Coords, DiplomacyOverride, DiplomaticRelation, EncounterDispositionReason, GameRng, Mission,
    MissionAbortReason, Order, Real48, StarbaseAlert, StarbaseSightingEvent, has_any_force,
    troop_transports,
};

use super::rules::{
    BASE_VISIBILITY, FleetFacts, Mode, Situation, Verdict, base_factor, decide_vs_base,
    decide_vs_fleet, fleet_posture, fleet_visibility, order, roe_permits, roll, sighting_percent,
    strength,
};
use super::{Awareness, Unit, UnitKind, layer_of};
use crate::CoreGameData;
use crate::maint::combat::{
    AssaultEvents, BattleSide, ContactObserver, FleetBattle, FleetBattlePhaseEvents,
    base_attack_strength, fight_encounter, mission_kind_for_order, nearest_friendly_world,
    push_fleet_contact, push_no_engagement, resolve_strike,
};
use crate::maint::movement::{BaseStep, TickEnd, YearMovement};
use crate::maint::{MissionEvent, MissionOutcome};

/// Set in a battle key's tick byte for the orbital gate's battle, which
/// may follow the same unit's encounter battle in the same tick.
const ORBITAL_GATE_BATTLE: u64 = 0x80;

pub(super) struct Year<'a> {
    pub units: Vec<Unit>,
    pub awareness: Awareness,
    pub rng: GameRng,
    pub campaign_seed: u64,
    pub diplomacy_overrides: &'a [DiplomacyOverride],
    pub movement: YearMovement<'a>,
    pub events: FleetBattlePhaseEvents,
    pub assault_events: AssaultEvents,
}

impl Year<'_> {
    pub(super) fn run_unit(
        &mut self,
        game_data: &mut CoreGameData,
        unit: usize,
        tick: u8,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if !self.units[unit].active {
            return Ok(());
        }
        let Unit { kind, index, .. } = self.units[unit];
        match kind {
            UnitKind::Fleet => {
                self.movement.step(game_data, index);
                self.follow_fleet(game_data, unit);
            }
            UnitKind::Base => {
                let step = self.movement.step_base(game_data, index, tick);
                self.follow_base_step(game_data, unit, &step);
            }
        }
        match kind {
            UnitKind::Fleet => self.fleet_scans(game_data, unit, tick),
            UnitKind::Base => self.base_scans(game_data, unit, tick),
        }
        match kind {
            UnitKind::Fleet if self.units[unit].active => {
                if self.movement.dispatch(game_data, index, tick)? == TickEnd::Strike {
                    self.strike(game_data, unit, tick);
                }
                if self.movement.removed(index) {
                    self.units[unit].active = false;
                }
            }
            UnitKind::Fleet => {}
            UnitKind::Base => self.movement.dispatch_base(game_data, index, tick),
        }
        Ok(())
    }

    /// Fleet unit `unit`'s strike test passed: once it holds the orbit, it
    /// attacks its target world now, as both stand this tick. A fleet the
    /// batteries destroy leaves play, and a captured world's new owner is
    /// what later ticks see.
    fn strike(&mut self, game_data: &mut CoreGameData, unit: usize, tick: u8) {
        if !self.holds_orbit(game_data, unit, tick) {
            return;
        }
        let fleet_idx = self.units[unit].index;
        let mut events = resolve_strike(game_data, self.campaign_seed, fleet_idx);
        events.stamp(tick);
        self.assault_events.append(events);
        self.movement.reload(fleet_idx);
        self.units[unit].active = has_any_force(&game_data.fleets[fleet_idx]);
    }

    /// The orbital gate, classic's starbase gate (`110a:67bd`) extended to
    /// the owner's fleets: before fleet unit `unit` assaults, everything the
    /// world's owner has in orbit fights it, sighted or not. That is his
    /// starbases there and his fleets there with warships, less any fleet
    /// already leaving on Seek Home (beaten earlier, or recalled). ROE
    /// decides as for a base blocking the path, weighing the attacker's
    /// attack strength against the defenders' (a starbase at 100), and a
    /// fleet unwilling to fight them aborts as it does before a base,
    /// without a shot: classic's gate runs the base decision (`1b05:1277`)
    /// itself. Only the holder of the field assaults: a beaten attacker has
    /// retreated with its battle, and a battle the round cap left
    /// unresolved gives no permission either.
    fn holds_orbit(&mut self, game_data: &mut CoreGameData, unit: usize, tick: u8) -> bool {
        let Unit {
            index: fleet_idx,
            owner,
            sector,
            ..
        } = self.units[unit];
        let Some(world_owner) = world_owner_at(game_data, sector)
            .filter(|world_owner| ![0, owner].contains(world_owner))
        else {
            return true;
        };
        let defenders: Vec<usize> = (0..self.units.len())
            .filter(|&other| {
                let u = self.units[other];
                u.active
                    && u.owner == world_owner
                    && u.sector == sector
                    && u.layer == 1
                    && match u.kind {
                        UnitKind::Base => true,
                        UnitKind::Fleet => {
                            let fleet = &game_data.fleets[u.index];
                            (fleet.ships.destroyers > 0
                                || fleet.ships.cruisers > 0
                                || fleet.ships.battleships > 0)
                                && !(fleet.order == Order::SeekHome && fleet.speed > 0)
                        }
                    }
            })
            .collect();
        if defenders.is_empty() {
            return true;
        }
        let order = game_data.fleets[fleet_idx].order;
        let facts = self.fleet_facts(game_data, fleet_idx);
        let defence = defenders
            .iter()
            .map(|&defender| match self.units[defender].kind {
                UnitKind::Base => {
                    base_attack_strength(&game_data.bases[self.units[defender].index])
                }
                UnitKind::Fleet => {
                    strength(&self.fleet_facts(game_data, self.units[defender].index))
                }
            })
            .sum::<u32>();
        if !roe_permits(facts.roe, strength(&facts), defence) {
            self.abort_mission(game_data, fleet_idx, AbortFallback::Hold, Some(tick));
            self.movement.reload(fleet_idx);
            return false;
        }
        let attacker = BattleSide {
            empire: owner,
            fleets: vec![fleet_idx],
            bases: Vec::new(),
        };
        let mut defender = BattleSide {
            empire: world_owner,
            ..BattleSide::default()
        };
        self.awareness.raise(world_owner, unit, 3);
        for &defending in &defenders {
            self.awareness.raise(owner, defending, 3);
            let u = self.units[defending];
            match u.kind {
                UnitKind::Fleet => defender.fleets.push(u.index),
                UnitKind::Base => defender.bases.push(u.index),
            }
        }
        let battle_key = ORBITAL_GATE_BATTLE | u64::from(tick) | (unit as u64) << 8;
        let held_by = self.fight(game_data, tick, battle_key, sector, &attacker, &defender);
        let fleet = &game_data.fleets[fleet_idx];
        held_by == Some(owner) && has_any_force(fleet) && fleet.order == order
    }

    /// Move fleet unit `unit`'s table entry to where its record now is; a
    /// unit that changes sector or layer is forgotten by every empire, and
    /// its escape from a battle there, if it was making one, is over.
    fn follow_fleet(&mut self, game_data: &mut CoreGameData, unit: usize) {
        let fleet = &mut game_data.fleets[self.units[unit].index];
        let (sector, layer) = (fleet.sector.to_array(), layer_of(fleet.depth));
        let entry = &mut self.units[unit];
        if (entry.sector, entry.layer) != (sector, layer) {
            entry.sector = sector;
            entry.layer = layer;
            fleet.escape = None;
            self.awareness.forget(unit);
        }
    }

    /// The table after a starbase's step: the base and the escort it
    /// carried move, both are forgotten when the base changed sector or
    /// layer, and guards merged into the escort leave play.
    fn follow_base_step(&mut self, game_data: &mut CoreGameData, unit: usize, step: &BaseStep) {
        let base = &game_data.bases[self.units[unit].index];
        let entry = &mut self.units[unit];
        entry.sector = base.sector.to_array();
        entry.layer = layer_of(base.depth);
        if step.moved_cell {
            self.awareness.forget(unit);
        }
        if let Some(escort) = step.escort.and_then(|fleet_idx| self.fleet_unit(fleet_idx)) {
            let fleet = &mut game_data.fleets[self.units[escort].index];
            let entry = &mut self.units[escort];
            entry.sector = fleet.sector.to_array();
            entry.layer = layer_of(fleet.depth);
            if step.moved_cell {
                fleet.escape = None;
                self.awareness.forget(escort);
            }
        }
        for &fleet_idx in &step.merged {
            if let Some(merged) = self.fleet_unit(fleet_idx) {
                self.units[merged].active = false;
            }
        }
    }

    fn fleet_unit(&self, fleet_idx: usize) -> Option<usize> {
        self.units
            .iter()
            .position(|unit| unit.kind == UnitKind::Fleet && unit.index == fleet_idx)
    }

    /// Units `observer` can meet: another empire's, in play, in the same
    /// sector and depth layer.
    fn meets(&self, observer: usize, other: usize) -> bool {
        let (o, t) = (&self.units[observer], &self.units[other]);
        t.active && t.owner != o.owner && t.sector == o.sector && t.layer == o.layer
    }

    fn fleet_facts(&self, game_data: &CoreGameData, fleet_idx: usize) -> FleetFacts {
        let fleet = &game_data.fleets[fleet_idx];
        let (order, target_flag) = self.movement.effective_order(game_data, fleet_idx);
        FleetFacts {
            owner: fleet.owner.0,
            order: order.code(),
            sector: fleet.sector.to_array(),
            layer: layer_of(fleet.depth),
            target: fleet.target.to_array(),
            target_layer: target_flag.min(1),
            speed: fleet.speed,
            max_speed: fleet.max_speed,
            roe: fleet.rules_of_engagement,
            battleships: fleet.ships.battleships,
            cruisers: fleet.ships.cruisers,
            destroyers: fleet.ships.destroyers,
            transports: troop_transports(fleet),
            etacs: fleet.ships.etacs,
            scouts: fleet.ships.scouts,
            crippled: fleet.crippled,
            escaping: fleet.escape.is_some(),
        }
    }

    fn declared_enemy(&self, game_data: &CoreGameData, from: u8, to: u8) -> bool {
        let relation = self
            .diplomacy_overrides
            .iter()
            .find(|directive| directive.from_empire == from && directive.to_empire == to)
            .map(|directive| directive.relation)
            .or_else(|| game_data.stored_diplomatic_relation(from, to));
        relation == Some(DiplomaticRelation::Enemy)
    }

    /// A starbase scans for alien units. It only reports: the identification
    /// is shared by its empire's fleets, and it never starts a fight. An
    /// alien starbase is always reported as newly located; classic opens
    /// with "previously reported starbase sighting BS-n" after a fleet's
    /// contact, which Rust does not report.
    fn base_scans(&mut self, game_data: &mut CoreGameData, observer: usize, tick: u8) {
        let Unit {
            index: base_idx,
            owner,
            layer,
            sector,
            ..
        } = self.units[observer];
        // A base under way does not scan.
        let base = &game_data.bases[base_idx];
        if base.sector != base.target
            || base.depth != Real48::from_int(i32::from(base.target_depth))
        {
            return;
        }
        let base_id = base.base_id.0;
        let factor = base_factor(layer);
        for target in 0..self.units.len() {
            if !self.meets(observer, target) {
                continue;
            }
            let aware = self.awareness.get(owner, target);
            if aware >= 2 {
                continue;
            }
            let unit = self.units[target];
            let visibility = match unit.kind {
                UnitKind::Fleet => fleet_visibility(&self.fleet_facts(game_data, unit.index)),
                UnitKind::Base => BASE_VISIBILITY,
            };
            if !roll(&mut self.rng, sighting_percent(aware, visibility, factor)) {
                continue;
            }
            self.awareness.raise(owner, target, 2);
            match unit.kind {
                UnitKind::Fleet => {
                    let alert =
                        self.base_alert(game_data, owner, &self.fleet_facts(game_data, unit.index));
                    push_fleet_contact(
                        &mut self.events.scout_contact_events,
                        &mut self.events.planet_intel_events,
                        game_data,
                        ContactObserver::Base(base_idx, alert),
                        unit.index,
                        sector,
                        tick,
                    );
                }
                UnitKind::Base => {
                    let target_base = &game_data.bases[unit.index];
                    let alerting = self.declared_enemy(game_data, owner, unit.owner);
                    self.events
                        .starbase_sighting_events
                        .push(StarbaseSightingEvent {
                            viewer_empire: owner,
                            starbase_id: base_id,
                            coords: sector,
                            in_system: layer == 1,
                            target_empire: unit.owner,
                            target_starbase_id: target_base.base_id.0,
                            alerting,
                            stardate_week: Some(tick),
                        });
                }
            }
        }
    }

    /// What a base of `owner` says about identified fleet `target`: an
    /// assault bound for the owner's world here, or any fleet inside its
    /// system, is "headed at our planet"; a declared enemy raises the
    /// alert; anything else is noted in passing.
    fn base_alert(
        &self,
        game_data: &CoreGameData,
        owner: u8,
        target: &FleetFacts,
    ) -> StarbaseAlert {
        let assault_here = matches!(target.order, order::BOMBARD | order::INVADE | order::BLITZ)
            && target.target_layer == 1
            && target.sector == target.target;
        if (assault_here || target.layer == 1)
            && owner != 0
            && world_owner_at(game_data, target.sector) == Some(owner)
        {
            StarbaseAlert::HeadedAtOurWorld
        } else if self.declared_enemy(game_data, owner, target.owner) {
            StarbaseAlert::Enemy
        } else {
            StarbaseAlert::Scanning
        }
    }

    /// A fleet scans the alien units it meets and may attack one of them.
    fn fleet_scans(&mut self, game_data: &mut CoreGameData, observer: usize, tick: u8) {
        let Unit {
            index: fleet_idx,
            owner,
            ..
        } = self.units[observer];
        let mut facts = self.fleet_facts(game_data, fleet_idx);
        let posture = fleet_posture(&facts);
        let mut closing = posture.closing;
        let scouting = facts.scouting();
        for target in 0..self.units.len() {
            if !self.units[observer].active || !self.meets(observer, target) {
                continue;
            }
            let aware = self.awareness.get(owner, target);
            let unit = self.units[target];
            let attack = match unit.kind {
                UnitKind::Fleet => {
                    if !closing && aware >= 3 {
                        continue;
                    }
                    let target_facts = self.fleet_facts(game_data, unit.index);
                    let chance =
                        sighting_percent(aware, fleet_visibility(&target_facts), posture.factor);
                    if !roll(&mut self.rng, chance) {
                        continue;
                    }
                    let mut level = aware.max(1);
                    let mut identify = false;
                    if aware == 0 {
                        push_fleet_contact(
                            &mut self.events.scout_contact_events,
                            &mut self.events.planet_intel_events,
                            game_data,
                            ContactObserver::Fleet(fleet_idx),
                            unit.index,
                            facts.sector,
                            tick,
                        );
                    }
                    let mode = if aware <= 1 && (scouting || closing) {
                        identify = true;
                        level = 2;
                        Some(Mode::Identify)
                    } else if aware >= 2 && !scouting {
                        Some(Mode::Recheck)
                    } else {
                        None
                    };
                    if scouting && aware == 0 {
                        // Classic stops closing on further contacts for the
                        // rest of a scout's scan this tick.
                        closing = false;
                    }
                    self.awareness.raise(owner, target, level);
                    // A fleet escaping from a battle here that this empire
                    // has already caught once has got away from it.
                    if game_data.fleets[unit.index]
                        .escape
                        .is_some_and(|escape| escape.pursued_by(owner))
                    {
                        continue;
                    }
                    let Some(mode) = mode else {
                        continue;
                    };
                    let verdict =
                        self.decide_vs_fleet(game_data, observer, &facts, &target_facts, mode);
                    let reason = match verdict {
                        Verdict::Avoided => Some(EncounterDispositionReason::RoeDeclined),
                        Verdict::Escaped => Some(EncounterDispositionReason::InterceptFailed),
                        _ => None,
                    };
                    if let (true, Some(reason)) = (identify, reason) {
                        push_no_engagement(
                            &mut self.events.encounter_disposition_events,
                            game_data,
                            fleet_idx,
                            unit.index,
                            facts.sector,
                            tick,
                            reason,
                        );
                    }
                    if verdict == Verdict::Aborted {
                        // A re-check's report is discarded, as in classic.
                        let report = identify.then_some(tick);
                        self.abort_mission(game_data, fleet_idx, AbortFallback::ClimbOut, report);
                        facts = self.fleet_facts(game_data, fleet_idx);
                    }
                    verdict.attacks()
                }
                UnitKind::Base => {
                    if !closing && aware != 0 {
                        continue;
                    }
                    let chance = sighting_percent(aware, BASE_VISIBILITY, posture.factor);
                    if !roll(&mut self.rng, chance) {
                        continue;
                    }
                    let decides = scouting || aware >= 2 || closing;
                    let level = if decides { aware.max(2) } else { aware.max(1) };
                    self.awareness.raise(owner, target, level);
                    if !decides {
                        continue;
                    }
                    let base = &game_data.bases[unit.index];
                    let guard = self.guard_fleet_of(game_data, target);
                    let base_strength = base_attack_strength(base);
                    let defense = base_strength.wrapping_add(guard.map_or(0, |g| {
                        strength(&self.fleet_facts(game_data, self.units[g].index))
                    }));
                    let declared = self.declared_enemy(game_data, owner, base.owner.0);
                    let verdict = decide_vs_base(&facts, defense, base_strength, declared);
                    if verdict == Verdict::Aborted {
                        let report = (aware <= 1).then_some(tick);
                        self.abort_mission(game_data, fleet_idx, AbortFallback::Hold, report);
                        facts = self.fleet_facts(game_data, fleet_idx);
                    }
                    if verdict.attacks()
                        && let Some(guard) = guard
                    {
                        self.awareness.raise(owner, guard, 3);
                    }
                    verdict.attacks()
                }
            };
            if attack {
                self.battle(game_data, observer, target, tick);
                break;
            }
        }
    }

    fn decide_vs_fleet(
        &mut self,
        game_data: &CoreGameData,
        observer: usize,
        facts: &FleetFacts,
        target_facts: &FleetFacts,
        mode: Mode,
    ) -> Verdict {
        let world_owner = (facts.layer == 1)
            .then(|| world_owner_at(game_data, facts.sector))
            .flatten();
        let situation = Situation {
            world_owner,
            declared_enemy: self.declared_enemy(game_data, facts.owner, target_facts.owner),
            target_awareness: self.awareness.get(target_facts.owner, observer),
        };
        decide_vs_fleet(&mut self.rng, facts, target_facts, situation, mode)
    }

    /// The first active fleet guarding base unit `base` (`1b05:21de`).
    fn guard_fleet_of(&self, game_data: &CoreGameData, base: usize) -> Option<usize> {
        let b = self.units[base];
        let base_id = game_data.bases[b.index].base_id.0;
        (0..self.units.len()).find(|&unit| {
            let u = self.units[unit];
            u.kind == UnitKind::Fleet
                && u.active
                && u.owner == b.owner
                && u.sector == b.sector
                && u.layer == b.layer
                && {
                    let fleet = &game_data.fleets[u.index];
                    fleet.order == Order::GuardStarbase && fleet.mission_target == base_id
                }
        })
    }

    /// The base a guard fleet at rest escorts, when it is here (`1b05:2133`).
    fn guarded_base_of(&self, game_data: &CoreGameData, fleet_unit: usize) -> Option<usize> {
        let f = self.units[fleet_unit];
        let fleet = &game_data.fleets[f.index];
        if fleet.order != Order::GuardStarbase || fleet.speed != 0 {
            return None;
        }
        (0..self.units.len()).find(|&unit| {
            let u = self.units[unit];
            u.kind == UnitKind::Base
                && u.active
                && u.owner == f.owner
                && u.sector == f.sector
                && u.layer == f.layer
                && game_data.bases[u.index].base_id.0 == fleet.mission_target
        })
    }

    /// Observer `observer` attacks unit `target`: exactly two sides fight,
    /// the observer alone against the target and its guard partner.
    fn battle(&mut self, game_data: &mut CoreGameData, observer: usize, target: usize, tick: u8) {
        let o = self.units[observer];
        let t = self.units[target];
        let partner = match t.kind {
            UnitKind::Fleet => self.guarded_base_of(game_data, target),
            UnitKind::Base => self.guard_fleet_of(game_data, target),
        };
        self.awareness.raise(t.owner, observer, 3);
        self.awareness.raise(o.owner, target, 3);
        if let Some(partner) = partner {
            self.awareness.raise(o.owner, partner, 3);
        }
        // The attacker loses this tick's movement: classic's battle reads
        // its record from before the step.
        self.movement.undo_step(game_data, o.index);
        let attacker = BattleSide {
            empire: o.owner,
            fleets: vec![o.index],
            bases: Vec::new(),
        };
        let mut defender = BattleSide {
            empire: t.owner,
            ..BattleSide::default()
        };
        for unit in std::iter::once(target).chain(partner) {
            let u = self.units[unit];
            match u.kind {
                UnitKind::Fleet => defender.fleets.push(u.index),
                UnitKind::Base => defender.bases.push(u.index),
            }
        }
        let battle_key = u64::from(tick) | (observer as u64) << 8;
        self.fight(game_data, tick, battle_key, o.sector, &attacker, &defender);
    }

    /// Fight `attacker` against `defender` at `sector` and put the result on
    /// the books: the events dated to the tick, the fighters' records
    /// reloaded, and destroyed units out of play. Returns who held the
    /// field.
    fn fight(
        &mut self,
        game_data: &mut CoreGameData,
        tick: u8,
        battle_key: u64,
        sector: [u8; 2],
        attacker: &BattleSide,
        defender: &BattleSide,
    ) -> Option<u8> {
        let FleetBattle {
            mut events,
            held_by,
        } = fight_encounter(
            game_data,
            self.campaign_seed,
            battle_key,
            sector,
            attacker,
            defender,
        );
        stamp_battle_events(&mut events, tick);
        self.events.append(events);
        for &fleet_idx in attacker.fleets.iter().chain(&defender.fleets) {
            self.movement.reload(fleet_idx);
        }
        for unit in &mut self.units {
            unit.active = match unit.kind {
                UnitKind::Fleet => unit.active && has_any_force(&game_data.fleets[unit.index]),
                UnitKind::Base => game_data.bases[unit.index].base_id.0 != 0,
            };
        }
        held_by
    }

    /// ROE forbids fighting a force that blocks the mission: the fleet heads
    /// for the nearest friendly world (`24a3:0876`), or takes `fallback`
    /// when its empire owns no other world. `report` is the tick to report
    /// it in, if the decision is reported.
    fn abort_mission(
        &mut self,
        game_data: &mut CoreGameData,
        fleet_idx: usize,
        fallback: AbortFallback,
        report: Option<u8>,
    ) {
        let fleet = &game_data.fleets[fleet_idx];
        let coords = fleet.sector.to_array();
        let order = fleet.order;
        let kind = match order {
            Order::ColonizeWorld => Some(Mission::ColonizeWorld),
            other => mission_kind_for_order(Some(other)),
        };
        let home = nearest_friendly_world(game_data, fleet_idx);
        let fleet = &mut game_data.fleets[fleet_idx];
        match (home, fallback) {
            (Some(home), _) => {
                fleet.order = Order::SeekHome;
                fleet.target = Coords::from_array(home);
                fleet.target_depth = 1;
                fleet.speed = fleet.max_speed;
            }
            (None, AbortFallback::ClimbOut) => {
                fleet.order = Order::MoveOnly;
                fleet.target = Coords::from_array(coords);
                fleet.target_depth = 0;
                fleet.speed = fleet.max_speed;
            }
            (None, AbortFallback::Hold) => {
                fleet.order = Order::HoldPosition;
                fleet.speed = 0;
            }
        }
        if let Some(tick) = report {
            self.push_orbit_blocked_abort(game_data, fleet_idx, kind, tick);
        }
    }

    /// Report, in tick `tick`, that fleet `fleet_idx` aborted its mission
    /// before the force in its way. Nothing is reported for an order that
    /// is not a mission.
    fn push_orbit_blocked_abort(
        &mut self,
        game_data: &CoreGameData,
        fleet_idx: usize,
        kind: Option<Mission>,
        tick: u8,
    ) {
        let Some(kind) = kind else {
            return;
        };
        let fleet = &game_data.fleets[fleet_idx];
        let coords = fleet.sector.to_array();
        self.events.mission_events.push(MissionEvent {
            fleet_idx,
            owner_empire: fleet.owner.0,
            kind,
            outcome: MissionOutcome::Aborted,
            abort_reason: Some(MissionAbortReason::OrbitBlocked),
            planet_idx: game_data
                .planets
                .iter()
                .position(|planet| planet.sector.to_array() == coords),
            location_coords: Some(coords),
            target_coords: Some(fleet.target.to_array()),
            stardate_week: Some(tick),
        });
    }
}

/// The owner of the world in `sector` (0 unowned), or `None` where there is
/// no world.
fn world_owner_at(game_data: &CoreGameData, sector: [u8; 2]) -> Option<u8> {
    game_data
        .planets
        .iter()
        .find(|planet| planet.sector.to_array() == sector)
        .map(|planet| planet.owner.0)
}

/// Where an aborting fleet goes with no friendly world to seek.
#[derive(Clone, Copy)]
enum AbortFallback {
    /// Move out of the system to deep space (blocked by a fleet).
    ClimbOut,
    /// Stop where it is (blocked by a starbase, or at the orbital gate).
    Hold,
}

fn stamp_battle_events(events: &mut FleetBattlePhaseEvents, tick: u8) {
    for event in &mut events.fleet_battle_events {
        event.stardate_week = Some(tick);
    }
    for event in &mut events.fleet_destroyed_events {
        event.stardate_week = Some(tick);
    }
    for event in &mut events.starbase_destroyed_events {
        event.stardate_week = Some(tick);
    }
    for event in &mut events.mission_events {
        event.stardate_week = Some(tick);
    }
    for event in &mut events.planet_intel_events {
        event.stardate_week = Some(tick);
    }
    for event in &mut events.encounter_disposition_events {
        match event {
            nc_data::EncounterDispositionEvent::NoEngagement { stardate_week, .. }
            | nc_data::EncounterDispositionEvent::Retreated { stardate_week, .. }
            | nc_data::EncounterDispositionEvent::PursuitFire { stardate_week, .. } => {
                *stardate_week = Some(tick)
            }
        }
    }
}
