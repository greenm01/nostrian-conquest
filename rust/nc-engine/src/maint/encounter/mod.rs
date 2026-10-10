//! Classic ECMAINT's year: 52 ticks, and in each tick every fleet and
//! starbase in a random order moves, scans the units sharing its sector and
//! depth layer, and runs its order's arrival test
//! (`docs/dev/ec-encounter-spec.md`).

mod handler;
pub mod rules;

use nc_data::{DiplomacyOverride, GameRng, RNG_TAG_ENCOUNTER, Real48};

use super::combat::FleetBattlePhaseEvents;
use super::movement::YearMovement;
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
        .records
        .iter()
        .enumerate()
        .map(|(index, fleet)| Unit {
            kind: UnitKind::Fleet,
            index,
            owner: fleet.owner_empire_raw(),
            sector: fleet.current_location_coords_raw(),
            layer: layer_of(fleet.depth()),
            active: fleet.has_any_force(),
        });
    let bases = game_data
        .bases
        .records
        .iter()
        .enumerate()
        .map(|(index, base)| Unit {
            kind: UnitKind::Base,
            index,
            owner: base.owner_empire_raw(),
            sector: base.coords_raw(),
            // Starbases do not move yet; they stay at their stored depth.
            layer: layer_of(base.depth()),
            active: base.base_id_raw() != 0,
        });
    fleets.chain(bases).collect()
}

/// What the year's ticks produced.
pub(super) struct YearOutcome {
    pub movement_events: MovementEvents,
    pub movement_remap: FleetRemovalRemapInfo,
    /// Sightings, decisions, and battles.
    pub encounter_events: FleetBattlePhaseEvents,
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
        &[u64::from(game_data.conquest.game_year())],
    );
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
    };
    for tick in 1..=TICKS_PER_YEAR {
        for &unit in &order {
            year.run_unit(game_data, unit, tick)?;
        }
    }
    let (movement_events, movement_remap) = year.movement.finish(game_data);
    Ok(YearOutcome {
        movement_events,
        movement_remap,
        encounter_events: year.events,
    })
}
