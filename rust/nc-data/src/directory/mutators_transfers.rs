use super::support::*;
use super::*;

impl CoreGameData {
    pub fn detach_ships_to_new_fleet(
        &mut self,
        player_index_1_based: usize,
        donor_fleet_record_index_1_based: usize,
        selection: FleetDetachSelection,
        donor_speed: Option<u8>,
        new_fleet_roe: u8,
    ) -> Result<FleetDetachResult, GameStateMutationError> {
        let owner_empire = player_index_1_based as u8;
        let donor = self
            .fleets
            .records
            .get(donor_fleet_record_index_1_based - 1)
            .ok_or(GameStateMutationError::MissingFleetRecord {
                index_1_based: donor_fleet_record_index_1_based,
            })?;
        if donor.owner_empire_raw() != owner_empire {
            return Err(GameStateMutationError::FleetOwnershipMismatch {
                player_index_1_based,
                fleet_index_1_based: donor_fleet_record_index_1_based,
            });
        }
        if selection.total_ships() == 0 {
            return Err(GameStateMutationError::FleetDetachSelectionEmpty {
                fleet_index_1_based: donor_fleet_record_index_1_based,
            });
        }

        let available_full_transports = donor.loaded_transport_count();
        let available_empty_transports = donor.empty_transport_count();
        for (ship_kind, requested, available) in [
            (
                "battleships",
                selection.battleships,
                donor.battleship_count(),
            ),
            ("cruisers", selection.cruisers, donor.cruiser_count()),
            ("destroyers", selection.destroyers, donor.destroyer_count()),
            (
                "full transports",
                selection.full_transports,
                available_full_transports,
            ),
            (
                "empty transports",
                selection.empty_transports,
                available_empty_transports,
            ),
            ("scout ships", selection.scouts, donor.scout_count()),
            ("ETAC ships", selection.etacs, donor.etac_count()),
        ] {
            if requested > available {
                return Err(
                    GameStateMutationError::FleetDetachSelectionExceedsAvailable {
                        fleet_index_1_based: donor_fleet_record_index_1_based,
                        ship_kind,
                        requested,
                        available,
                    },
                );
            }
        }

        let remaining_ships = donor
            .total_starships()
            .saturating_sub(selection.total_ships());
        if remaining_ships == 0 {
            return Err(GameStateMutationError::FleetDetachLeavesFleetEmpty {
                fleet_index_1_based: donor_fleet_record_index_1_based,
            });
        }

        let mut donor_after = donor.clone();
        donor_after.set_battleship_count(
            donor_after
                .battleship_count()
                .saturating_sub(selection.battleships),
        );
        donor_after.set_cruiser_count(
            donor_after
                .cruiser_count()
                .saturating_sub(selection.cruisers),
        );
        donor_after.set_destroyer_count(
            donor_after
                .destroyer_count()
                .saturating_sub(selection.destroyers),
        );
        donor_after.set_empty_transport_count(
            donor_after
                .empty_transport_count()
                .saturating_sub(selection.empty_transports),
        );
        donor_after.set_loaded_transport_count(
            donor_after
                .loaded_transport_count()
                .saturating_sub(selection.full_transports),
        );
        donor_after.set_scout_count(donor_after.scout_count().saturating_sub(selection.scouts));
        donor_after.set_etac_count(donor_after.etac_count().saturating_sub(selection.etacs));
        donor_after.recompute_max_speed_from_composition();
        let donor_mission_invalidated = composition_invalidates_current_mission(&donor_after);
        if donor_mission_invalidated.is_none()
            && donor_after.max_speed() > 0
            && donor.current_speed() > donor_after.max_speed()
        {
            let requested = donor_speed.unwrap_or(donor_after.max_speed());
            if requested == 0 || requested > donor_after.max_speed() {
                return Err(GameStateMutationError::InvalidFleetSpeed {
                    fleet_index_1_based: donor_fleet_record_index_1_based,
                    requested,
                    max: donor_after.max_speed(),
                });
            }
            donor_after.set_current_speed(requested);
        }
        if donor_mission_invalidated.is_some() {
            set_fleet_to_local_hold(&mut donor_after);
        }
        clear_roe_without_warships(&mut donor_after);

        let mut new_fleet = FleetRecord::new_zeroed();
        new_fleet.set_owner_empire_raw(owner_empire);
        new_fleet.set_local_slot_word_raw(next_available_owned_fleet_local_slot(
            &self.fleets.records,
            owner_empire,
        ));
        new_fleet.set_fleet_id_word_raw(next_available_global_fleet_id(&self.fleets.records));
        new_fleet.set_current_location_coords_raw(donor.current_location_coords_raw());
        new_fleet.set_tuple_a_payload_raw(donor.tuple_a_payload_raw());
        new_fleet.set_tuple_b_payload_raw(donor.tuple_b_payload_raw());
        new_fleet.set_tuple_c_payload_raw(donor.tuple_c_payload_raw());
        new_fleet.set_standing_order_kind(Order::HoldPosition);
        new_fleet.set_standing_order_target_coords_raw(donor.current_location_coords_raw());
        new_fleet.set_mission_target(0, 0);
        new_fleet.set_rules_of_engagement(new_fleet_roe.min(10));
        new_fleet.set_battleship_count(selection.battleships);
        new_fleet.set_cruiser_count(selection.cruisers);
        new_fleet.set_destroyer_count(selection.destroyers);
        new_fleet.set_empty_transport_count(selection.empty_transports);
        new_fleet.set_loaded_transport_count(selection.full_transports);
        new_fleet.set_scout_count(selection.scouts);
        new_fleet.set_etac_count(selection.etacs);
        new_fleet.recompute_max_speed_from_composition();
        new_fleet.set_current_speed(0);
        clear_roe_without_warships(&mut new_fleet);

        let donor_fleet_number = donor_after.local_slot_word_raw();
        self.fleets.records[donor_fleet_record_index_1_based - 1] = donor_after;
        let player = self
            .player
            .records
            .get_mut(player_index_1_based - 1)
            .ok_or(GameStateMutationError::MissingPlayerRecord {
                index_1_based: player_index_1_based,
            })?;
        let new_fleet_record_index_1_based =
            append_fleet_to_owner_chain(&mut self.fleets.records, player, new_fleet);
        // The chain sort can move the donor's contents to another record.
        let donor_fleet_record_index_1_based = self
            .fleets
            .records
            .iter()
            .position(|fleet| {
                fleet.owner_empire_raw() == owner_empire
                    && fleet.local_slot_word_raw() == donor_fleet_number
            })
            .map_or(donor_fleet_record_index_1_based, |idx| idx + 1);

        Ok(FleetDetachResult {
            donor_fleet_record_index_1_based,
            new_fleet_record_index_1_based,
        })
    }

