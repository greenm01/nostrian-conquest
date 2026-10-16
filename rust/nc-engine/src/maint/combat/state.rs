use crate::{CoreGameData, EncounterDispositionReason, Order, ShipLosses};
use nc_data::{
    CrippledHulls, CrippledShips, Fleet, HullClass, ShipCounts, Starbase, hull_count,
    slowest_speed, troop_transports,
};

pub(super) const IDX_DD: usize = 0;
pub(super) const IDX_CA: usize = 1;
pub(super) const IDX_BB: usize = 2;
pub(super) const IDX_SB: usize = 3;
pub(super) const IDX_SC: usize = 4;
pub(super) const IDX_TT: usize = 5;
pub(super) const IDX_ET: usize = 6;

const AS_DD: u32 = 12;
const AS_CA: u32 = 33;
const AS_BB: u32 = 90;
const AS_SB: u32 = 100;
const AS_SC: u32 = 0;
const AS_TT: u32 = 0;
const AS_ET: u32 = 0;

/// A starbase's attack strength, which is also what it counts for in the
/// decision to fight.
pub(crate) const STARBASE_ATTACK_STRENGTH: u32 = AS_SB;

const DS_DD: u32 = 7;
const DS_CA: u32 = 30;
const DS_BB: u32 = 110;
const DS_SB: u32 = 130;
const DS_SC: u32 = 10;
const DS_TT: u32 = 10;
const DS_ET: u32 = 20;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum BattleRole {
    IncumbentDefender,
    GuardingDefender,
    Attacker,
    Neutral,
}

/// A side's hulls by class (the `IDX_` constants) and, among them, the
/// crippled ones by recovery state. The state is loaded from the records at
/// the start of a battle or strike and written back at its end, so damage
/// carries into every later battle.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct FleetCombatState {
    pub(super) counts: [u32; 7],
    pub(super) crippled: [CrippledHulls; 7],
}

/// Hits a side took in a fleet battle but has not yet spent on a damage
/// step, per fire pool.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct CarriedHits {
    pub(super) suppression: u32,
    pub(super) execution: u32,
}

/// How a fleet left the battle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Withdrawal {
    /// Its ROE failed, or it had no armed ship left.
    pub(super) reason: EncounterDispositionReason,
    /// The other side caught it in a pursuit exchange.
    pub(super) pursued: bool,
}

/// A fleet's transports of one cargo state, loaded or empty: the hulls and,
/// among them, the crippled cohorts. Each keeps its own counter on the
/// record, and fire strikes the loaded ones first.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct TransportGroup {
    pub(super) hulls: u32,
    pub(super) crippled: CrippledHulls,
}

/// A fleet's transports by cargo state. Together they are its `IDX_TT`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct Transports {
    pub(super) loaded: TransportGroup,
    pub(super) empty: TransportGroup,
}

impl Transports {
    /// The fleet's transports as its record has them.
    pub(super) fn from_record(fleet: &Fleet) -> Self {
        Self {
            loaded: TransportGroup {
                hulls: u32::from(fleet.ships.loaded_transports),
                crippled: fleet.crippled.loaded_transports,
            },
            empty: TransportGroup {
                hulls: u32::from(fleet.ships.empty_transports),
                crippled: fleet.crippled.empty_transports,
            },
        }
    }

    /// Both subgroups together: the hulls and crippled cohorts of the
    /// fleet's `IDX_TT`.
    pub(super) fn total(&self) -> (u32, CrippledHulls) {
        let mut crippled = self.loaded.crippled;
        crippled.add(&self.empty.crippled);
        (self.loaded.hulls + self.empty.hulls, crippled)
    }
}

/// The record a member of a side is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Record {
    Fleet(usize),
    Base(usize),
}

/// One fleet or starbase on a side, with its own hulls. The roster is fixed
/// when the battle starts; a fleet leaves it only by breaking off.
#[derive(Clone, Debug)]
pub(super) struct Member {
    pub(super) record: Record,
    pub(super) state: FleetCombatState,
    /// The fleet's transports by cargo state, each with its own crippled
    /// cohorts. Transport losses fall on the loaded ones first.
    pub(super) transports: Transports,
    pub(super) roe: u8,
    /// Set once the fleet has broken off: it is out of the battle. A
    /// starbase never leaves.
    pub(super) left: Option<Withdrawal>,
}

impl Member {
    pub(super) fn fleet_idx(&self) -> Option<usize> {
        match self.record {
            Record::Fleet(idx) => Some(idx),
            Record::Base(_) => None,
        }
    }

    /// Still in the battle: it has not left.
    pub(super) fn fighting(&self) -> bool {
        self.left.is_none()
    }
}

