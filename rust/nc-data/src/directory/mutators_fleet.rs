use std::collections::BTreeSet;

use super::*;

fn arms_on_station_hostile_order(order: Order) -> bool {
    matches!(
        order,
        Order::BombardWorld | Order::InvadeWorld | Order::BlitzWorld
    )
}

impl CoreGameData {
    pub fn conflicting_friendly_colonize_fleet_record(
        &self,
        owner_empire_raw: u8,
        target: [u8; 2],
        excluded_fleet_record_indexes_1_based: &BTreeSet<usize>,
    ) -> Option<usize> {
        if owner_empire_raw == 0 {
            return None;
        }
        self.fleets
            .records
            .iter()
            .enumerate()
            .find(|(idx, fleet)| {
                fleet.owner_empire_raw() == owner_empire_raw
                    && !excluded_fleet_record_indexes_1_based.contains(&(idx + 1))
                    && fleet.etac_count() > 0
                    && fleet.standing_order_kind() == Order::ColonizeWorld
                    && fleet.standing_order_target_coords_raw() == target
            })
            .map(|(idx, _)| idx + 1)
    }

    pub fn validate_friendly_colonize_target_available(
        &self,
        owner_empire_raw: u8,
        target: [u8; 2],
        excluded_fleet_record_indexes_1_based: &BTreeSet<usize>,
    ) -> Result<(), FleetOrderValidationError> {
        if let Some(conflicting_fleet_record_index_1_based) = self
            .conflicting_friendly_colonize_fleet_record(
                owner_empire_raw,
                target,
                excluded_fleet_record_indexes_1_based,
            )
        {
            return Err(FleetOrderValidationError::DuplicateFriendlyColonizeTarget {
                target,
                conflicting_fleet_record_index_1_based,
            });
        }
        Ok(())
    }

    pub fn set_fleet_local_slot(
        &mut self,
        player_index_1_based: usize,
        fleet_index_1_based: usize,
        local_slot: u16,
    ) -> Result<(), GameStateMutationError> {
        let owner_empire = player_index_1_based as u8;
        let fleet = self.fleets.records.get(fleet_index_1_based - 1).ok_or(
            GameStateMutationError::MissingFleetRecord {
                index_1_based: fleet_index_1_based,
            },
        )?;
        if fleet.owner_empire_raw() != owner_empire {
            return Err(GameStateMutationError::FleetOwnershipMismatch {
                player_index_1_based,
                fleet_index_1_based,
            });
        }
        if local_slot == 0
            || self.fleets.records.iter().enumerate().any(|(idx, other)| {
                idx + 1 != fleet_index_1_based
                    && other.owner_empire_raw() == owner_empire
                    && other.local_slot_word_raw() == local_slot
            })
        {
            return Err(GameStateMutationError::InvalidFleetLocalSlot {
                fleet_index_1_based,
                requested: local_slot,
            });
        }
        self.fleets.records[fleet_index_1_based - 1].set_local_slot_word_raw(local_slot);
        Ok(())
    }

    pub fn set_fleet_order(
        &mut self,
        record_index_1_based: usize,
        speed: u8,
        order_code: u8,
        target: [u8; 2],
        aux0: Option<u8>,
        aux1: Option<u8>,
    ) -> Result<[u8; 2], GameStateMutationError> {
        self.validate_fleet_order_payload(record_index_1_based, order_code, target, aux0, aux1)
            .map_err(|reason| GameStateMutationError::InvalidFleetOrder {
                fleet_index_1_based: record_index_1_based,
                reason,
            })?;
        let max_speed = self
            .fleets
            .records
            .get(record_index_1_based - 1)
            .ok_or(GameStateMutationError::MissingFleetRecord {
                index_1_based: record_index_1_based,
            })?
            .max_speed();
        if speed > max_speed {
            return Err(GameStateMutationError::InvalidFleetSpeed {
                fleet_index_1_based: record_index_1_based,
                requested: speed,
                max: max_speed,
            });
        }
        let record = self
            .fleets
            .records
            .get_mut(record_index_1_based - 1)
            .ok_or(GameStateMutationError::MissingFleetRecord {
                index_1_based: record_index_1_based,
            })?;
        reset_motion_state_for_new_orders(record);
        record.set_current_speed(speed);
        record.set_standing_order_code_raw(order_code);
        record.set_standing_order_target_coords_raw(target);
        // ECGAME writes 0x22 = world-target flag (0 for sector-targeted orders,
        // 1 otherwise) and 0x23 = mission id (starbase id for Guard Starbase,
        // host fleet id for Join); explicit aux bytes override byte-for-byte.
        let mut mission_aux = [u8::from(Order::from_raw(order_code).targets_world()), 0];
        if let Some(value) = aux0 {
            mission_aux[0] = value;
        }
        if let Some(value) = aux1 {
            mission_aux[1] = value;
        }
        record.set_mission_aux_bytes(mission_aux);
        if arms_on_station_hostile_order(Order::from_raw(order_code))
            && record.current_location_coords_raw() == target
        {
            record.set_current_speed(0);
            record.set_extended_tuple_c_payload_raw([0x80, 0xb9, 0xff, 0xff, 0xff, 0x7f]);
        }
        Ok(record.mission_aux_bytes())
    }

