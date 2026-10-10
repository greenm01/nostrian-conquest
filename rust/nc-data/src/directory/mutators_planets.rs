use super::*;

impl CoreGameData {
    pub fn validate_planet_player_inputs(
        &self,
        planet_index_1_based: usize,
    ) -> Result<(), PlanetPlayerInputValidationError> {
        let Some(planet) = self.planets.records.get(planet_index_1_based - 1) else {
            return Ok(());
        };
        for slot in 0..10 {
            let build_count = planet.build_count_raw(slot);
            let build_kind = planet.build_kind_raw(slot);
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
            let stardock_count = planet.stardock_count_raw(slot);
            let stardock_kind = planet.stardock_kind_raw(slot);
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
        let record = self
            .planets
            .records
            .get_mut(record_index_1_based - 1)
            .ok_or(GameStateMutationError::MissingPlanetRecord {
                index_1_based: record_index_1_based,
            })?;
        record.set_build_count_raw(0, slot_raw);
        record.set_build_kind_raw(0, kind_raw);
        Ok(())
    }

    pub fn clear_planet_build_queue(
        &mut self,
        record_index_1_based: usize,
    ) -> Result<(), GameStateMutationError> {
        let record = self
            .planets
            .records
            .get_mut(record_index_1_based - 1)
            .ok_or(GameStateMutationError::MissingPlanetRecord {
                index_1_based: record_index_1_based,
            })?;
        for slot in 0..10 {
            record.set_build_count_raw(slot, 0);
            record.set_build_kind_raw(slot, 0);
        }
        Ok(())
    }

    pub fn scorch_planet_surface(
        &mut self,
        record_index_1_based: usize,
    ) -> Result<(), GameStateMutationError> {
        let record = self
            .planets
            .records
            .get_mut(record_index_1_based - 1)
            .ok_or(GameStateMutationError::MissingPlanetRecord {
                index_1_based: record_index_1_based,
            })?;

        record.set_potential_production_raw([0, 0]);
        let _ = record.set_present_production_points(0);
        record.set_stored_production_points(0);

        for slot in 0..10 {
            record.set_build_count_raw(slot, 0);
            record.set_build_kind_raw(slot, 0);
        }
        for slot in 0..crate::STARDOCK_SLOT_COUNT {
            record.set_stardock_count_raw(slot, 0);
            record.set_stardock_kind_raw(slot, 0);
        }

        Ok(())
    }

    pub fn clear_planet_build_orders_by_kind(
        &mut self,
        record_index_1_based: usize,
        kind: ProductionItemKind,
    ) -> Result<usize, GameStateMutationError> {
        let record = self
            .planets
            .records
            .get_mut(record_index_1_based - 1)
            .ok_or(GameStateMutationError::MissingPlanetRecord {
                index_1_based: record_index_1_based,
            })?;
        let mut cleared = 0usize;
        for slot in 0..10 {
            if ProductionItemKind::from_raw(record.build_kind_raw(slot)) == kind {
                record.set_build_count_raw(slot, 0);
                record.set_build_kind_raw(slot, 0);
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
        let record = self
            .planets
            .records
            .get_mut(record_index_1_based - 1)
            .ok_or(GameStateMutationError::MissingPlanetRecord {
                index_1_based: record_index_1_based,
            })?;
        let mut remaining = points_to_remove;
        let mut removed = 0u32;
        for slot in (0..10).rev() {
            if remaining == 0 {
                break;
            }
            if ProductionItemKind::from_raw(record.build_kind_raw(slot)) != kind {
                continue;
            }
            let slot_points = u32::from(record.build_count_raw(slot));
            if slot_points == 0 {
                continue;
            }
            let remove_here = slot_points.min(remaining);
            let left = slot_points - remove_here;
            record.set_build_count_raw(slot, left as u8);
            if left == 0 {
                record.set_build_kind_raw(slot, 0);
            }
            removed += remove_here;
            remaining -= remove_here;
        }
        Ok(removed)
    }

    pub fn planet_free_stardock_slots(
        &self,
        record_index_1_based: usize,
    ) -> Result<usize, GameStateMutationError> {
        let record = self.planets.records.get(record_index_1_based - 1).ok_or(
            GameStateMutationError::MissingPlanetRecord {
                index_1_based: record_index_1_based,
            },
        )?;

        let occupied_stardock = (0..crate::STARDOCK_SLOT_COUNT)
            .filter(|&s| record.stardock_kind_raw(s) != 0)
            .count();

        let pending_ship_slots = (0..10)
            .filter(|&s| {
                let count = record.build_count_raw(s);
                let kind = record.build_kind_raw(s);
                if count == 0 || kind == 0 {
                    return false;
                }
                !matches!(
                    ProductionItemKind::from_raw(kind),
                    ProductionItemKind::Army | ProductionItemKind::GroundBattery
                )
            })
            .count();

        let reserved = occupied_stardock + pending_ship_slots;
        Ok(crate::STARDOCK_SLOT_COUNT.saturating_sub(reserved))
    }

    pub fn planet_open_stardock_slots_now(
        &self,
        record_index_1_based: usize,
    ) -> Result<usize, GameStateMutationError> {
        let record = self.planets.records.get(record_index_1_based - 1).ok_or(
            GameStateMutationError::MissingPlanetRecord {
                index_1_based: record_index_1_based,
            },
        )?;

        Ok((0..crate::STARDOCK_SLOT_COUNT)
            .filter(|&s| record.stardock_kind_raw(s) == 0)
            .count())
    }

    pub fn planet_free_army_capacity(
        &self,
        record_index_1_based: usize,
    ) -> Result<u16, GameStateMutationError> {
        let record = self.planets.records.get(record_index_1_based - 1).ok_or(
            GameStateMutationError::MissingPlanetRecord {
                index_1_based: record_index_1_based,
            },
        )?;
        Ok(u16::from(u8::MAX.saturating_sub(record.army_count_raw())))
    }

    pub fn planet_free_ground_battery_capacity(
        &self,
        record_index_1_based: usize,
    ) -> Result<u16, GameStateMutationError> {
        let record = self.planets.records.get(record_index_1_based - 1).ok_or(
            GameStateMutationError::MissingPlanetRecord {
                index_1_based: record_index_1_based,
            },
        )?;
        Ok(u16::from(
            u8::MAX.saturating_sub(record.ground_batteries_raw()),
        ))
    }

    pub fn planet_additional_build_points_capacity(
        &self,
        record_index_1_based: usize,
        kind: ProductionItemKind,
    ) -> Result<u32, GameStateMutationError> {
        let record = self.planets.records.get(record_index_1_based - 1).ok_or(
            GameStateMutationError::MissingPlanetRecord {
                index_1_based: record_index_1_based,
            },
        )?;
        let slot_capacity = build_queue_slot_capacity(kind).unwrap_or(u32::from(u8::MAX));
        let existing_capacity = (0..10)
            .filter_map(|slot| {
                if ProductionItemKind::from_raw(record.build_kind_raw(slot)) != kind {
                    return None;
                }
                let current = u32::from(record.build_count_raw(slot));
                if current == 0 {
                    return None;
                }
                Some(slot_capacity.saturating_sub(current))
            })
            .sum::<u32>();
        let empty_slots = (0..10)
            .filter(|&slot| record.build_count_raw(slot) == 0 && record.build_kind_raw(slot) == 0)
            .count() as u32;
        let new_slot_capacity = if kind.requires_stardock() {
            empty_slots.min(self.planet_free_stardock_slots(record_index_1_based)? as u32)
        } else {
            empty_slots
        };

        Ok(existing_capacity.saturating_add(new_slot_capacity.saturating_mul(slot_capacity)))
    }

    /// The planet's build budget for this turn: `min(treasury, build_capacity)`.
    pub fn planet_build_budget(
        &self,
        record_index_1_based: usize,
    ) -> Result<u32, GameStateMutationError> {
        let record = self.planets.records.get(record_index_1_based - 1).ok_or(
            GameStateMutationError::MissingPlanetRecord {
                index_1_based: record_index_1_based,
            },
        )?;
        let capacity = crate::build_capacity(
            record.present_production_points().unwrap_or(0),
            self.planet_has_friendly_starbase(record.owner_empire_slot_raw(), record.coords_raw()),
        );
        Ok(u32::from(capacity).min(record.stored_production_points()))
    }

    /// Budget still free for new orders this turn: the build budget less the
    /// points already queued (the build screen's BUDGET).
    pub fn planet_build_points_left(
        &self,
        record_index_1_based: usize,
    ) -> Result<u32, GameStateMutationError> {
        let budget = self.planet_build_budget(record_index_1_based)?;
        let record = &self.planets.records[record_index_1_based - 1];
        let committed = (0..10)
            .map(|slot| u32::from(record.build_count_raw(slot)))
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
        let record = self
            .planets
            .records
            .get_mut(record_index_1_based - 1)
            .ok_or(GameStateMutationError::MissingPlanetRecord {
                index_1_based: record_index_1_based,
            })?;

        let mut remaining = points_remaining_raw;
        for slot in 0..10 {
            if remaining == 0 || ProductionItemKind::from_raw(record.build_kind_raw(slot)) != kind {
                continue;
            }
            let current = u32::from(record.build_count_raw(slot));
            if current == 0 {
                continue;
            }
            let add_here = remaining.min(slot_capacity.saturating_sub(current));
            record.set_build_count_raw(slot, (current + add_here) as u8);
            remaining -= add_here;
        }

        for slot in 0..10 {
            if remaining == 0 {
                break;
            }
            if record.build_count_raw(slot) != 0 || record.build_kind_raw(slot) != 0 {
                continue;
            }
            let add_here = remaining.min(slot_capacity);
            record.set_build_count_raw(slot, add_here as u8);
            record.set_build_kind_raw(slot, kind_raw);
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