/// One empire's side of a battle: its fleets and starbases in the sector
/// and layer, each with its own hulls, and the aggregate they fire with.
#[derive(Clone, Debug)]
pub(super) struct TaskForce {
    pub(super) empire: u8,
    pub(super) members: Vec<Member>,
    /// The hulls of the members still in the battle: what the side fires
    /// with and what fire lands on. Rebuilt whenever a member leaves.
    pub(super) state: FleetCombatState,
    pub(super) role: BattleRole,
    pub(super) engaged_in_battle: bool,
    /// Hits carried against this side from earlier exchanges of this
    /// battle. They belong to the side, not to its fleets: a fleet that
    /// leaves takes none with them, and the battle's end drops them.
    pub(super) carried_hits: CarriedHits,
}

impl TaskForce {
    /// Every fleet on the roster, in it or not.
    pub(super) fn fleet_indices(&self) -> Vec<usize> {
        self.members.iter().filter_map(Member::fleet_idx).collect()
    }

    /// The aggregate of the members still in the battle.
    pub(super) fn rebuild(&mut self) {
        self.state = sum_states(self.members.iter().filter(|m| m.fighting()));
    }

    /// The hulls of the whole roster, departed fleets included, as the
    /// battle left them.
    pub(super) fn full_state(&self) -> FleetCombatState {
        sum_states(self.members.iter())
    }

    /// A fleet has left the battle.
    pub(super) fn any_left(&self) -> bool {
        self.members.iter().any(|m| m.left.is_some())
    }
}

fn sum_states<'a>(members: impl Iterator<Item = &'a Member>) -> FleetCombatState {
    let mut state = FleetCombatState::default();
    for member in members {
        for idx in 0..7 {
            state.counts[idx] += member.state.counts[idx];
            state.crippled[idx].add(&member.state.crippled[idx]);
        }
    }
    state
}

impl FleetCombatState {
    pub(super) fn total_combat_as(&self) -> u32 {
        self.suppression_as() + self.execution_as()
    }

    pub(super) fn suppression_as(&self) -> u32 {
        [IDX_CA, IDX_BB]
            .into_iter()
            .map(|idx| {
                let as_value = fleet_class_as(idx);
                hull_attack_strength(as_value, self.counts[idx], self.crippled_count(idx))
            })
            .sum()
    }

    pub(super) fn execution_as(&self) -> u32 {
        [IDX_DD, IDX_SB]
            .into_iter()
            .map(|idx| {
                let as_value = fleet_class_as(idx);
                hull_attack_strength(as_value, self.counts[idx], self.crippled_count(idx))
            })
            .sum()
    }

    pub(super) fn has_units(&self) -> bool {
        self.counts.iter().any(|&c| c > 0)
    }

    pub(super) fn crippled_count(&self, idx: usize) -> u32 {
        self.crippled[idx].total()
    }

    pub(super) fn nominal_count(&self, idx: usize) -> u32 {
        self.counts[idx].saturating_sub(self.crippled_count(idx))
    }

    /// `count` nominal hulls of class `idx` are crippled.
    pub(super) fn cripple(&mut self, idx: usize, count: u32) {
        self.crippled[idx].cripple(count);
    }

    /// `count` crippled hulls of class `idx` are destroyed, those crippled
    /// this pass first and then the ones with the most turns left.
    pub(super) fn finish_crippled(&mut self, idx: usize, count: u32) {
        self.crippled[idx].take(count);
        self.counts[idx] -= count;
    }

    /// The side has ships, as opposed to starbases only.
    pub(super) fn has_ships(&self) -> bool {
        self.counts
            .iter()
            .enumerate()
            .any(|(idx, &count)| idx != IDX_SB && count > 0)
    }

    /// The speed the side's ships can make together: that of the slowest
    /// class still present, as the fleet's maximum speed is recomputed
    /// after the battle.
    pub(super) fn ship_speed(&self) -> u8 {
        let count = |idx: usize| self.counts[idx].min(u32::from(u16::MAX)) as u16;
        slowest_speed(&ShipCounts {
            battleships: count(IDX_BB),
            cruisers: count(IDX_CA),
            destroyers: count(IDX_DD),
            empty_transports: count(IDX_TT),
            loaded_transports: 0,
            etacs: count(IDX_ET),
            scouts: count(IDX_SC),
        })
    }
}

/// The attack strength of a fleet's warships: the yardstick the decision to
/// fight shares with the battle. A crippled hull counts at its crippled AS.
pub(crate) fn warship_attack_strength(
    destroyers: u16,
    cruisers: u16,
    battleships: u16,
    crippled: &CrippledShips,
) -> u32 {
    hull_attack_strength(AS_DD, destroyers.into(), crippled.destroyers.total())
        + hull_attack_strength(AS_CA, cruisers.into(), crippled.cruisers.total())
        + hull_attack_strength(AS_BB, battleships.into(), crippled.battleships.total())
}