    pub fn validate_fleet_order_payload(
        &self,
        record_index_1_based: usize,
        order_code: u8,
        target: [u8; 2],
        aux0: Option<u8>,
        aux1: Option<u8>,
    ) -> Result<(), FleetOrderValidationError> {
        let Some(fleet) = self.fleets.records.get(record_index_1_based - 1) else {
            return Ok(());
        };
        let order = Order::from_raw(order_code);
        if matches!(order, Order::Unknown(_)) {
            return Err(FleetOrderValidationError::UnknownOrderCode(order_code));
        }
        let owner = fleet.owner_empire_raw();
        let has_combat = fleet.has_any_combat_ships();
        let has_scout = fleet.scout_count() > 0;
        let has_etac = fleet.etac_count() > 0;
        let has_loaded_troops =
            fleet.troop_transport_count() > 0 && u16::from(fleet.army_count()) > 0;
        let planet_owner = self
            .planets
            .records
            .iter()
            .find(|planet| planet.coords_raw() == target)
            .map(|planet| planet.owner_empire_slot_raw());

        match order {
            Order::GuardStarbase => {
                if !has_combat {
                    return Err(FleetOrderValidationError::MissingCombatShips);
                }
                let _ = aux0;
                let base_id = aux1.unwrap_or_else(|| fleet.guard_starbase_index_raw());
                if base_id == 0
                    || !self.bases.records.iter().any(|base| {
                        base.owner_empire_raw() == owner && base.base_id_raw() == base_id
                    })
                {
                    return Err(FleetOrderValidationError::InvalidGuardStarbase);
                }
            }
            Order::GuardBlockadeWorld => {
                if !has_combat {
                    return Err(FleetOrderValidationError::MissingCombatShips);
                }
                if planet_owner.is_none() {
                    return Err(FleetOrderValidationError::MissingPlanetTarget);
                }
            }
            Order::BombardWorld => {
                if !has_combat {
                    return Err(FleetOrderValidationError::MissingCombatShips);
                }
                match planet_owner {
                    None => return Err(FleetOrderValidationError::MissingPlanetTarget),
                    Some(owner_empire) if owner_empire == owner => {
                        return Err(FleetOrderValidationError::TargetOwnedByFleetEmpire);
                    }
                    _ => {}
                }
            }
            Order::InvadeWorld => {
                if !has_combat {
                    return Err(FleetOrderValidationError::MissingCombatShips);
                }
                if !has_loaded_troops {
                    return Err(FleetOrderValidationError::MissingLoadedTroopTransports);
                }
                match planet_owner {
                    None => return Err(FleetOrderValidationError::MissingPlanetTarget),
                    Some(owner_empire) if owner_empire == owner => {
                        return Err(FleetOrderValidationError::TargetOwnedByFleetEmpire);
                    }
                    _ => {}
                }
            }
            Order::BlitzWorld => {
                if !has_loaded_troops {
                    return Err(FleetOrderValidationError::MissingLoadedTroopTransports);
                }
                match planet_owner {
                    None => return Err(FleetOrderValidationError::MissingPlanetTarget),
                    Some(owner_empire) if owner_empire == owner => {
                        return Err(FleetOrderValidationError::TargetOwnedByFleetEmpire);
                    }
                    _ => {}
                }
            }
            Order::ViewWorld | Order::ScoutSolarSystem | Order::Salvage => {
                if matches!(order, Order::ScoutSolarSystem) && !has_scout {
                    return Err(FleetOrderValidationError::MissingScoutShip);
                }
                let Some(owner_empire) = planet_owner else {
                    return Err(FleetOrderValidationError::MissingPlanetTarget);
                };
                if matches!(order, Order::Salvage) && owner_empire != owner {
                    return Err(FleetOrderValidationError::TargetNotOwnedByFleetEmpire);
                }
            }
            Order::ScoutSector => {
                if !has_scout {
                    return Err(FleetOrderValidationError::MissingScoutShip);
                }
            }
            Order::ColonizeWorld => {
                if !has_etac {
                    return Err(FleetOrderValidationError::MissingEtac);
                }
                if planet_owner.is_none() {
                    return Err(FleetOrderValidationError::MissingPlanetTarget);
                }
                let mut excluded = BTreeSet::new();
                excluded.insert(record_index_1_based);
                self.validate_friendly_colonize_target_available(owner, target, &excluded)?;
            }
            Order::JoinAnotherFleet => {
                let _ = aux0;
                let host_id = aux1.unwrap_or_else(|| fleet.join_host_fleet_id_raw());
                if host_id == 0
                    || host_id == fleet.fleet_id()
                    || !self
                        .fleets
                        .records
                        .iter()
                        .any(|host| host.fleet_id() == host_id && host.owner_empire_raw() == owner)
                {
                    return Err(FleetOrderValidationError::InvalidJoinHost);
                }
            }
            _ => {}
        }
        Ok(())
    }

