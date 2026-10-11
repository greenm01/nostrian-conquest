use super::*;
use crate::{
    ProductionItemKind, Real48, present_production_points, stardock_slot_kind,
    stored_production_points,
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
            let build_kind = planet.build_queue[slot].kind.to_raw();
            if build_count == 0 && build_kind != 0 {
                return Err(PlanetPlayerInputValidationError::MissingBuildCountForKind);
            }
            if build_count != 0 && build_kind == 0 {
                return Err(PlanetPlayerInputValidationError::MissingBuildKindForCount);
            }
            if build_kind != 0
                && matches!(
                    ProductionItemKind::from_raw(build_kind),
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
            let stardock_kind = stardock_slot_kind(planet, slot).to_raw();
            if stardock_count == 0 && stardock_kind != 0 {
                return Err(PlanetPlayerInputValidationError::MissingStardockCountForKind);
            }
            if stardock_count != 0 && stardock_kind == 0 {
                return Err(PlanetPlayerInputValidationError::MissingStardockKindForCount);
            }
            if stardock_kind != 0
                && matches!(
                    ProductionItemKind::from_raw(stardock_kind),
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
    pub fn set_planet_build(
        &mut self,
        record_index_1_based: usize,
        slot_raw: u8,
        kind_raw: u8,
    ) -> Result<(), GameStateMutationError> {
        if kind_raw == 0 {
            return Err(GameStateMutationError::InvalidPlanetPlayerInput {
                planet_index_1_based: record_index_1_based,
                reason: PlanetPlayerInputValidationError::MissingBuildKindForCount,
            });
        }
        if matches!(
            ProductionItemKind::from_raw(kind_raw),
            ProductionItemKind::Unknown(_)
        ) {
            return Err(GameStateMutationError::InvalidPlanetPlayerInput {
                planet_index_1_based: record_index_1_based,
                reason: PlanetPlayerInputValidationError::InvalidBuildKind(kind_raw),
            });
        }
        let record = self.planets.get_mut(record_index_1_based - 1).ok_or(
            GameStateMutationError::MissingPlanetRecord {
                index_1_based: record_index_1_based,
            },
        )?;
        record.build_queue[0].points_owed = slot_raw;
        record.build_queue[0].kind = ProductionItemKind::from_raw(kind_raw);
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

    pub fn scorch_planet_surface(
        &mut self,
        record_index_1_based: usize,
    ) -> Result<(), GameStateMutationError> {
        let record = self.planets.get_mut(record_index_1_based - 1).ok_or(
            GameStateMutationError::MissingPlanetRecord {
                index_1_based: record_index_1_based,
            },
        )?;

        record.potential_production = 0;
        record.production = Real48::ZERO;
        record.stored_goods = Real48::ZERO;

        for slot in 0..10 {
            record.build_queue[slot].points_owed = 0;
            record.build_queue[slot].kind = ProductionItemKind::Unknown(0);
        }
        for slot in 0..crate::STARDOCK_SLOT_COUNT {
            record.stardock[slot] = 0;
        }

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
            let slot_points = u32::from(record.build_queue[slot].points_owed);
            if slot_points == 0 {
                continue;
            }
            let remove_here = slot_points.min(remaining);
            let left = slot_points - remove_here;
            record.build_queue[slot].points_owed = left as u8;
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

    pub fn planet_additional_build_points_capacity(
        &self,
        record_index_1_based: usize,
        kind: ProductionItemKind,
    ) -> Result<u32, GameStateMutationError> {
        let record = self.planets.get(record_index_1_based - 1).ok_or(
            GameStateMutationError::MissingPlanetRecord {
                index_1_based: record_index_1_based,
            },
        )?;
        let slot_capacity = build_queue_slot_capacity(kind).unwrap_or(u32::from(u8::MAX));
        let existing_capacity = (0..10)
            .filter_map(|slot| {
                if record.build_queue[slot].kind != kind {
                    return None;
                }
                let current = u32::from(record.build_queue[slot].points_owed);
                if current == 0 {
                    return None;
                }
                Some(slot_capacity.saturating_sub(current))
            })
            .sum::<u32>();
        let empty_slots = (0..10)
            .filter(|&slot| {
                record.build_queue[slot].points_owed == 0
                    && record.build_queue[slot].kind.to_raw() == 0
            })
            .count() as u32;
        Ok(existing_capacity.saturating_add(empty_slots.saturating_mul(slot_capacity)))
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
            .map(|slot| u32::from(record.build_queue[slot].points_owed))
            .sum::<u32>();
        Ok(budget.saturating_sub(committed))
    }

    pub fn append_planet_build_order(
        &mut self,
        record_index_1_based: usize,
        points_remaining_raw: u32,
        kind_raw: u8,
    ) -> Result<(), GameStateMutationError> {
        if points_remaining_raw == 0 || kind_raw == 0 {
            return Err(GameStateMutationError::InvalidPlanetPlayerInput {
                planet_index_1_based: record_index_1_based,
                reason: if kind_raw == 0 {
                    PlanetPlayerInputValidationError::MissingBuildKindForCount
                } else {
                    PlanetPlayerInputValidationError::MissingBuildCountForKind
                },
            });
        }
        let kind = ProductionItemKind::from_raw(kind_raw);
        if matches!(kind, ProductionItemKind::Unknown(_)) {
            return Err(GameStateMutationError::InvalidPlanetPlayerInput {
                planet_index_1_based: record_index_1_based,
                reason: PlanetPlayerInputValidationError::InvalidBuildKind(kind_raw),
            });
        }
        if let Some(cost) = kind.build_cost() {
            if points_remaining_raw % cost != 0 {
                return Err(GameStateMutationError::InvalidPlanetPlayerInput {
                    planet_index_1_based: record_index_1_based,
                    reason: PlanetPlayerInputValidationError::InvalidBuildPointsForKind {
                        kind_raw,
                        points_remaining_raw,
                    },
                });
            }
        }
        let total_capacity =
            self.planet_additional_build_points_capacity(record_index_1_based, kind)?;
        if points_remaining_raw > total_capacity {
            return Err(GameStateMutationError::PlanetBuildQueueFull {
                index_1_based: record_index_1_based,
            });
        }

        let slot_capacity = build_queue_slot_capacity(kind).unwrap_or(u32::from(u8::MAX));
        let record = self.planets.get_mut(record_index_1_based - 1).ok_or(
            GameStateMutationError::MissingPlanetRecord {
                index_1_based: record_index_1_based,
            },
        )?;

        let mut remaining = points_remaining_raw;
        for slot in 0..10 {
            if remaining == 0 || record.build_queue[slot].kind != kind {
                continue;
            }
            let current = u32::from(record.build_queue[slot].points_owed);
            if current == 0 {
                continue;
            }
            let add_here = remaining.min(slot_capacity.saturating_sub(current));
            record.build_queue[slot].points_owed = (current + add_here) as u8;
            remaining -= add_here;
        }

        for slot in 0..10 {
            if remaining == 0 {
                break;
            }
            if record.build_queue[slot].points_owed != 0
                || record.build_queue[slot].kind.to_raw() != 0
            {
                continue;
            }
            let add_here = remaining.min(slot_capacity);
            record.build_queue[slot].points_owed = add_here as u8;
            record.build_queue[slot].kind = ProductionItemKind::from_raw(kind_raw);
            remaining -= add_here;
        }
        Ok(())
    }
    pub fn replace_planet_build_queue_with_single_order(
        &mut self,
        record_index_1_based: usize,
        points_remaining_raw: u8,
        kind_raw: u8,
    ) -> Result<(), GameStateMutationError> {
        self.clear_planet_build_queue(record_index_1_based)?;
        self.set_planet_build(record_index_1_based, points_remaining_raw, kind_raw)
    }
}

fn build_queue_slot_capacity(kind: ProductionItemKind) -> Option<u32> {
    let cost = kind.build_cost()?;
    Some(if kind == ProductionItemKind::Starbase {
        cost
    } else if kind.requires_stardock() {
        (u32::from(u8::MAX) / cost) * cost
    } else {
        u32::from(u8::MAX)
    })
}