/// A starbase's attack strength: 100, or 50 while it is crippled.
pub(crate) fn base_attack_strength(base: &Starbase) -> u32 {
    hull_attack_strength(AS_SB, 1, base.crippled.total().min(1))
}

/// `hulls` of one class, `crippled` of them crippled, each hull at its
/// current AS.
fn hull_attack_strength(as_value: u32, hulls: u32, crippled: u32) -> u32 {
    hulls.saturating_sub(crippled) * as_value + crippled * crippled_as(as_value)
}

/// The attack strength of a side's warships, each hull at its current AS.
pub(super) fn warship_attack_as(state: &FleetCombatState) -> u32 {
    [IDX_DD, IDX_CA, IDX_BB]
        .into_iter()
        .map(|idx| {
            hull_attack_strength(
                fleet_class_as(idx),
                state.counts[idx],
                state.crippled_count(idx),
            )
        })
        .sum()
}

fn crippled_as(as_value: u32) -> u32 {
    as_value / 2
}

/// The combat-state index of a ship class.
pub(super) fn class_index(class: HullClass) -> usize {
    match class {
        HullClass::Destroyer => IDX_DD,
        HullClass::Cruiser => IDX_CA,
        HullClass::Battleship => IDX_BB,
        HullClass::Scout => IDX_SC,
        HullClass::LoadedTransport | HullClass::EmptyTransport => IDX_TT,
        HullClass::Etac => IDX_ET,
    }
}

/// The state of the fleets and starbase records named: their hulls, and
/// the damage they carry in from earlier battles.
pub(super) fn fleet_state_from_records(
    game_data: &CoreGameData,
    fleet_indices: &[usize],
    bases: &[usize],
) -> FleetCombatState {
    let mut state = FleetCombatState::default();
    for &idx in fleet_indices {
        let fleet = &game_data.fleets[idx];
        state.counts[IDX_DD] += fleet.ships.destroyers as u32;
        state.counts[IDX_CA] += fleet.ships.cruisers as u32;
        state.counts[IDX_BB] += fleet.ships.battleships as u32;
        state.counts[IDX_SC] += fleet.ships.scouts as u32;
        state.counts[IDX_TT] += troop_transports(fleet) as u32;
        state.counts[IDX_ET] += fleet.ships.etacs as u32;
        for class in HullClass::ALL {
            state.crippled[class_index(class)].add(fleet.crippled.get(class));
        }
    }
    state.counts[IDX_SB] = bases.len() as u32;
    for &idx in bases {
        state.crippled[IDX_SB].add(&game_data.bases[idx].crippled);
    }
    state
}

fn fleet_class_as(idx: usize) -> u32 {
    match idx {
        IDX_DD => AS_DD,
        IDX_CA => AS_CA,
        IDX_BB => AS_BB,
        IDX_SB => AS_SB,
        IDX_SC => AS_SC,
        IDX_TT => AS_TT,
        IDX_ET => AS_ET,
        _ => 0,
    }
}

pub(super) fn fleet_class_ds(idx: usize) -> u32 {
    match idx {
        IDX_DD => DS_DD,
        IDX_CA => DS_CA,
        IDX_BB => DS_BB,
        IDX_SB => DS_SB,
        IDX_SC => DS_SC,
        IDX_TT => DS_TT,
        IDX_ET => DS_ET,
        _ => 0,
    }
}

pub(super) fn fleet_target_order() -> [usize; 7] {
    [IDX_DD, IDX_SC, IDX_TT, IDX_ET, IDX_CA, IDX_BB, IDX_SB]
}

pub(super) fn fleet_combat_line_order() -> [usize; 4] {
    [IDX_DD, IDX_CA, IDX_BB, IDX_SB]
}

pub(super) fn fleet_auxiliary_order() -> [usize; 3] {
    [IDX_SC, IDX_TT, IDX_ET]
}

pub(super) fn fleet_combat_line_present(state: &FleetCombatState) -> bool {
    fleet_combat_line_order()
        .into_iter()
        .any(|idx| state.counts[idx] > 0)
}

fn task_force_role(
    game_data: &CoreGameData,
    empire: u8,
    coords: [u8; 2],
    fleet_indices: &[usize],
) -> BattleRole {
    if let Some(planet) = game_data
        .planets
        .iter()
        .find(|p| p.sector.to_array() == coords)
    {
        if planet.owner.0 == empire {
            return BattleRole::IncumbentDefender;
        }
    }
    let guarding = fleet_indices.iter().any(|&idx| {
        matches!(
            game_data.fleets[idx].order,
            Order::GuardStarbase | Order::GuardBlockadeWorld
        )
    });
    if guarding {
        BattleRole::GuardingDefender
    } else if !fleet_indices.is_empty() {
        BattleRole::Attacker
    } else {
        BattleRole::Neutral
    }
}