    pub fn transfer_ships_between_fleets(
        &mut self,
        player_index_1_based: usize,
        donor_fleet_record_index_1_based: usize,
        host_fleet_record_index_1_based: usize,
        selection: FleetDetachSelection,
    ) -> Result<FleetTransferResult, GameStateMutationError> {
        if donor_fleet_record_index_1_based == host_fleet_record_index_1_based {
            return Err(GameStateMutationError::InvalidFleetMergeSelection {
                fleet_index_1_based: donor_fleet_record_index_1_based,
                host_fleet_index_1_based: host_fleet_record_index_1_based,
            });
        }

        let owner_empire = player_index_1_based as u8;
        let donor = self
            .fleets
            .records
            .get(donor_fleet_record_index_1_based - 1)
            .ok_or(GameStateMutationError::MissingFleetRecord {
                index_1_based: donor_fleet_record_index_1_based,
            })?
            .clone();
        let host = self
            .fleets
            .records
            .get(host_fleet_record_index_1_based - 1)
            .ok_or(GameStateMutationError::MissingFleetRecord {
                index_1_based: host_fleet_record_index_1_based,
            })?
            .clone();

        if donor.owner_empire_raw() != owner_empire {
            return Err(GameStateMutationError::FleetOwnershipMismatch {
                player_index_1_based,
                fleet_index_1_based: donor_fleet_record_index_1_based,
            });
        }
        if host.owner_empire_raw() != owner_empire {
            return Err(GameStateMutationError::FleetOwnershipMismatch {
                player_index_1_based,
                fleet_index_1_based: host_fleet_record_index_1_based,
            });
        }
        if donor.current_location_coords_raw() != host.current_location_coords_raw() {
            return Err(GameStateMutationError::InvalidFleetMergeSelection {
                fleet_index_1_based: donor_fleet_record_index_1_based,
                host_fleet_index_1_based: host_fleet_record_index_1_based,
            });
        }
        if selection.total_ships() == 0 {
            return Err(GameStateMutationError::FleetDetachSelectionEmpty {
                fleet_index_1_based: donor_fleet_record_index_1_based,
            });
        }

        let available_full_transports = donor.loaded_transport_count();
        let available_empty_transports = donor.empty_transport_count();
        for (ship_kind, requested, available) in [
            (
                "battleships",
                selection.battleships,
                donor.battleship_count(),
            ),
            ("cruisers", selection.cruisers, donor.cruiser_count()),
            ("destroyers", selection.destroyers, donor.destroyer_count()),
            (
                "full transports",
                selection.full_transports,
                available_full_transports,
            ),
            (
                "empty transports",
                selection.empty_transports,
                available_empty_transports,
            ),
            ("scout ships", selection.scouts, donor.scout_count()),
            ("ETAC ships", selection.etacs, donor.etac_count()),
        ] {
            if requested > available {
                return Err(
                    GameStateMutationError::FleetDetachSelectionExceedsAvailable {
                        fleet_index_1_based: donor_fleet_record_index_1_based,
                        ship_kind,
                        requested,
                        available,
                    },
                );
            }
        }

        let remaining_ships = donor
            .total_starships()
            .saturating_sub(selection.total_ships());
        if remaining_ships == 0 {
            return Err(GameStateMutationError::FleetDetachLeavesFleetEmpty {
                fleet_index_1_based: donor_fleet_record_index_1_based,
            });
        }

        let mut donor_after = donor.clone();
        donor_after.set_battleship_count(
            donor_after
                .battleship_count()
                .saturating_sub(selection.battleships),
        );
        donor_after.set_cruiser_count(
            donor_after
                .cruiser_count()
                .saturating_sub(selection.cruisers),
        );
        donor_after.set_destroyer_count(
            donor_after
                .destroyer_count()
                .saturating_sub(selection.destroyers),
        );
        donor_after.set_empty_transport_count(
            donor_after
                .empty_transport_count()
                .saturating_sub(selection.empty_transports),
        );
        donor_after.set_loaded_transport_count(
            donor_after
                .loaded_transport_count()
                .saturating_sub(selection.full_transports),
        );
        donor_after.set_scout_count(donor_after.scout_count().saturating_sub(selection.scouts));
        donor_after.set_etac_count(donor_after.etac_count().saturating_sub(selection.etacs));
        donor_after.recompute_max_speed_from_composition();
        if donor_after.current_speed() > donor_after.max_speed() {
            donor_after.set_current_speed(donor_after.max_speed());
        }

        let mut host_after = host.clone();
        host_after.set_battleship_count(
            host_after
                .battleship_count()
                .saturating_add(selection.battleships),
        );
        host_after.set_cruiser_count(
            host_after
                .cruiser_count()
                .saturating_add(selection.cruisers),
        );
        host_after.set_destroyer_count(
            host_after
                .destroyer_count()
                .saturating_add(selection.destroyers),
        );
        host_after.set_empty_transport_count(
            host_after
                .empty_transport_count()
                .saturating_add(selection.empty_transports),
        );
        host_after.set_loaded_transport_count(
            host_after
                .loaded_transport_count()
                .saturating_add(selection.full_transports),
        );
        host_after.set_scout_count(host_after.scout_count().saturating_add(selection.scouts));
        host_after.set_etac_count(host_after.etac_count().saturating_add(selection.etacs));
        host_after.recompute_max_speed_from_composition();
        if host_after.current_speed() > host_after.max_speed() {
            host_after.set_current_speed(host_after.max_speed());
        }

        // ECGAME's transfer leaves the host's ROE alone.
        normalize_post_composition_fleet_state(&mut donor_after);
        clear_roe_without_warships(&mut donor_after);

        self.fleets.records[donor_fleet_record_index_1_based - 1] = donor_after;
        self.fleets.records[host_fleet_record_index_1_based - 1] = host_after;

        Ok(FleetTransferResult {
            donor_fleet_record_index_1_based,
            host_fleet_record_index_1_based,
        })
    }