    pub fn validate_fleet_player_inputs(
        &self,
        record_index_1_based: usize,
        order_code: u8,
        target: [u8; 2],
        aux0: Option<u8>,
        aux1: Option<u8>,
    ) -> Result<(), FleetPlayerInputValidationError> {
        let Some(fleet) = self.fleets.records.get(record_index_1_based - 1) else {
            return Ok(());
        };
        if fleet.army_count() > fleet.troop_transport_count() {
            return Err(
                FleetPlayerInputValidationError::LoadedArmiesExceedTransportCapacity {
                    loaded_armies: fleet.army_count(),
                    transports: fleet.troop_transport_count(),
                },
            );
        }
        if fleet.current_speed() > fleet.max_speed() {
            return Err(FleetPlayerInputValidationError::SpeedExceedsMaximum {
                speed: fleet.current_speed(),
                max: fleet.max_speed(),
            });
        }
        if fleet.is_support_only() && fleet.rules_of_engagement() != 0 {
            return Err(
                FleetPlayerInputValidationError::NonCombatFleetMustUseZeroRoe {
                    roe: fleet.rules_of_engagement(),
                },
            );
        }
        if fleet.rules_of_engagement() > 10 {
            return Err(
                FleetPlayerInputValidationError::RulesOfEngagementOutOfRange {
                    roe: fleet.rules_of_engagement(),
                },
            );
        }
        self.validate_fleet_order_payload(record_index_1_based, order_code, target, aux0, aux1)
            .map_err(FleetPlayerInputValidationError::InvalidOrder)
    }

    pub fn set_join_fleet_order(
        &mut self,
        player_index_1_based: usize,
        fleet_index_1_based: usize,
        host_fleet_index_1_based: usize,
    ) -> Result<(), GameStateMutationError> {
        if fleet_index_1_based == host_fleet_index_1_based {
            return Err(GameStateMutationError::InvalidFleetMergeSelection {
                fleet_index_1_based,
                host_fleet_index_1_based,
            });
        }

        let owner_empire = player_index_1_based as u8;
        let host = self
            .fleets
            .records
            .get(host_fleet_index_1_based - 1)
            .ok_or(GameStateMutationError::MissingFleetRecord {
                index_1_based: host_fleet_index_1_based,
            })?;
        let fleet = self.fleets.records.get(fleet_index_1_based - 1).ok_or(
            GameStateMutationError::MissingFleetRecord {
                index_1_based: fleet_index_1_based,
            },
        )?;

        if fleet.owner_empire_raw() != owner_empire {
            return Err(GameStateMutationError::FleetOwnershipMismatch {
                player_index_1_based,
                fleet_index_1_based,
            });
        }
        if host.owner_empire_raw() != owner_empire {
            return Err(GameStateMutationError::FleetOwnershipMismatch {
                player_index_1_based,
                fleet_index_1_based: host_fleet_index_1_based,
            });
        }

        let host_coords = host.current_location_coords_raw();
        let host_fleet_id = host.fleet_id();
        let fleet = self.fleets.records.get_mut(fleet_index_1_based - 1).ok_or(
            GameStateMutationError::MissingFleetRecord {
                index_1_based: fleet_index_1_based,
            },
        )?;
        reset_motion_state_for_new_orders(fleet);
        fleet.set_current_speed(fleet.max_speed());
        fleet.set_standing_order_kind(Order::JoinAnotherFleet);
        fleet.set_standing_order_target_coords_raw(host_coords);
        fleet.set_mission_aux_bytes([1, host_fleet_id]);
        Ok(())
    }

    pub fn set_fleet_rules_of_engagement(
        &mut self,
        player_index_1_based: usize,
        fleet_index_1_based: usize,
        roe: u8,
    ) -> Result<(), GameStateMutationError> {
        let owner_empire = player_index_1_based as u8;
        let fleet = self.fleets.records.get(fleet_index_1_based - 1).ok_or(
            GameStateMutationError::MissingFleetRecord {
                index_1_based: fleet_index_1_based,
            },
        )?;
        if fleet.owner_empire_raw() != owner_empire {
            return Err(GameStateMutationError::FleetOwnershipMismatch {
                player_index_1_based,
                fleet_index_1_based,
            });
        }
        if roe > 10 {
            return Err(GameStateMutationError::InvalidFleetPlayerInput {
                fleet_index_1_based,
                reason: FleetPlayerInputValidationError::RulesOfEngagementOutOfRange { roe },
            });
        }
        if fleet.is_support_only() && roe != 0 {
            return Err(GameStateMutationError::InvalidFleetPlayerInput {
                fleet_index_1_based,
                reason: FleetPlayerInputValidationError::NonCombatFleetMustUseZeroRoe { roe },
            });
        }

        self.fleets.records[fleet_index_1_based - 1].set_rules_of_engagement(roe);
        Ok(())
    }
}
