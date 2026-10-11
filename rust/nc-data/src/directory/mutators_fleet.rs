use std::collections::BTreeSet;

use super::*;
use crate::{
    Coords, Order, has_combat_ships, is_support_only, recompute_max_speed, set_mission_target,
};

impl CoreGameData {
    pub fn conflicting_friendly_colonize_fleet_record(
        &self,
        owner_empire: u8,
        target: [u8; 2],
        excluded_fleet_record_indexes_1_based: &BTreeSet<usize>,
    ) -> Option<usize> {
        if owner_empire == 0 {
            return None;
        }
        self.fleets
            .iter()
            .enumerate()
            .find(|(idx, fleet)| {
                fleet.owner.0 == owner_empire
                    && !excluded_fleet_record_indexes_1_based.contains(&(idx + 1))
                    && fleet.ships.etacs > 0
                    && fleet.order == Order::ColonizeWorld
                    && fleet.target.to_array() == target
            })
            .map(|(idx, _)| idx + 1)
    }

    pub fn validate_friendly_colonize_target_available(
        &self,
        owner_empire: u8,
        target: [u8; 2],
        excluded_fleet_record_indexes_1_based: &BTreeSet<usize>,
    ) -> Result<(), FleetOrderValidationError> {
        if let Some(conflicting_fleet_record_index_1_based) = self
            .conflicting_friendly_colonize_fleet_record(
                owner_empire,
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
        let fleet = self.fleets.get(fleet_index_1_based - 1).ok_or(
            GameStateMutationError::MissingFleetRecord {
                index_1_based: fleet_index_1_based,
            },
        )?;
        if fleet.owner.0 != owner_empire {
            return Err(GameStateMutationError::FleetOwnershipMismatch {
                player_index_1_based,
                fleet_index_1_based,
            });
        }
        if local_slot == 0
            || self.fleets.iter().enumerate().any(|(idx, other)| {
                idx + 1 != fleet_index_1_based
                    && other.owner.0 == owner_empire
                    && other.fleet_number.0 == local_slot
            })
        {
            return Err(GameStateMutationError::InvalidFleetLocalSlot {
                fleet_index_1_based,
                requested: local_slot,
            });
        }
        // ECGAME's renumbering (`0x15b35`) points the empire's Join orders
        // that name the old number at the new one.
        let old_slot = fleet.fleet_number.0;
        for joiner in self.fleets.iter_mut().filter(|joiner| {
            joiner.owner.0 == owner_empire
                && joiner.order == Order::JoinAnotherFleet
                && joiner.mission_target == old_slot
        }) {
            joiner.mission_target = local_slot;
        }
        self.fleets[fleet_index_1_based - 1].fleet_number.0 = local_slot;
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
            .get(record_index_1_based - 1)
            .ok_or(GameStateMutationError::MissingFleetRecord {
                index_1_based: record_index_1_based,
            })?
            .max_speed;
        if Order::from_code(order_code) == Order::SeekHome
            && ![
                self.nearest_owned_world(record_index_1_based, false),
                self.nearest_owned_world(record_index_1_based, true),
            ]
            .contains(&Some(target))
        {
            // ECGAME picks the Seek Home target itself: the nearest owned
            // world, or the next closest when the fleet is already there.
            return Err(GameStateMutationError::InvalidFleetOrder {
                fleet_index_1_based: record_index_1_based,
                reason: FleetOrderValidationError::NotNearestOwnedWorld,
            });
        }
        if speed > max_speed {
            return Err(GameStateMutationError::InvalidFleetSpeed {
                fleet_index_1_based: record_index_1_based,
                requested: speed,
                max: max_speed,
            });
        }
        let record = self.fleets.get_mut(record_index_1_based - 1).ok_or(
            GameStateMutationError::MissingFleetRecord {
                index_1_based: record_index_1_based,
            },
        )?;
        record.speed = speed;
        record.order = Order::from_code(order_code);
        record.target = Coords::from_array(target);
        // ECGAME writes 0x22 = world-target flag (0 for sector-targeted orders,
        // 1 otherwise) and the 0x23 word = mission id (starbase id for Guard
        // Starbase, host fleet number for Join); explicit aux bytes override
        // 0x22 and 0x23 byte for byte, and the high byte 0x24 is zero.
        let mut mission_aux = [u8::from(Order::from_code(order_code).targets_world()), 0];
        if let Some(value) = aux0 {
            mission_aux[0] = value;
        }
        if let Some(value) = aux1 {
            mission_aux[1] = value;
        }
        set_mission_target(record, mission_aux[0], u16::from(mission_aux[1]));
        // ECGAME clears ROE on any order given to a fleet with no BB, CA, or DD.
        if !has_combat_ships(record) {
            record.rules_of_engagement = 0;
        }
        Ok([record.target_depth, record.mission_target as u8])
    }

    pub fn validate_fleet_order_payload(
        &self,
        record_index_1_based: usize,
        order_code: u8,
        target: [u8; 2],
        aux0: Option<u8>,
        aux1: Option<u8>,
    ) -> Result<(), FleetOrderValidationError> {
        let Some(fleet) = self.fleets.get(record_index_1_based - 1) else {
            return Ok(());
        };
        let order = Order::from_code(order_code);
        if matches!(order, Order::Unknown(_)) {
            return Err(FleetOrderValidationError::UnknownOrderCode(order_code));
        }
        let owner = fleet.owner.0;
        let has_combat = has_combat_ships(fleet);
        let has_scout = fleet.ships.scouts > 0;
        let has_etac = fleet.ships.etacs > 0;
        let has_loaded_troops = fleet.ships.loaded_transports > 0;
        let planet_owner = self
            .planets
            .iter()
            .find(|planet| planet.sector.to_array() == target)
            .map(|planet| planet.owner.0);

        match order {
            Order::GuardStarbase => {
                if !has_combat {
                    return Err(FleetOrderValidationError::MissingCombatShips);
                }
                let _ = aux0;
                let base_id = aux1.map(u16::from).unwrap_or_else(|| fleet.mission_target);
                if base_id == 0
                    || !self
                        .bases
                        .iter()
                        .any(|base| base.owner.0 == owner && base.base_id.0 == base_id)
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
                let host_number = aux1.map(u16::from).unwrap_or_else(|| fleet.mission_target);
                if host_number == 0
                    || host_number == fleet.fleet_number.0
                    || !self
                        .fleets
                        .iter()
                        .any(|host| host.owner.0 == owner && host.fleet_number.0 == host_number)
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
        let Some(fleet) = self.fleets.get(record_index_1_based - 1) else {
            return Ok(());
        };
        if fleet.speed > fleet.max_speed {
            return Err(FleetPlayerInputValidationError::SpeedExceedsMaximum {
                speed: fleet.speed,
                max: fleet.max_speed,
            });
        }
        if is_support_only(fleet) && fleet.rules_of_engagement != 0 {
            return Err(
                FleetPlayerInputValidationError::NonCombatFleetMustUseZeroRoe {
                    roe: fleet.rules_of_engagement,
                },
            );
        }
        if fleet.rules_of_engagement > 10 {
            return Err(
                FleetPlayerInputValidationError::RulesOfEngagementOutOfRange {
                    roe: fleet.rules_of_engagement,
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
        let host = self.fleets.get(host_fleet_index_1_based - 1).ok_or(
            GameStateMutationError::MissingFleetRecord {
                index_1_based: host_fleet_index_1_based,
            },
        )?;
        let fleet = self.fleets.get(fleet_index_1_based - 1).ok_or(
            GameStateMutationError::MissingFleetRecord {
                index_1_based: fleet_index_1_based,
            },
        )?;

        if fleet.owner.0 != owner_empire {
            return Err(GameStateMutationError::FleetOwnershipMismatch {
                player_index_1_based,
                fleet_index_1_based,
            });
        }
        if host.owner.0 != owner_empire {
            return Err(GameStateMutationError::FleetOwnershipMismatch {
                player_index_1_based,
                fleet_index_1_based: host_fleet_index_1_based,
            });
        }

        let host_coords = host.sector.to_array();
        // ECGAME stores the host's fleet number (its 0x00 word), which is
        // what ECMAINT's merge matches against; the record id is not stored.
        let host_fleet_number = host.fleet_number.0;
        let fleet = self.fleets.get_mut(fleet_index_1_based - 1).ok_or(
            GameStateMutationError::MissingFleetRecord {
                index_1_based: fleet_index_1_based,
            },
        )?;
        fleet.speed = fleet.max_speed;
        fleet.order = Order::JoinAnotherFleet;
        fleet.target = Coords::from_array(host_coords);
        set_mission_target(fleet, 1, host_fleet_number);
        Ok(())
    }

    /// ECGAME's speed change (`0x11c77`): recompute the maximum speed (0x09)
    /// from the ships aboard, then write the current speed (0x0a). A fleet
    /// on Hold, or already in its target sector at the target depth layer,
    /// keeps speed 0. Returns the speed written.
    pub fn set_fleet_speed(
        &mut self,
        player_index_1_based: usize,
        fleet_index_1_based: usize,
        speed: u8,
    ) -> Result<u8, GameStateMutationError> {
        let fleet = self.fleets.get_mut(fleet_index_1_based - 1).ok_or(
            GameStateMutationError::MissingFleetRecord {
                index_1_based: fleet_index_1_based,
            },
        )?;
        if fleet.owner.0 != player_index_1_based as u8 {
            return Err(GameStateMutationError::FleetOwnershipMismatch {
                player_index_1_based,
                fleet_index_1_based,
            });
        }
        let mut changed = fleet.clone();
        recompute_max_speed(&mut changed);
        let arrived = changed.target == changed.sector
            && changed.depth.round() == i32::from(changed.target_depth);
        let speed = if arrived || changed.order == Order::HoldPosition {
            0
        } else if (1..=changed.max_speed).contains(&speed) {
            speed
        } else {
            return Err(GameStateMutationError::InvalidFleetSpeed {
                fleet_index_1_based,
                requested: speed,
                max: changed.max_speed,
            });
        };
        changed.speed = speed;
        *fleet = changed;
        Ok(speed)
    }

    pub fn set_fleet_rules_of_engagement(
        &mut self,
        player_index_1_based: usize,
        fleet_index_1_based: usize,
        roe: u8,
    ) -> Result<(), GameStateMutationError> {
        let owner_empire = player_index_1_based as u8;
        let fleet = self.fleets.get(fleet_index_1_based - 1).ok_or(
            GameStateMutationError::MissingFleetRecord {
                index_1_based: fleet_index_1_based,
            },
        )?;
        if fleet.owner.0 != owner_empire {
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
        if is_support_only(fleet) && roe != 0 {
            return Err(GameStateMutationError::InvalidFleetPlayerInput {
                fleet_index_1_based,
                reason: FleetPlayerInputValidationError::NonCombatFleetMustUseZeroRoe { roe },
            });
        }

        self.fleets[fleet_index_1_based - 1].rules_of_engagement = roe;
        Ok(())
    }
}
