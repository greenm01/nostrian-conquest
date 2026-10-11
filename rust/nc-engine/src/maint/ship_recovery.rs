//! Automatic ship recovery (combat decision 3, RT). Crippled hulls stay
//! crippled through every battle of a pass; once per pass, after the 52
//! ticks, the post-year merges, and colonization, and before the AI and the
//! reports, every turns-remaining count drops by one, hulls that reach zero
//! are whole again, and the hulls crippled this pass enter at RT. RT is 2
//! for every class: a hull crippled in pass N stays crippled through
//! passes N+1 and N+2, recovers when pass N+2 ends, and fights whole from
//! N+3.
//!
//! A hull can recover one pass early with a starbase's help. The engine
//! decides that from a snapshot taken when the pass's ticks begin and kept
//! only for the pass, so it needs no stored state. The snapshot names the
//! crippled hulls each fleet had then, if the fleet rests in orbit of its
//! owner's world beside one of the owner's starbases; every unit's turn
//! strikes off what no longer qualifies. A hull crippled during the pass,
//! or that merged or arrived during it, is never in the snapshot. A
//! starbase does not assist its own recovery.

use nc_data::{
    Coords, CoreGameData, CrippledShips, HullClass, Real48, Recovered, RecoveredUnit,
    ShipsRecoveredEvent, Starbase, has_any_force,
};

/// A fleet in the snapshot: where it rests, the starbases that were there
/// with it, and the hulls that still qualify for the extra turn.
#[derive(Debug)]
struct Watched {
    /// The fleet's record. Records keep their places for the whole year.
    fleet_idx: usize,
    owner: u8,
    fleet_number: u16,
    sector: Coords,
    x_fraction: Real48,
    y_fraction: Real48,
    depth: Real48,
    planet_idx: usize,
    /// The records and ids of the owner's bases at rest in orbit there at
    /// the start. The fleet qualifies while any one of them stays.
    bases: Vec<(usize, u16)>,
    /// The fleet's crippled hulls that still qualify, by turns left;
    /// `fresh` is always zero.
    assisted: CrippledShips,
}

/// The pass's snapshot for starbase assistance.
#[derive(Debug, Default)]
pub(super) struct AssistWatch {
    entries: Vec<Watched>,
}

/// A base at rest in orbit of the owner's world: at depth 0.5 or more with
/// its moving flag clear, as the growth bonus reads it.
fn rests_with(base: &Starbase, owner: u8, sector: Coords) -> bool {
    base.base_id.0 != 0
        && base.owner.0 == owner
        && base.sector == sector
        && base.depth >= Real48::HALF
        && !base.moving
}

impl Watched {
    /// Whether the fleet still qualifies after a unit's turn, narrowing
    /// its assisted hulls to those it still has. Hulls that were never
    /// assisted are the first lost, so a loss costs assistance only once it
    /// eats into the assisted hulls themselves.
    fn observe(&mut self, game_data: &CoreGameData) -> bool {
        let fleet = &game_data.fleets[self.fleet_idx];
        let stayed = fleet.owner.0 == self.owner
            && fleet.fleet_number.0 == self.fleet_number
            && has_any_force(fleet)
            && fleet.sector == self.sector
            && fleet.x_fraction == self.x_fraction
            && fleet.y_fraction == self.y_fraction
            && fleet.depth == self.depth
            && game_data.planets[self.planet_idx].owner.0 == self.owner;
        if !stayed {
            return false;
        }
        self.bases.retain(|&(idx, id)| {
            let base = &game_data.bases[idx];
            base.base_id.0 == id && rests_with(base, self.owner, self.sector)
        });
        if self.bases.is_empty() {
            return false;
        }
        for class in HullClass::ALL {
            let present = fleet.crippled.get(class);
            let kept = self.assisted.get(class).limited_to(present);
            *self.assisted.get_mut(class) = kept;
        }
        true
    }
}

