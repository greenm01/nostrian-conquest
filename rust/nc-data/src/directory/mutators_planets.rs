use super::*;
use crate::{
    ProductionItemKind, present_production_points, stardock_slot_kind, stored_production_points,
};

impl CoreGameData {
    pub fn validate_planet_player_inputs(
        &self,
        planet_index_1_based: usize,
    ) -> Result<(), PlanetPlayerInputValidationError> {
        let Some(planet) = self.planets.get(planet_index_1_based - 1) else {
            return Ok(());
        };
        for slot in 0..10 {
            let build_count = planet.build_queue[slot].points_owed;
            let build_kind = planet.build_queue[slot].kind.code();
            if build_count == 0 && build_kind != 0 {
                return Err(PlanetPlayerInputValidationError::MissingBuildCountForKind);
            }
            if build_count != 0 && build_kind == 0 {
                return Err(PlanetPlayerInputValidationError::MissingBuildKindForCount);
            }
            if build_kind != 0
                && matches!(
                    ProductionItemKind::from_code(build_kind),
                    ProductionItemKind::Unknown(_)
                )
            {
                return Err(PlanetPlayerInputValidationError::InvalidBuildKind(
                    build_kind,
                ));
            }
        }
        for slot in 0..crate::STARDOCK_SLOT_COUNT {
            let stardock_count = u16::try_from(planet.stardock[slot]).unwrap_or(u16::MAX);
            let stardock_kind = stardock_slot_kind(planet, slot).code();
            if stardock_count == 0 && stardock_kind != 0 {
                return Err(PlanetPlayerInputValidationError::MissingStardockCountForKind);
            }
            if stardock_count != 0 && stardock_kind == 0 {
                return Err(PlanetPlayerInputValidationError::MissingStardockKindForCount);
            }
            if stardock_kind != 0
                && matches!(
                    ProductionItemKind::from_code(stardock_kind),
                    ProductionItemKind::Unknown(_)
                )
            {
                return Err(PlanetPlayerInputValidationError::InvalidStardockKind(
                    stardock_kind,
                ));
            }
        }
        Ok(())
    }
    pub fn clear_planet_build_queue(
        &mut self,
        record_index_1_based: usize,
    ) -> Result<(), GameStateMutationError> {
        let record = self.planets.get_mut(record_index_1_based - 1).ok_or(
            GameStateMutationError::MissingPlanetRecord {
                index_1_based: record_index_1_based,
            },
        )?;
        for slot in 0..10 {
            record.build_queue[slot].points_owed = 0;
            record.build_queue[slot].kind = ProductionItemKind::Unknown(0);
        }
        Ok(())
    }

    /// Set or rescind the planet's Scorch-Earth order. The order changes
    /// nothing at once: each yearly pass while it is set decays the world.
    pub fn set_planet_scorch_earth(
        &mut self,
        record_index_1_based: usize,
        scorched: bool,
    ) -> Result<(), GameStateMutationError> {
        let record = self.planets.get_mut(record_index_1_based - 1).ok_or(
            GameStateMutationError::MissingPlanetRecord {
                index_1_based: record_index_1_based,
            },
        )?;
        record.scorch_earth = scorched;
        Ok(())
    }

    pub fn clear_planet_build_orders_by_kind(
        &mut self,
        record_index_1_based: usize,
        kind: ProductionItemKind,
    ) -> Result<usize, GameStateMutationError> {
        let record = self.planets.get_mut(record_index_1_based - 1).ok_or(
            GameStateMutationError::MissingPlanetRecord {
                index_1_based: record_index_1_based,
            },
        )?;
        let mut cleared = 0usize;
        for slot in 0..10 {
            if record.build_queue[slot].kind == kind {
                record.build_queue[slot].points_owed = 0;
                record.build_queue[slot].kind = ProductionItemKind::Unknown(0);
                cleared += 1;
            }
        }
        Ok(cleared)
    }

    pub fn remove_planet_build_points_by_kind(
        &mut self,
        record_index_1_based: usize,
        kind: ProductionItemKind,
        points_to_remove: u32,
    ) -> Result<u32, GameStateMutationError> {
        let record = self.planets.get_mut(record_index_1_based - 1).ok_or(
            GameStateMutationError::MissingPlanetRecord {
                index_1_based: record_index_1_based,
            },
        )?;
        let mut remaining = points_to_remove;
        let mut removed = 0u32;
        for slot in (0..10).rev() {
            if remaining == 0 {
                break;
            }
            if record.build_queue[slot].kind != kind {
                continue;
            }
            let slot_points = record.build_queue[slot].points_owed;
            if slot_points == 0 {
                continue;
            }
            let remove_here = slot_points.min(remaining);
            let left = slot_points - remove_here;
            record.build_queue[slot].points_owed = left;
            if left == 0 {
                record.build_queue[slot].kind = ProductionItemKind::Unknown(0);
            }
            removed += remove_here;
            remaining -= remove_here;
        }
        Ok(removed)
    }