pub(super) fn new_task_force(
    game_data: &CoreGameData,
    empire: u8,
    coords: [u8; 2],
    fleet_indices: Vec<usize>,
    bases: &[usize],
) -> TaskForce {
    let role = task_force_role(game_data, empire, coords, &fleet_indices);
    let fleets = fleet_indices.iter().map(|&idx| {
        let fleet = &game_data.fleets[idx];
        Member {
            record: Record::Fleet(idx),
            state: fleet_state_from_records(game_data, &[idx], &[]),
            transports: Transports::from_record(fleet),
            roe: fleet.rules_of_engagement,
            left: None,
        }
    });
    let bases = bases.iter().map(|&idx| Member {
        record: Record::Base(idx),
        state: fleet_state_from_records(game_data, &[], &[idx]),
        transports: Transports::default(),
        roe: 0,
        left: None,
    });
    let mut task_force = TaskForce {
        empire,
        members: fleets.chain(bases).collect(),
        state: FleetCombatState::default(),
        role,
        engaged_in_battle: false,
        carried_hits: CarriedHits::default(),
    };
    task_force.rebuild();
    task_force
}

/// The ships of fleet `fleet_idx` as `state` has them: counts by class and
/// the crippled cohorts, the transports by cargo state from `transports`.
pub(super) fn write_fleet_state(
    game_data: &mut CoreGameData,
    fleet_idx: usize,
    state: &FleetCombatState,
    transports: &Transports,
) -> bool {
    debug_assert_eq!(
        transports.total().0,
        state.counts[IDX_TT],
        "the cargo subgroups add up to the transports"
    );
    let fleet = &mut game_data.fleets[fleet_idx];
    let mut lost = false;
    for class in HullClass::ALL {
        let idx = class_index(class);
        let (count, crippled) = match class {
            HullClass::LoadedTransport => (transports.loaded.hulls, transports.loaded.crippled),
            HullClass::EmptyTransport => (transports.empty.hulls, transports.empty.crippled),
            _ => (state.counts[idx], state.crippled[idx]),
        };
        lost |= count < hull_count(fleet, class);
        *fleet.crippled.get_mut(class) = crippled;
        let count = count.min(u32::from(u16::MAX)) as u16;
        match class {
            HullClass::Destroyer => fleet.ships.destroyers = count,
            HullClass::Cruiser => fleet.ships.cruisers = count,
            HullClass::Battleship => fleet.ships.battleships = count,
            HullClass::Scout => fleet.ships.scouts = count,
            HullClass::LoadedTransport => fleet.ships.loaded_transports = count,
            HullClass::EmptyTransport => fleet.ships.empty_transports = count,
            HullClass::Etac => fleet.ships.etacs = count,
        }
    }
    lost
}

pub(super) fn ship_losses_from_states(
    before: &FleetCombatState,
    after: &FleetCombatState,
) -> ShipLosses {
    ShipLosses {
        destroyers: before.counts[IDX_DD].saturating_sub(after.counts[IDX_DD]),
        cruisers: before.counts[IDX_CA].saturating_sub(after.counts[IDX_CA]),
        battleships: before.counts[IDX_BB].saturating_sub(after.counts[IDX_BB]),
        scouts: before.counts[IDX_SC].saturating_sub(after.counts[IDX_SC]),
        transports: before.counts[IDX_TT].saturating_sub(after.counts[IDX_TT]),
        etacs: before.counts[IDX_ET].saturating_sub(after.counts[IDX_ET]),
    }
}

pub(super) fn ship_counts_from_state(state: &FleetCombatState) -> ShipLosses {
    ShipLosses {
        destroyers: state.counts[IDX_DD],
        cruisers: state.counts[IDX_CA],
        battleships: state.counts[IDX_BB],
        scouts: state.counts[IDX_SC],
        transports: state.counts[IDX_TT],
        etacs: state.counts[IDX_ET],
    }
}

/// The roster has a hull left anywhere, departed fleets included.
pub(super) fn tf_has_any_units(tf: &TaskForce) -> bool {
    tf.full_state().has_units()
}

pub(super) fn planet_idx_at_coords(game_data: &CoreGameData, coords: [u8; 2]) -> Option<usize> {
    game_data
        .planets
        .iter()
        .position(|planet| planet.sector.to_array() == coords)
}