    pub fn load_planet_armies_onto_fleet(
        &mut self,
        player_index_1_based: usize,
        planet_record_index_1_based: usize,
        fleet_record_index_1_based: usize,
        qty: u16,
    ) -> Result<(), GameStateMutationError> {
        if qty == 0 {
            return Ok(());
        }
        let owner_empire = player_index_1_based as u8;
        let planet = self
            .planets
            .records
            .get(planet_record_index_1_based - 1)
            .ok_or(GameStateMutationError::MissingPlanetRecord {
                index_1_based: planet_record_index_1_based,
            })?;
        let fleet = self
            .fleets
            .records
            .get(fleet_record_index_1_based - 1)
            .ok_or(GameStateMutationError::MissingFleetRecord {
                index_1_based: fleet_record_index_1_based,
            })?;

        if planet.owner_empire_slot_raw() != owner_empire
            || fleet.owner_empire_raw() != owner_empire
            || fleet.current_location_coords_raw() != planet.coords_raw()
        {
            return Err(GameStateMutationError::FleetNotAtPlanet {
                fleet_index_1_based: fleet_record_index_1_based,
                planet_index_1_based: planet_record_index_1_based,
            });
        }

        let available_planet_armies = u16::from(planet.army_count_raw());
        if qty > available_planet_armies {
            return Err(GameStateMutationError::PlanetArmyShortage {
                planet_index_1_based: planet_record_index_1_based,
                requested: qty,
                available: available_planet_armies,
            });
        }

        let available_transport_capacity = fleet.empty_transport_count();
        if qty > available_transport_capacity {
            return Err(GameStateMutationError::TransportCapacityExceeded {
                fleet_index_1_based: fleet_record_index_1_based,
                requested: qty,
                available: available_transport_capacity,
            });
        }

        let planet = self
            .planets
            .records
            .get_mut(planet_record_index_1_based - 1)
            .ok_or(GameStateMutationError::MissingPlanetRecord {
                index_1_based: planet_record_index_1_based,
            })?;
        let fleet = self
            .fleets
            .records
            .get_mut(fleet_record_index_1_based - 1)
            .ok_or(GameStateMutationError::MissingFleetRecord {
                index_1_based: fleet_record_index_1_based,
            })?;

        planet.set_army_count_raw(planet.army_count_raw().saturating_sub(qty as u8));
        fleet.set_army_count(fleet.army_count().saturating_add(qty));
        Ok(())
    }