    pub fn planet_free_army_capacity(
        &self,
        record_index_1_based: usize,
    ) -> Result<u16, GameStateMutationError> {
        let record = self.planets.get(record_index_1_based - 1).ok_or(
            GameStateMutationError::MissingPlanetRecord {
                index_1_based: record_index_1_based,
            },
        )?;
        Ok(u16::from(u8::MAX.saturating_sub(
            u8::try_from(record.armies).unwrap_or(u8::MAX),
        )))
    }

    pub fn planet_free_ground_battery_capacity(
        &self,
        record_index_1_based: usize,
    ) -> Result<u16, GameStateMutationError> {
        let record = self.planets.get(record_index_1_based - 1).ok_or(
            GameStateMutationError::MissingPlanetRecord {
                index_1_based: record_index_1_based,
            },
        )?;
        Ok(u16::from(u8::MAX.saturating_sub(
            u8::try_from(record.ground_batteries).unwrap_or(u8::MAX),
        )))
    }

    /// The planet's build budget for this turn: `min(treasury, build_capacity)`.
    pub fn planet_build_budget(
        &self,
        record_index_1_based: usize,
    ) -> Result<u32, GameStateMutationError> {
        let record = self.planets.get(record_index_1_based - 1).ok_or(
            GameStateMutationError::MissingPlanetRecord {
                index_1_based: record_index_1_based,
            },
        )?;
        let capacity = crate::build_capacity(
            present_production_points(record),
            self.planet_has_friendly_starbase(record.owner.0, record.sector.to_array()),
        );
        Ok(u32::from(capacity).min(stored_production_points(record)))
    }

    /// Budget still free for new orders this turn: the build budget less the
    /// points already queued (the build screen's BUDGET).
    pub fn planet_build_points_left(
        &self,
        record_index_1_based: usize,
    ) -> Result<u32, GameStateMutationError> {
        let budget = self.planet_build_budget(record_index_1_based)?;
        let record = &self.planets[record_index_1_based - 1];
        let committed = (0..10)
            .map(|slot| record.build_queue[slot].points_owed)
            .sum::<u32>();
        Ok(budget.saturating_sub(committed))
    }

    pub fn append_planet_build_order(
        &mut self,
        record_index_1_based: usize,
        points_remaining: u32,
        kind_code: u8,
    ) -> Result<(), GameStateMutationError> {
        if points_remaining == 0 || kind_code == 0 {
            return Err(GameStateMutationError::InvalidPlanetPlayerInput {
                planet_index_1_based: record_index_1_based,
                reason: if kind_code == 0 {
                    PlanetPlayerInputValidationError::MissingBuildKindForCount
                } else {
                    PlanetPlayerInputValidationError::MissingBuildCountForKind
                },
            });
        }
        let kind = ProductionItemKind::from_code(kind_code);
        if matches!(kind, ProductionItemKind::Unknown(_)) {
            return Err(GameStateMutationError::InvalidPlanetPlayerInput {
                planet_index_1_based: record_index_1_based,
                reason: PlanetPlayerInputValidationError::InvalidBuildKind(kind_code),
            });
        }
        if let Some(cost) = kind.build_cost() {
            if points_remaining % cost != 0 {
                return Err(GameStateMutationError::InvalidPlanetPlayerInput {
                    planet_index_1_based: record_index_1_based,
                    reason: PlanetPlayerInputValidationError::InvalidBuildPointsForKind {
                        kind: kind_code,
                        points_remaining,
                    },
                });
            }
        }
        let record = self.planets.get_mut(record_index_1_based - 1).ok_or(
            GameStateMutationError::MissingPlanetRecord {
                index_1_based: record_index_1_based,
            },
        )?;

        // Orders of one kind share a slot; the budget is the only limit.
        let slot = record
            .build_queue
            .iter()
            .position(|slot| slot.points_owed != 0 && slot.kind == kind)
            .or_else(|| {
                record
                    .build_queue
                    .iter()
                    .position(|slot| slot.points_owed == 0 && slot.kind.code() == 0)
            })
            .ok_or(GameStateMutationError::PlanetBuildQueueFull {
                index_1_based: record_index_1_based,
            })?;
        record.build_queue[slot].points_owed = record.build_queue[slot]
            .points_owed
            .saturating_add(points_remaining);
        record.build_queue[slot].kind = kind;
        Ok(())
    }

    /// Order `quantity` units of `kind`: `quantity * cost` points owed.
    pub fn append_planet_build_units(
        &mut self,
        record_index_1_based: usize,
        kind: ProductionItemKind,
        quantity: u32,
    ) -> Result<(), GameStateMutationError> {
        let Some(cost) = kind.build_cost() else {
            return Err(GameStateMutationError::InvalidPlanetPlayerInput {
                planet_index_1_based: record_index_1_based,
                reason: PlanetPlayerInputValidationError::InvalidBuildKind(kind.code()),
            });
        };
        self.append_planet_build_order(
            record_index_1_based,
            quantity.saturating_mul(cost),
            kind.code(),
        )
    }
}