impl AssistWatch {
    /// Snapshot the fleets that have crippled hulls and rest in orbit of
    /// their owner's world with one of the owner's bases at rest there.
    pub(super) fn begin(game_data: &CoreGameData) -> Self {
        let entries = game_data
            .fleets
            .iter()
            .enumerate()
            .filter(|(_, fleet)| {
                fleet.owner.0 != 0
                    && has_any_force(fleet)
                    && !fleet.crippled.is_empty()
                    && fleet.depth >= Real48::HALF
            })
            .filter_map(|(fleet_idx, fleet)| {
                let planet_idx = game_data.planets.iter().position(|planet| {
                    planet.sector == fleet.sector && planet.owner == fleet.owner
                })?;
                let bases: Vec<(usize, u16)> = game_data
                    .bases
                    .iter()
                    .enumerate()
                    .filter(|(_, base)| rests_with(base, fleet.owner.0, fleet.sector))
                    .map(|(idx, base)| (idx, base.base_id.0))
                    .collect();
                if bases.is_empty() {
                    return None;
                }
                let mut assisted = fleet.crippled;
                for class in HullClass::ALL {
                    assisted.get_mut(class).fresh = 0;
                }
                Some(Watched {
                    fleet_idx,
                    owner: fleet.owner.0,
                    fleet_number: fleet.fleet_number.0,
                    sector: fleet.sector,
                    x_fraction: fleet.x_fraction,
                    y_fraction: fleet.y_fraction,
                    depth: fleet.depth,
                    planet_idx,
                    bases,
                    assisted,
                })
            })
            .collect();
        Self { entries }
    }

    /// A unit has taken its turn: strike off the fleets that moved, lost
    /// their world or their base, or changed hands.
    pub(super) fn observe(&mut self, game_data: &CoreGameData) {
        self.entries.retain_mut(|entry| entry.observe(game_data));
    }

    /// The hulls of fleet `fleet_number` of `owner` that qualified for the
    /// extra turn through the whole pass, if any did.
    fn assisted(&self, owner: u8, fleet_number: u16) -> Option<&CrippledShips> {
        self.entries
            .iter()
            .find(|entry| entry.owner == owner && entry.fleet_number == fleet_number)
            .map(|entry| &entry.assisted)
    }
}

/// The recovery step. Every fleet's and starbase's crippled counts drop by
/// one turn (the assisted hulls by two), hulls that reach zero are whole,
/// and this pass's newly crippled hulls enter at RT. Returns what recovered.
pub(super) fn recover_ships(
    game_data: &mut CoreGameData,
    watch: &AssistWatch,
) -> Vec<ShipsRecoveredEvent> {
    let mut events = Vec::new();
    for fleet in &mut game_data.fleets {
        if fleet.crippled.is_empty() {
            continue;
        }
        let assisted = watch.assisted(fleet.owner.0, fleet.fleet_number.0);
        let mut recovered = Recovered::default();
        for class in HullClass::ALL {
            let help = assisted
                .map(|assisted| assisted.get(class).limited_to(fleet.crippled.get(class)))
                .unwrap_or_default();
            let step = fleet.crippled.get_mut(class).recover(&help);
            recovered.hulls += step.hulls;
            recovered.early += step.early;
        }
        if recovered.hulls > 0 {
            events.push(ShipsRecoveredEvent {
                owner_empire: fleet.owner.0,
                unit: RecoveredUnit::Fleet {
                    fleet_number: fleet.fleet_number.0,
                },
                coords: fleet.sector.to_array(),
                in_system: fleet.depth >= Real48::HALF,
                hulls: recovered.hulls,
                early: recovered.early,
            });
        }
    }
    for base in &mut game_data.bases {
        if base.base_id.0 == 0 || base.crippled.is_empty() {
            continue;
        }
        let step = base.crippled.recover(&Default::default());
        if step.hulls > 0 {
            events.push(ShipsRecoveredEvent {
                owner_empire: base.owner.0,
                unit: RecoveredUnit::Starbase {
                    base_id: base.base_id.0,
                },
                coords: base.sector.to_array(),
                in_system: base.depth >= Real48::HALF,
                hulls: step.hulls,
                early: 0,
            });
        }
    }
    events
}

#[cfg(test)]
#[path = "ship_recovery_tests.rs"]
mod tests;