    pub fn unload_fleet_armies_to_planet(
        &mut self,
        player_index_1_based: usize,
        planet_record_index_1_based: usize,
        fleet_record_index_1_based: usize,
        qty: u16,
    ) -> Result<(), GameStateMutationError> {
        if qty == 0 {
            return Ok(());
        }
        let owner_empire = player_index_1_based as u8;
        let planet = self
            .planets
            .records
            .get(planet_record_index_1_based - 1)
            .ok_or(GameStateMutationError::MissingPlanetRecord {
                index_1_based: planet_record_index_1_based,
            })?;
        let fleet = self
            .fleets
            .records
            .get(fleet_record_index_1_based - 1)
            .ok_or(GameStateMutationError::MissingFleetRecord {
                index_1_based: fleet_record_index_1_based,
            })?;

        if planet.owner_empire_slot_raw() != owner_empire
            || fleet.owner_empire_raw() != owner_empire
            || fleet.current_location_coords_raw() != planet.coords_raw()
        {
            return Err(GameStateMutationError::FleetNotAtPlanet {
                fleet_index_1_based: fleet_record_index_1_based,
                planet_index_1_based: planet_record_index_1_based,
            });
        }

        let available_loaded_armies = fleet.army_count();
        if qty > available_loaded_armies {
            return Err(GameStateMutationError::FleetArmyShortage {
                fleet_index_1_based: fleet_record_index_1_based,
                requested: qty,
                available: available_loaded_armies,
            });
        }
        let available_planet_capacity = u16::from(u8::MAX.saturating_sub(planet.army_count_raw()));
        if qty > available_planet_capacity {
            return Err(GameStateMutationError::PlanetArmyCapacityExceeded {
                planet_index_1_based: planet_record_index_1_based,
                requested: qty,
                available: available_planet_capacity,
            });
        }

        let planet = self
            .planets
            .records
            .get_mut(planet_record_index_1_based - 1)
            .ok_or(GameStateMutationError::MissingPlanetRecord {
                index_1_based: planet_record_index_1_based,
            })?;
        let fleet = self
            .fleets
            .records
            .get_mut(fleet_record_index_1_based - 1)
            .ok_or(GameStateMutationError::MissingFleetRecord {
                index_1_based: fleet_record_index_1_based,
            })?;

        planet.set_army_count_raw(planet.army_count_raw().saturating_add(qty as u8));
        fleet.set_army_count(fleet.army_count().saturating_sub(qty));
        normalize_post_composition_fleet_state(fleet);
        Ok(())
    }
}
