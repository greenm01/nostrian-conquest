//! One unit's turn in a tick: step, encounter check (`18ea:13cf`), order
//! dispatch, and the strike a hostile order makes when its test passes.

use nc_data::{
    Coords, DiplomacyOverride, DiplomaticRelation, EncounterDispositionReason, GameRng, Mission,
    MissionAbortReason, Order, Real48, StarbaseAlert, StarbaseSightingEvent, has_any_force,
    troop_transports,
};

use super::rules::{
    BASE_VISIBILITY, FleetFacts, Situation, Verdict, admitted, base_factor, decide_vs_base,
    decide_vs_fleet, fleet_posture, fleet_visibility, order, roll, sighting_percent, strength,
};
use super::{Awareness, Presence, Unit, UnitKind, layer_of};
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

/// The units one empire brings to a battle, as unit table indices.
#[derive(Debug, Default)]
struct Roster {
    fleets: Vec<usize>,
    bases: Vec<usize>,
}

pub(super) struct Year<'a> {
    pub units: Vec<Unit>,
    pub awareness: Awareness,
    pub rng: GameRng,
    pub campaign_seed: u64,
    pub diplomacy_overrides: &'a [DiplomacyOverride],
    pub movement: YearMovement<'a>,
    pub events: FleetBattlePhaseEvents,
    pub assault_events: AssaultEvents,
    /// Where units stopped being since the last settling: their empire, and
    /// the sector and layer they were in.
    pub departed: Vec<Presence>,
    /// Fleets that have bombarded this year, alone or with others (t202).
    pub bombarded: Vec<usize>,
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
                    self.departed.push(Presence::of(&self.units[unit]));
                }
            }
            UnitKind::Fleet => {}
            UnitKind::Base => self.movement.dispatch_base(game_data, index, tick),
        }
        self.settle_sightings(game_data);
        Ok(())
    }

    /// Forget what an empire remembers where its units have left or died,
    /// unless another of its units is still there: a viewer's sighting
    /// memory lasts as long as it has a unit in the sector and layer.
    fn settle_sightings(&mut self, game_data: &mut CoreGameData) {
        for presence in std::mem::take(&mut self.departed) {
            let Presence {
                empire,
                sector,
                layer,
            } = presence;
            let sector = Coords::from_array(sector);
            if game_data.sightings.holds(empire, sector, layer) && !self.occupied(presence) {
                game_data.sightings.forget(empire, sector, layer);
            }
        }
    }

    /// Whether the empire has a unit in play at `presence`.
    fn occupied(&self, presence: Presence) -> bool {
        self.units
            .iter()
            .any(|u| u.active && Presence::of(u) == presence)
    }

    /// Fleet unit `unit`'s strike test passed: once it holds the orbit, it
    /// attacks its target world now, as both stand this tick. A fleet the
    /// batteries destroy leaves play, and a captured world's new owner is
    /// what later ticks see.
    fn strike(&mut self, game_data: &mut CoreGameData, unit: usize, tick: u8) {
        let fleet_idx = self.units[unit].index;
        if self.bombarded.contains(&fleet_idx) {
            return;
        }
        if !self.holds_orbit(game_data, unit, tick) {
            return;
        }
        let Unit {
            owner,
            sector,
            layer,
            ..
        } = self.units[unit];
        if let Some(world_owner) = world_owner_at(game_data, sector)
            .filter(|world_owner| ![0, owner].contains(world_owner))
        {
            self.note_combat(game_data, sector, layer, owner, world_owner);
        }
        // An empire's fleets bombarding the same world strike together.
        let companions: Vec<usize> = if game_data.fleets[fleet_idx].order == Order::BombardWorld {
            let target = game_data.fleets[fleet_idx].target;
            (0..self.units.len())
                .filter(|&other| {
                    let u = &self.units[other];
                    // A base unit's index is a starbase record, not a fleet.
                    other != unit
                        && u.active
                        && u.kind == UnitKind::Fleet
                        && u.owner == owner
                        && !self.bombarded.contains(&u.index)
                        && game_data.fleets.get(u.index).is_some_and(|fleet| {
                            fleet.order == Order::BombardWorld
                                && fleet.target == target
                                && fleet.sector == target
                        })
                })
                .collect()
        } else {
            Vec::new()
        };
        let companion_fleets: Vec<usize> = companions
            .iter()
            .map(|&other| self.units[other].index)
            .collect();
        let bombarding = game_data.fleets[fleet_idx].order == Order::BombardWorld;
        let mut events =
            resolve_strike(game_data, self.campaign_seed, fleet_idx, &companion_fleets);
        events.stamp(tick);
        self.assault_events.append(events);
        if bombarding {
            self.bombarded.push(fleet_idx);
            self.bombarded.extend(&companion_fleets);
        }
        for member in std::iter::once(unit).chain(companions) {
            let index = self.units[member].index;
            self.movement.reload(index);
            self.units[member].active = has_any_force(&game_data.fleets[index]);
            if !self.units[member].active {
                self.departed.push(Presence::of(&self.units[member]));
            }
        }
    }

    /// The orbital gate, classic's starbase gate (`110a:67bd`) extended to
    /// the owner's fleets: before fleet unit `unit` assaults, everything the
    /// world's owner has in orbit fights it, sighted or not. That is his
    /// starbases there and his fleets there with warships, less any fleet
    /// already leaving on Seek Home (beaten earlier, or recalled). The
    /// attacker's empire's other fleets in orbit volunteer under their own
    /// ROE, and ROE decides for the attacker as for a base blocking the
    /// path, weighing its side's assembled attack strength against the
    /// defenders' (a starbase at 100); a fleet unwilling to fight them
    /// aborts as it does before a base, without a shot: classic's gate runs
    /// the base decision (`1b05:1277`) itself. Only the holder of the field
    /// assaults: a beaten attacker has retreated with its battle, and a
    /// battle the round cap left unresolved gives no permission either.
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
        let mut defenders = Roster::default();
        for other in 0..self.units.len() {
            let u = self.units[other];
            if !(u.active && u.owner == world_owner && u.sector == sector && u.layer == 1) {
                continue;
            }
            match u.kind {
                UnitKind::Base => defenders.bases.push(other),
                UnitKind::Fleet => {
                    let fleet = &game_data.fleets[u.index];
                    if (fleet.ships.destroyers > 0
                        || fleet.ships.cruisers > 0
                        || fleet.ships.battleships > 0)
                        && !(fleet.order == Order::SeekHome && fleet.speed > 0)
                    {
                        defenders.fleets.push(other);
                    }
                }
            }
        }
        if defenders.fleets.is_empty() && defenders.bases.is_empty() {
            return true;
        }
        let order = game_data.fleets[fleet_idx].order;
        let defence = self.roster_strength(game_data, &defenders);
        let volunteers = self.volunteers(game_data, unit, defence);
        if !volunteers.contains(&unit) {
            self.abort_mission(game_data, fleet_idx, AbortFallback::Hold, Some(tick));
            self.movement.reload(fleet_idx);
            return false;
        }
        let battle_key = ORBITAL_GATE_BATTLE | u64::from(tick) | (unit as u64) << 8;
        let held_by = self.battle(
            game_data,
            unit,
            battle_key,
            &volunteers,
            world_owner,
            &defenders,
            tick,
        );
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
            self.departed.push(Presence::of(entry));
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
        let (sector, layer) = (base.sector.to_array(), layer_of(base.depth));
        if (entry.sector, entry.layer) != (sector, layer) {
            self.departed.push(Presence::of(entry));
        }
        entry.sector = sector;
        entry.layer = layer;
        if step.moved_cell {
            self.awareness.forget(unit);
        }
        if let Some(escort) = step.escort.and_then(|fleet_idx| self.fleet_unit(fleet_idx)) {
            let fleet = &mut game_data.fleets[self.units[escort].index];
            let entry = &mut self.units[escort];
            let (sector, layer) = (fleet.sector.to_array(), layer_of(fleet.depth));
            if (entry.sector, entry.layer) != (sector, layer) {
                self.departed.push(Presence::of(entry));
            }
            entry.sector = sector;
            entry.layer = layer;
            if step.moved_cell {
                fleet.escape = None;
                self.awareness.forget(escort);
            }
        }
        for &fleet_idx in &step.merged {
            if let Some(merged) = self.fleet_unit(fleet_idx) {
                self.departed.push(Presence::of(&self.units[merged]));
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

    /// Unit `observer` has sighted a unit of empire `opponent`: its empire
    /// remembers that here, for all its units in the sector and layer.
    fn note_sighting(&self, game_data: &mut CoreGameData, observer: usize, opponent: u8) {
        let u = self.units[observer];
        let sector = Coords::from_array(u.sector);
        game_data.sightings.note(u.owner, sector, u.layer, opponent);
    }

    /// Empires `a` and `b` fought in a sector and layer: each that has a
    /// unit there remembers the other.
    fn note_combat(&self, game_data: &mut CoreGameData, sector: [u8; 2], layer: u8, a: u8, b: u8) {
        for (viewer, opponent) in [(a, b), (b, a)] {
            let presence = Presence {
                empire: viewer,
                sector,
                layer,
            };
            if self.occupied(presence) {
                game_data
                    .sightings
                    .note(viewer, Coords::from_array(sector), layer, opponent);
            }
        }
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
        }
    }

    /// The units of `target`'s empire that would defend it: its empire's
    /// fleets in the sector and layer, less those escaping from a battle
    /// there that `attacker` has already caught once (they have got away
    /// from it), and the starbases the target rule brings in, defenders
    /// only: the target itself when it is a base, and the base each guard
    /// fleet at rest among them guards.
    fn defender_roster(&self, game_data: &CoreGameData, target: usize, attacker: u8) -> Roster {
        let t = self.units[target];
        let mut roster = Roster::default();
        for unit in 0..self.units.len() {
            let u = self.units[unit];
            if !(u.kind == UnitKind::Fleet
                && u.active
                && u.owner == t.owner
                && u.sector == t.sector
                && u.layer == t.layer)
            {
                continue;
            }
            let got_away = unit != target
                && game_data.fleets[u.index]
                    .escape
                    .is_some_and(|escape| escape.pursued_by(attacker));
            if !got_away {
                roster.fleets.push(unit);
            }
        }
        if t.kind == UnitKind::Base {
            roster.bases.push(target);
        }
        for &fleet in &roster.fleets {
            if let Some(base) = self.guarded_base_of(game_data, fleet)
                && !roster.bases.contains(&base)
            {
                roster.bases.push(base);
            }
        }
        roster
    }

    /// A roster's assembled strength: each fleet's warships at their attack
    /// strength and each starbase at its own.
    fn roster_strength(&self, game_data: &CoreGameData, roster: &Roster) -> u32 {
        let fleets = roster
            .fleets
            .iter()
            .map(|&unit| strength(&self.fleet_facts(game_data, self.units[unit].index)));
        let bases = roster
            .bases
            .iter()
            .map(|&unit| base_attack_strength(&game_data.bases[self.units[unit].index]));
        fleets.chain(bases).sum()
    }

    /// The fleets of `observer`'s empire in its sector and layer that
    /// volunteer to attack a side of strength `enemy`: each with ROE above 0
    /// and warships, admitted by its own ROE against the volunteers'
    /// assembled strength, recomputed until none refuses. The observer is
    /// among them only when it is willing itself.
    fn volunteers(&self, game_data: &CoreGameData, observer: usize, enemy: u32) -> Vec<usize> {
        let o = self.units[observer];
        let candidates: Vec<usize> = (0..self.units.len())
            .filter(|&unit| {
                let u = self.units[unit];
                u.kind == UnitKind::Fleet
                    && u.active
                    && u.owner == o.owner
                    && u.sector == o.sector
                    && u.layer == o.layer
            })
            .collect();
        let facts: Vec<(u8, u32)> = candidates
            .iter()
            .map(|&unit| {
                let fleet = self.fleet_facts(game_data, self.units[unit].index);
                (fleet.roe, strength(&fleet))
            })
            .collect();
        candidates
            .into_iter()
            .zip(admitted(&facts, enemy))
            .filter_map(|(unit, stays)| stays.then_some(unit))
            .collect()
    }

    /// The strength of the volunteers assembled.
    fn volunteers_strength(&self, game_data: &CoreGameData, volunteers: &[usize]) -> u32 {
        volunteers
            .iter()
            .map(|&unit| strength(&self.fleet_facts(game_data, self.units[unit].index)))
            .sum()
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
            self.note_sighting(game_data, observer, unit.owner);
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
                    self.note_sighting(game_data, observer, unit.owner);
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
                    let decides = if aware <= 1 && (scouting || closing) {
                        identify = true;
                        level = 2;
                        true
                    } else {
                        aware >= 2 && !scouting
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
                    if !decides {
                        continue;
                    }
                    let roster = self.defender_roster(game_data, target, owner);
                    let enemy = self.roster_strength(game_data, &roster);
                    let volunteers = self.volunteers(game_data, observer, enemy);
                    let own = self.volunteers_strength(game_data, &volunteers);
                    let verdict = self.decide_vs_fleet(
                        game_data,
                        observer,
                        &facts,
                        &target_facts,
                        own,
                        enemy,
                    );
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
                    verdict.attacks().then_some((volunteers, roster))
                }
                UnitKind::Base => {
                    if !closing && aware != 0 {
                        continue;
                    }
                    let chance = sighting_percent(aware, BASE_VISIBILITY, posture.factor);
                    if !roll(&mut self.rng, chance) {
                        continue;
                    }
                    self.note_sighting(game_data, observer, unit.owner);
                    let decides = scouting || aware >= 2 || closing;
                    let level = if decides { aware.max(2) } else { aware.max(1) };
                    self.awareness.raise(owner, target, level);
                    if !decides {
                        continue;
                    }
                    let roster = self.defender_roster(game_data, target, owner);
                    let enemy = self.roster_strength(game_data, &roster);
                    let volunteers = self.volunteers(game_data, observer, enemy);
                    let own = self.volunteers_strength(game_data, &volunteers);
                    let declared = self.declared_enemy(game_data, owner, unit.owner);
                    let verdict = decide_vs_base(&facts, own, enemy, declared);
                    if verdict == Verdict::Aborted {
                        let report = (aware <= 1).then_some(tick);
                        self.abort_mission(game_data, fleet_idx, AbortFallback::Hold, report);
                        facts = self.fleet_facts(game_data, fleet_idx);
                    }
                    verdict.attacks().then_some((volunteers, roster))
                }
            };
            if let Some((volunteers, roster)) = attack {
                // The attacker loses this tick's movement: classic's battle
                // reads its record from before the step.
                self.movement.undo_step(game_data, fleet_idx);
                let battle_key = u64::from(tick) | (observer as u64) << 8;
                self.battle(
                    game_data,
                    observer,
                    battle_key,
                    &volunteers,
                    unit.owner,
                    &roster,
                    tick,
                );
                break;
            }
        }
    }

    /// Observer `observer` decides about fleet `target_facts` with the
    /// sides as they would fight: `own` the strength of its empire's
    /// volunteers and `enemy` the target's roster.
    fn decide_vs_fleet(
        &mut self,
        game_data: &CoreGameData,
        observer: usize,
        facts: &FleetFacts,
        target_facts: &FleetFacts,
        own: u32,
        enemy: u32,
    ) -> Verdict {
        let world_owner = (facts.layer == 1)
            .then(|| world_owner_at(game_data, facts.sector))
            .flatten();
        let situation = Situation {
            world_owner,
            declared_enemy: self.declared_enemy(game_data, facts.owner, target_facts.owner),
            target_awareness: self.awareness.get(target_facts.owner, observer),
            own_strength: own,
            enemy_strength: enemy,
        };
        decide_vs_fleet(&mut self.rng, facts, target_facts, situation)
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

    /// Fleet unit `attacker`'s empire attacks `defender_empire`: the two
    /// sides are the attacker's `volunteers` and the defenders' `roster`,
    /// each side its empire's units in the sector and layer; nobody else
    /// joins. Every unit of each side becomes known (3, fought) to the
    /// other empire, and each empire remembers the other here. The attack
    /// is a surprise when the defending empire had not sighted the
    /// attacking one in this sector and layer, decided before the combat
    /// adds to anyone's memory. Returns who held the field.
    #[allow(clippy::too_many_arguments)]
    fn battle(
        &mut self,
        game_data: &mut CoreGameData,
        attacker: usize,
        battle_key: u64,
        volunteers: &[usize],
        defender_empire: u8,
        roster: &Roster,
        tick: u8,
    ) -> Option<u8> {
        let a = self.units[attacker];
        let sector = Coords::from_array(a.sector);
        let surprise = !game_data
            .sightings
            .knows(defender_empire, sector, a.layer, a.owner);
        self.note_combat(game_data, a.sector, a.layer, a.owner, defender_empire);
        for &unit in volunteers {
            self.awareness.raise(defender_empire, unit, 3);
        }
        for &unit in roster.fleets.iter().chain(&roster.bases) {
            self.awareness.raise(a.owner, unit, 3);
        }
        let record = |unit: &usize| self.units[*unit].index;
        let attacking = BattleSide {
            empire: a.owner,
            fleets: volunteers.iter().map(record).collect(),
            bases: Vec::new(),
        };
        let defending = BattleSide {
            empire: defender_empire,
            fleets: roster.fleets.iter().map(record).collect(),
            bases: roster.bases.iter().map(record).collect(),
        };
        self.fight(
            game_data, tick, battle_key, a.sector, &attacking, &defending, surprise,
        )
    }

    /// Fight `attacker` against `defender` at `sector`, the attacker's first
    /// exchange a surprise if `surprise`, and put the result on the books:
    /// the events dated to the tick, the fighters' records reloaded, and
    /// destroyed units out of play. Returns who held the field.
    #[allow(clippy::too_many_arguments)]
    fn fight(
        &mut self,
        game_data: &mut CoreGameData,
        tick: u8,
        battle_key: u64,
        sector: [u8; 2],
        attacker: &BattleSide,
        defender: &BattleSide,
        surprise: bool,
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
            surprise,
        );
        stamp_battle_events(&mut events, tick);
        self.events.append(events);
        for &fleet_idx in attacker.fleets.iter().chain(&defender.fleets) {
            self.movement.reload(fleet_idx);
        }
        for unit in &mut self.units {
            let active = match unit.kind {
                UnitKind::Fleet => unit.active && has_any_force(&game_data.fleets[unit.index]),
                UnitKind::Base => game_data.bases[unit.index].base_id.0 != 0,
            };
            if unit.active && !active {
                self.departed.push(Presence::of(unit));
            }
            unit.active = active;
        }
        held_by
    }

    /// ROE forbids fighting a force that blocks the mission: the fleet heads
    /// for the nearest friendly world (`24a3:0876`), or takes `fallback`
    /// when its empire owns no other world. `report` is the tick to report
    /// it in, if the decision is reported. A Guard Starbase fleet would let
    /// go of its base first, as in ECMAINT (`1b05:185e`, `1b05:1f5d`); the
    /// fleets that abort here are on mission orders and have none.
    fn abort_mission(
        &mut self,
        game_data: &mut CoreGameData,
        fleet_idx: usize,
        fallback: AbortFallback,
        report: Option<u8>,
    ) {
        game_data.release_starbase_escort(fleet_idx);
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
