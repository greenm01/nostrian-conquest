//! Starbases in the year's ticks (`docs/dev/ec-starbase-spec.md`): the
//! speed-1 step that carries the escort and retargets the guards
//! (ECMAINT `0824:02d8`), and the dispatcher's arrival and friendly-world
//! checks (`0b5d:5793`).

use super::YearMovement;
use crate::CoreGameData;
use crate::maint::merging::merge_fleet_into_host;
use crate::navigation::motion::{Course, Motion, TICKS_PER_YEAR, is_at, step, target_depth};
use nc_data::{
    FleetMergeEvent, LostWorldResponse, Mission, Order, Real48, StarbaseReport, StarbaseReportEvent,
};

/// A fleet cannot grow past this many ships by merging (`2895:3057`).
const MAX_MERGED_SHIPS: u32 = 3000;

/// What a starbase's step changed, for the unit table.
#[derive(Debug, Default)]
pub(in crate::maint) struct BaseStep {
    /// The base moved to another sector or depth layer.
    pub moved_cell: bool,
    /// The escort it carried along.
    pub escort: Option<usize>,
    /// Guards merged into the escort; they leave play.
    pub merged: Vec<usize>,
}

/// Whether `fleet` guards base `base_id` of empire `owner`.
fn guards(fleet: &nc_data::FleetRecord, owner: u8, base_id: u16) -> bool {
    fleet.owner_empire_raw() == owner
        && fleet.standing_order_kind() == Order::GuardStarbase
        && fleet.guard_starbase_index_raw() == base_id
}

/// 1 inside a world's system (depth 0.5 or more), else 0.
fn layer(depth: Real48) -> u8 {
    u8::from(depth.round() == 1)
}

/// Whether two fleets merged would stay within the ship limit.
pub(super) fn merge_fits(game_data: &CoreGameData, host: usize, donor: usize) -> bool {
    let fleets = &game_data.fleets.records;
    fleets[host].total_starships() + fleets[donor].total_starships() <= MAX_MERGED_SHIPS
}

