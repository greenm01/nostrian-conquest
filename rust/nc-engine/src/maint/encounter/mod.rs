//! Classic ECMAINT's year: 52 ticks, and in each tick every fleet and
//! starbase in a random order moves, scans the units sharing its sector and
//! depth layer, and runs its order's arrival test
//! (`docs/dev/ec-encounter-spec.md`). A hostile order whose strike test
//! passes attacks its world in that tick.

mod handler;
pub mod rules;

use std::collections::HashSet;

use nc_data::{DiplomacyOverride, GameRng, RNG_TAG_ENCOUNTER, Real48, has_any_force};

use super::combat::{AssaultEvents, FleetBattlePhaseEvents};
use super::movement::YearMovement;
use super::ship_recovery::AssistWatch;
use super::{FleetRemovalRemapInfo, MovementEvents};
use crate::navigation::motion::TICKS_PER_YEAR;
use crate::{CoreGameData, VisibleHazardIntel};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UnitKind {
    Fleet,
    Base,
}

/// One entry of classic's unit table (`2895:56d4`): a fleet or a starbase,
/// where it is, and whether it is still in play.
#[derive(Debug, Clone, Copy)]
struct Unit {
    kind: UnitKind,
    /// Record index in `fleets.records` or `bases.records`.
    index: usize,
    owner: u8,
    sector: [u8; 2],
    layer: u8,
    active: bool,
}

/// An empire's units standing in a sector and depth layer: the place its
/// sighting memory is kept (`SightingMemory`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Presence {
    empire: u8,
    sector: [u8; 2],
    layer: u8,
}

impl Presence {
    fn of(unit: &Unit) -> Self {
        Self {
            empire: unit.owner,
            sector: unit.sector,
            layer: unit.layer,
        }
    }
}

/// Classic keeps one awareness byte per empire per unit.
const EMPIRE_SLOTS: usize = 25;

/// How much each empire knows about each unit (0 unknown, 1 contact, 2
/// identified, 3 fought). It is shared by all of an empire's units, only
/// rises, and is forgotten when the unit changes sector or layer.
struct Awareness(Vec<[u8; EMPIRE_SLOTS]>);

impl Awareness {
    fn get(&self, empire: u8, unit: usize) -> u8 {
        empire_slot(empire).map_or(0, |slot| self.0[unit][slot])
    }

    fn raise(&mut self, empire: u8, unit: usize, level: u8) {
        if let Some(slot) = empire_slot(empire) {
            let known = &mut self.0[unit][slot];
            *known = (*known).max(level);
        }
    }

    fn forget(&mut self, unit: usize) {
        self.0[unit] = [0; EMPIRE_SLOTS];
    }
}

fn empire_slot(empire: u8) -> Option<usize> {
    usize::from(empire)
        .checked_sub(1)
        .filter(|slot| *slot < EMPIRE_SLOTS)
}

/// 1 inside a world's system (depth at least 0.5), else 0.
fn layer_of(depth: Real48) -> u8 {
    u8::from(depth >= Real48::HALF)
}

fn build_units(game_data: &CoreGameData) -> Vec<Unit> {
    let fleets = game_data
        .fleets
        .iter()
        .enumerate()
        .map(|(index, fleet)| Unit {
            kind: UnitKind::Fleet,
            index,
            owner: fleet.owner.0,
            sector: fleet.sector.to_array(),
            layer: layer_of(fleet.depth),
            active: has_any_force(fleet),
        });
    let bases = game_data
        .bases
        .iter()
        .enumerate()
        .map(|(index, base)| Unit {
            kind: UnitKind::Base,
            index,
            owner: base.owner.0,
            sector: base.sector.to_array(),
            layer: layer_of(base.depth),
            active: base.base_id.0 != 0,
        });
    fleets.chain(bases).collect()
}

/// Drop the sighting memory of every empire that has no fleet or starbase
/// left in the sector and layer it remembers. The ticks keep this true as
/// units move and die; this settles what happens outside them, in the
/// orders phase and after the year (colonization, defections).
pub(super) fn prune_sightings(game_data: &mut CoreGameData) {
    if game_data.sightings.is_empty() {
        return;
    }
    let present: HashSet<(u8, [u8; 2], u8)> = build_units(game_data)
        .iter()
        .filter(|unit| unit.active)
        .map(|unit| (unit.owner, unit.sector, unit.layer))
        .collect();
    game_data
        .sightings
        .retain(|viewer, sector, layer| present.contains(&(viewer, sector.to_array(), layer)));
}

/// What the year's ticks produced.
pub(super) struct YearOutcome {
    pub movement_events: MovementEvents,
    pub movement_remap: FleetRemovalRemapInfo,
    /// Sightings, decisions, and battles.
    pub encounter_events: FleetBattlePhaseEvents,
    /// Planetary strikes, each in its tick.
    pub assault_events: AssaultEvents,
    /// Which resting fleets' crippled hulls kept their starbase through
    /// every tick, for the recovery step.
    pub assist_watch: AssistWatch,
}

/// Run the year's 52 ticks (`0824:1004`).
pub(super) fn run_year(
    game_data: &mut CoreGameData,
    campaign_seed: u64,
    visible_hazards_by_empire: &[VisibleHazardIntel],
    diplomacy_overrides: &[DiplomacyOverride],
) -> Result<YearOutcome, Box<dyn std::error::Error>> {
    let mut rng = GameRng::from_context(
        campaign_seed,
        RNG_TAG_ENCOUNTER,
        &[u64::from(game_data.conquest.game_year)],
    );
    prune_sightings(game_data);
    let units = build_units(game_data);
    // The order is drawn once per year: a Random(0xFFDC) key per unit,
    // sorted descending, ties in table order.
    let keys: Vec<u64> = units.iter().map(|_| rng.below(0xFFDC)).collect();
    let mut order: Vec<usize> = (0..units.len()).collect();
    order.sort_by(|a, b| keys[*b].cmp(&keys[*a]));

    let mut year = handler::Year {
        awareness: Awareness(vec![[0; EMPIRE_SLOTS]; units.len()]),
        units,
        rng,
        campaign_seed,
        diplomacy_overrides,
        movement: YearMovement::new(game_data, visible_hazards_by_empire),
        events: FleetBattlePhaseEvents::default(),
        assault_events: AssaultEvents::default(),
        departed: Vec::new(),
        bombarded: Vec::new(),
    };
    // The starbase's extra recovery turn is decided from what rests where
    // when the ticks begin, and each unit's turn can only strike entries off.
    let mut assist_watch = AssistWatch::begin(game_data);
    for tick in 1..=TICKS_PER_YEAR {
        for &unit in &order {
            year.run_unit(game_data, unit, tick)?;
            assist_watch.observe(game_data);
        }
    }
    let (movement_events, movement_remap) = year.movement.finish(game_data);
    Ok(YearOutcome {
        movement_events,
        movement_remap,
        encounter_events: year.events,
        assault_events: year.assault_events,
        assist_watch,
    })
}