impl YearMovement<'_> {
    /// A fleet still in play: it has ships and has not merged away.
    pub(super) fn in_play(&self, game_data: &CoreGameData, fleet_idx: usize) -> bool {
        !self.to_remove[fleet_idx] && game_data.fleets.records[fleet_idx].has_any_force()
    }

    /// Step base `base_idx` one tick at speed 1 (`0824:02d8`).
    pub(in crate::maint) fn step_base(
        &mut self,
        game_data: &mut CoreGameData,
        base_idx: usize,
        tick: u8,
    ) -> BaseStep {
        let base = &mut game_data.bases.records[base_idx];
        if base.target_depth_raw() > 1 {
            base.set_target_depth_raw(1);
        }
        let target = base.target_coords_raw();
        let depth = target_depth(base.target_depth_raw());
        let motion = Motion::of_base(base);
        if is_at(motion, target, depth) {
            return BaseStep::default();
        }
        let escort = self.find_escort(game_data, base_idx);
        let stepped = step(motion, 1, &mut Course::direct(target), depth);
        let base = &mut game_data.bases.records[base_idx];
        stepped.write_to_base(base);
        let base = base.clone();
        let (owner, base_id) = (base.owner_empire_raw(), base.base_id_raw());
        let (sector, base_layer) = (base.coords_raw(), layer(base.depth()));
        let mut result = BaseStep {
            moved_cell: stepped.cell() != motion.cell(),
            escort,
            merged: Vec::new(),
        };
        if let Some(escort) = escort {
            self.carry_escort(game_data, escort, base_idx);
        }
        if !result.moved_cell {
            return result;
        }
        // Every guard heads for the base's new sector and layer; one that
        // already shares them joins the escort (`0824:0e2a`).
        for fleet_idx in 0..game_data.fleets.records.len() {
            if !self.in_play(game_data, fleet_idx)
                || !guards(&game_data.fleets.records[fleet_idx], owner, base_id)
            {
                continue;
            }
            let fleet = &mut game_data.fleets.records[fleet_idx];
            fleet.set_standing_order_target_coords_raw(sector);
            fleet.set_mission_target(base_layer, fleet.mission_id_raw());
            self.tracks[fleet_idx].reload();
            let co_located =
                fleet.current_location_coords_raw() == sector && layer(fleet.depth()) == base_layer;
            if let Some(escort) = escort
                && co_located
                && fleet_idx != escort
            {
                self.merge_guard(game_data, escort, fleet_idx, tick, false);
                result.merged.push(fleet_idx);
            }
        }
        result
    }

    /// The escort rides with the base (`0824:0c46`): it takes the base's
    /// position and depth, and aims at the base's sector and layer.
    fn carry_escort(&mut self, game_data: &mut CoreGameData, escort: usize, base_idx: usize) {
        let base = &game_data.bases.records[base_idx];
        let fleet = &mut game_data.fleets.records[escort];
        fleet.set_current_location_coords_raw(base.coords_raw());
        fleet.set_x_fraction(base.x_fraction());
        fleet.set_y_fraction(base.y_fraction());
        fleet.set_depth(base.depth());
        fleet.set_standing_order_target_coords_raw(base.coords_raw());
        fleet.set_mission_target(layer(base.depth()), fleet.mission_id_raw());
        self.tracks[escort].reload();
    }

    /// The fleet that rides with base `base_idx` (`0824:042a`): the first
    /// guard of the base, at rest, that is its recorded escort or shares
    /// its sector and layer.
    fn find_escort(&self, game_data: &CoreGameData, base_idx: usize) -> Option<usize> {
        let base = &game_data.bases.records[base_idx];
        let (owner, base_id) = (base.owner_empire_raw(), base.base_id_raw());
        let cell = (base.coords_raw(), layer(base.depth()));
        (0..game_data.fleets.records.len()).find(|&fleet_idx| {
            let fleet = &game_data.fleets.records[fleet_idx];
            self.in_play(game_data, fleet_idx)
                && fleet.owner_empire_raw() == owner
                && (fleet.local_slot_word_raw() == base.escort_fleet_slot_raw()
                    || (fleet.current_location_coords_raw(), layer(fleet.depth())) == cell)
                && guards(fleet, owner, base_id)
                && fleet.current_speed() == 0
        })
    }

    /// Merge guard `donor` into `host`; the donor leaves play. `reported`
    /// marks the merge of an arriving guard, which reports it.
    pub(super) fn merge_guard(
        &mut self,
        game_data: &mut CoreGameData,
        host: usize,
        donor: usize,
        tick: u8,
        reported: bool,
    ) {
        merge_fleet_into_host(game_data, host, donor);
        self.to_remove[donor] = true;
        let fleets = &game_data.fleets.records;
        let event = FleetMergeEvent {
            fleet_idx: donor,
            owner_empire_raw: fleets[donor].owner_empire_raw(),
            kind: Mission::GuardStarbase,
            host_fleet_id_raw: fleets[host].fleet_id_word_raw(),
            absorbed_fleet_id_raw: fleets[donor].fleet_id_word_raw(),
            host_fleet_number: fleets[host].local_slot_word_raw() as u8,
            absorbed_fleet_number: fleets[donor].local_slot_word_raw() as u8,
            coords: fleets[donor].current_location_coords_raw(),
            survivor_side: false,
            stardate_week: Some(tick),
        };
        self.events.guard_merges.push((event, reported));
    }

    /// The base's arrival test and friendly-world check after its step and
    /// scan (`0b5d:5793`).
    pub(in crate::maint) fn dispatch_base(
        &mut self,
        game_data: &mut CoreGameData,
        base_idx: usize,
        tick: u8,
    ) {
        let last_tick = tick == TICKS_PER_YEAR;
        let base = &game_data.bases.records[base_idx];
        let owner = base.owner_empire_raw();
        let sector = base.coords_raw();
        let target = base.target_coords_raw();
        let target_layer = base.target_depth_raw();
        let depth = base.depth();
        let at_target = sector == target
            && (depth == Real48::from_int(i32::from(target_layer))
                || (last_tick && depth.round() == i32::from(target_layer)));
        let mut moving = base.moving_flag_raw() != 0;
        let report = |game_data: &CoreGameData, report| {
            let base = &game_data.bases.records[base_idx];
            StarbaseReportEvent {
                owner_empire_raw: owner,
                starbase_id: base.base_id_raw(),
                coords: base.coords_raw(),
                in_system: layer(base.depth()) == 1,
                report,
                stardate_week: Some(tick),
            }
        };
        if at_target {
            if moving {
                moving = false;
                self.events.starbase_reports.push(report(
                    game_data,
                    StarbaseReport::Arrived {
                        orbit: target_layer == 1,
                    },
                ));
            }
        } else {
            moving = true;
        }
        game_data.bases.records[base_idx].set_moving_flag_raw(u8::from(moving));

        // A base bound for a world, or resting in orbit, needs its owner to
        // hold that world.
        let world_sector = if moving {
            (target_layer == 1).then_some(target)
        } else {
            (depth.round() == 1).then_some(sector)
        };
        let Some(world_sector) = world_sector else {
            return;
        };
        let planet_idx = game_data
            .planets
            .records
            .iter()
            .position(|planet| planet.coords_raw() == world_sector);
        if planet_idx
            .is_some_and(|idx| game_data.planets.records[idx].owner_empire_slot_raw() == owner)
        {
            return;
        }
        let in_orbit_there = planet_idx.is_some() && sector == world_sector && depth.round() == 1;
        let holds = moving && !in_orbit_there;
        // A base that lifts on the last tick drops to deep space at once;
        // its escort drops with it. Classic leaves the escort behind
        // (t040).
        let lifted_escort = if !holds && last_tick {
            self.find_escort(game_data, base_idx)
        } else {
            None
        };
        let base = &mut game_data.bases.records[base_idx];
        let response = if holds {
            let held = depth.round().clamp(0, 1);
            base.set_target_depth_raw(held as u8);
            base.set_depth(Real48::from_int(held));
            base.set_moving_flag_raw(0);
            base.set_target_coords_raw(sector);
            LostWorldResponse::Holding
        } else {
            base.set_target_depth_raw(0);
            base.set_moving_flag_raw(1);
            if last_tick {
                base.set_depth(Real48::ZERO);
            }
            LostWorldResponse::Leaving
        };
        if let Some(escort) = lifted_escort {
            self.carry_escort(game_data, escort, base_idx);
        }
        self.events.starbase_reports.push(report(
            game_data,
            StarbaseReport::LostWorld {
                planet_idx,
                system: target,
                response,
            },
        ));
    }
}

// A base lifts on the last tick only when its world changes hands during
// the year's ticks, which no Rust phase does, so the escort's drop is
// pinned here on the dispatcher itself (t040).
#[cfg(test)]
mod tests {
    use super::*;
    use nc_compat::load_classic_core_files;
    use std::path::Path;

    #[test]
    fn an_escort_drops_with_a_base_that_lifts_on_the_last_tick() {
        // Empire 1's base rests in orbit at a world empire 2 now holds,
        // escorted by fleet 1.
        let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/ecmaint-starbase-lift-pre/v1.5");
        let mut game_data = load_classic_core_files(&dir).expect("fixture loads");
        let mut movement = YearMovement::new(&game_data, &[]);

        movement.dispatch_base(&mut game_data, 0, TICKS_PER_YEAR);

        let base = &game_data.bases.records[0];
        let escort = &game_data.fleets.records[0];
        assert_eq!(base.depth(), Real48::ZERO);
        assert_eq!(base.target_depth_raw(), 0);
        assert_eq!(escort.depth(), Real48::ZERO);
        assert_eq!(escort.world_target_flag_raw(), 0);
    }
}
