use super::support::*;
use super::*;
use crate::{
    ResolvedSelection, recompute_max_speed, resolve_selection, set_armies_aboard,
    set_mission_target, starship_count,
};

/// The recovery state that goes with a selection: `n!` takes crippled ships,
/// a plain number takes healthy ships first (see [`resolve_selection`]).
fn resolve_moved_ships(
    donor: &Fleet,
    fleet_index_1_based: usize,
    selection: &FleetDetachSelection,
) -> Result<ResolvedSelection, GameStateMutationError> {
    resolve_selection(donor, selection).map_err(|shortage| {
        GameStateMutationError::FleetDetachCrippledExceedsAvailable {
            fleet_index_1_based,
            ship_kind: shortage.category.code(),
            requested: shortage.requested,
            available: shortage.available,
        }
    })
}

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
            .get(donor_fleet_record_index_1_based - 1)
            .ok_or(GameStateMutationError::MissingFleetRecord {
                index_1_based: donor_fleet_record_index_1_based,
            })?;
        if donor.owner.0 != owner_empire {
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

        let available_full_transports = donor.ships.loaded_transports;
        let available_empty_transports = donor.ships.empty_transports;
        for (ship_kind, requested, available) in [
            (
                "battleships",
                selection.battleships,
                donor.ships.battleships,
            ),
            ("cruisers", selection.cruisers, donor.ships.cruisers),
            ("destroyers", selection.destroyers, donor.ships.destroyers),
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
            ("scout ships", selection.scouts, donor.ships.scouts),
            ("ETAC ships", selection.etacs, donor.ships.etacs),
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

        let remaining_ships = starship_count(donor).saturating_sub(selection.total_ships());
        if remaining_ships == 0 {
            return Err(GameStateMutationError::FleetDetachLeavesFleetEmpty {
                fleet_index_1_based: donor_fleet_record_index_1_based,
            });
        }

        let mut donor_after = donor.clone();
        donor_after.ships.battleships = donor_after
            .ships
            .battleships
            .saturating_sub(selection.battleships);
        donor_after.ships.cruisers = donor_after
            .ships
            .cruisers
            .saturating_sub(selection.cruisers);
        donor_after.ships.destroyers = donor_after
            .ships
            .destroyers
            .saturating_sub(selection.destroyers);
        donor_after.ships.empty_transports = donor_after
            .ships
            .empty_transports
            .saturating_sub(selection.empty_transports);
        donor_after.ships.loaded_transports = donor_after
            .ships
            .loaded_transports
            .saturating_sub(selection.full_transports);
        donor_after.ships.scouts = donor_after.ships.scouts.saturating_sub(selection.scouts);
        donor_after.ships.etacs = donor_after.ships.etacs.saturating_sub(selection.etacs);
        let moved = resolve_moved_ships(donor, donor_fleet_record_index_1_based, &selection)?;
        donor_after.crippled = moved.remaining;
        recompute_max_speed(&mut donor_after);
        let donor_mission_invalidated = composition_invalidates_current_mission(&donor_after);
        if donor_mission_invalidated.is_none()
            && donor_after.max_speed > 0
            && donor.speed > donor_after.max_speed
        {
            let requested = donor_speed.unwrap_or(donor_after.max_speed);
            if requested == 0 || requested > donor_after.max_speed {
                return Err(GameStateMutationError::InvalidFleetSpeed {
                    fleet_index_1_based: donor_fleet_record_index_1_based,
                    requested,
                    max: donor_after.max_speed,
                });
            }
            donor_after.speed = requested;
        }
        if donor_mission_invalidated.is_some() {
            set_fleet_to_local_hold(&mut donor_after);
        }
        clear_roe_without_warships(&mut donor_after);

        let mut new_fleet = Fleet::default();
        new_fleet.owner.0 = owner_empire;
        new_fleet.fleet_number.0 =
            next_available_owned_fleet_local_slot(&self.fleets, owner_empire);
        new_fleet.record.0 = next_available_global_fleet_id(&self.fleets);
        new_fleet.sector = donor.sector;
        new_fleet.x_fraction = donor.x_fraction;
        new_fleet.y_fraction = donor.y_fraction;
        new_fleet.depth = donor.depth;
        new_fleet.order = Order::HoldPosition;
        new_fleet.target = donor.sector;
        set_mission_target(&mut new_fleet, 0, 0);
        new_fleet.rules_of_engagement = new_fleet_roe.min(10);
        new_fleet.ships.battleships = selection.battleships;
        new_fleet.ships.cruisers = selection.cruisers;
        new_fleet.ships.destroyers = selection.destroyers;
        new_fleet.ships.empty_transports = selection.empty_transports;
        new_fleet.ships.loaded_transports = selection.full_transports;
        new_fleet.ships.scouts = selection.scouts;
        new_fleet.ships.etacs = selection.etacs;
        new_fleet.crippled = moved.moved;
        // Ships detached from an escaping fleet are still escaping: a split
        // gives nobody a second pursuit.
        new_fleet.escape = donor.escape;
        recompute_max_speed(&mut new_fleet);
        new_fleet.speed = 0;
        clear_roe_without_warships(&mut new_fleet);

        let donor_fleet_number = donor_after.fleet_number.0;
        self.fleets[donor_fleet_record_index_1_based - 1] = donor_after;
        let player = self.players.get_mut(player_index_1_based - 1).ok_or(
            GameStateMutationError::MissingPlayerRecord {
                index_1_based: player_index_1_based,
            },
        )?;
        let new_fleet_record_index_1_based =
            append_fleet_to_owner_chain(&mut self.fleets, player, new_fleet);
        // The chain sort can move the donor's contents to another record.
        let donor_fleet_record_index_1_based = self
            .fleets
            .iter()
            .position(|fleet| {
                fleet.owner.0 == owner_empire && fleet.fleet_number.0 == donor_fleet_number
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
            .get(donor_fleet_record_index_1_based - 1)
            .ok_or(GameStateMutationError::MissingFleetRecord {
                index_1_based: donor_fleet_record_index_1_based,
            })?
            .clone();
        let host = self
            .fleets
            .get(host_fleet_record_index_1_based - 1)
            .ok_or(GameStateMutationError::MissingFleetRecord {
                index_1_based: host_fleet_record_index_1_based,
            })?
            .clone();

        if donor.owner.0 != owner_empire {
            return Err(GameStateMutationError::FleetOwnershipMismatch {
                player_index_1_based,
                fleet_index_1_based: donor_fleet_record_index_1_based,
            });
        }
        if host.owner.0 != owner_empire {
            return Err(GameStateMutationError::FleetOwnershipMismatch {
                player_index_1_based,
                fleet_index_1_based: host_fleet_record_index_1_based,
            });
        }
        if donor.sector != host.sector {
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

        let available_full_transports = donor.ships.loaded_transports;
        let available_empty_transports = donor.ships.empty_transports;
        for (ship_kind, requested, available) in [
            (
                "battleships",
                selection.battleships,
                donor.ships.battleships,
            ),
            ("cruisers", selection.cruisers, donor.ships.cruisers),
            ("destroyers", selection.destroyers, donor.ships.destroyers),
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
            ("scout ships", selection.scouts, donor.ships.scouts),
            ("ETAC ships", selection.etacs, donor.ships.etacs),
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

        let remaining_ships = starship_count(&donor).saturating_sub(selection.total_ships());
        if remaining_ships == 0 {
            return Err(GameStateMutationError::FleetDetachLeavesFleetEmpty {
                fleet_index_1_based: donor_fleet_record_index_1_based,
            });
        }

        let mut donor_after = donor.clone();
        donor_after.ships.battleships = donor_after
            .ships
            .battleships
            .saturating_sub(selection.battleships);
        donor_after.ships.cruisers = donor_after
            .ships
            .cruisers
            .saturating_sub(selection.cruisers);
        donor_after.ships.destroyers = donor_after
            .ships
            .destroyers
            .saturating_sub(selection.destroyers);
        donor_after.ships.empty_transports = donor_after
            .ships
            .empty_transports
            .saturating_sub(selection.empty_transports);
        donor_after.ships.loaded_transports = donor_after
            .ships
            .loaded_transports
            .saturating_sub(selection.full_transports);
        donor_after.ships.scouts = donor_after.ships.scouts.saturating_sub(selection.scouts);
        donor_after.ships.etacs = donor_after.ships.etacs.saturating_sub(selection.etacs);
        let moved = resolve_moved_ships(&donor, donor_fleet_record_index_1_based, &selection)?;
        donor_after.crippled = moved.remaining;
        recompute_max_speed(&mut donor_after);
        if donor_after.speed > donor_after.max_speed {
            donor_after.speed = donor_after.max_speed;
        }

        let mut host_after = host.clone();
        host_after.ships.battleships = host_after
            .ships
            .battleships
            .saturating_add(selection.battleships);
        host_after.ships.cruisers = host_after.ships.cruisers.saturating_add(selection.cruisers);
        host_after.ships.destroyers = host_after
            .ships
            .destroyers
            .saturating_add(selection.destroyers);
        host_after.ships.empty_transports = host_after
            .ships
            .empty_transports
            .saturating_add(selection.empty_transports);
        host_after.ships.loaded_transports = host_after
            .ships
            .loaded_transports
            .saturating_add(selection.full_transports);
        host_after.ships.scouts = host_after.ships.scouts.saturating_add(selection.scouts);
        host_after.ships.etacs = host_after.ships.etacs.saturating_add(selection.etacs);
        host_after.crippled.add(&moved.moved);
        recompute_max_speed(&mut host_after);
        if host_after.speed > host_after.max_speed {
            host_after.speed = host_after.max_speed;
        }

        // ECGAME's transfer leaves the host's ROE alone.
        normalize_post_composition_fleet_state(&mut donor_after);
        clear_roe_without_warships(&mut donor_after);

        self.fleets[donor_fleet_record_index_1_based - 1] = donor_after;
        self.fleets[host_fleet_record_index_1_based - 1] = host_after;

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
        let planet = self.planets.get(planet_record_index_1_based - 1).ok_or(
            GameStateMutationError::MissingPlanetRecord {
                index_1_based: planet_record_index_1_based,
            },
        )?;
        let fleet = self.fleets.get(fleet_record_index_1_based - 1).ok_or(
            GameStateMutationError::MissingFleetRecord {
                index_1_based: fleet_record_index_1_based,
            },
        )?;

        if planet.owner.0 != owner_empire
            || fleet.owner.0 != owner_empire
            || fleet.sector != planet.sector
        {
            return Err(GameStateMutationError::FleetNotAtPlanet {
                fleet_index_1_based: fleet_record_index_1_based,
                planet_index_1_based: planet_record_index_1_based,
            });
        }

        let available_planet_armies = u16::from(u8::try_from(planet.armies).unwrap_or(u8::MAX));
        if qty > available_planet_armies {
            return Err(GameStateMutationError::PlanetArmyShortage {
                planet_index_1_based: planet_record_index_1_based,
                requested: qty,
                available: available_planet_armies,
            });
        }

        let available_transport_capacity = fleet.ships.empty_transports;
        if qty > available_transport_capacity {
            return Err(GameStateMutationError::TransportCapacityExceeded {
                fleet_index_1_based: fleet_record_index_1_based,
                requested: qty,
                available: available_transport_capacity,
            });
        }

        let planet = self
            .planets
            .get_mut(planet_record_index_1_based - 1)
            .ok_or(GameStateMutationError::MissingPlanetRecord {
                index_1_based: planet_record_index_1_based,
            })?;
        let fleet = self.fleets.get_mut(fleet_record_index_1_based - 1).ok_or(
            GameStateMutationError::MissingFleetRecord {
                index_1_based: fleet_record_index_1_based,
            },
        )?;

        planet.armies = u16::from(
            u8::try_from(planet.armies)
                .unwrap_or(u8::MAX)
                .saturating_sub(qty as u8),
        );
        set_armies_aboard(fleet, fleet.ships.loaded_transports.saturating_add(qty));
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
        let planet = self.planets.get(planet_record_index_1_based - 1).ok_or(
            GameStateMutationError::MissingPlanetRecord {
                index_1_based: planet_record_index_1_based,
            },
        )?;
        let fleet = self.fleets.get(fleet_record_index_1_based - 1).ok_or(
            GameStateMutationError::MissingFleetRecord {
                index_1_based: fleet_record_index_1_based,
            },
        )?;

        if planet.owner.0 != owner_empire
            || fleet.owner.0 != owner_empire
            || fleet.sector != planet.sector
        {
            return Err(GameStateMutationError::FleetNotAtPlanet {
                fleet_index_1_based: fleet_record_index_1_based,
                planet_index_1_based: planet_record_index_1_based,
            });
        }

        let available_loaded_armies = fleet.ships.loaded_transports;
        if qty > available_loaded_armies {
            return Err(GameStateMutationError::FleetArmyShortage {
                fleet_index_1_based: fleet_record_index_1_based,
                requested: qty,
                available: available_loaded_armies,
            });
        }
        let available_planet_capacity =
            u16::from(u8::MAX.saturating_sub(u8::try_from(planet.armies).unwrap_or(u8::MAX)));
        if qty > available_planet_capacity {
            return Err(GameStateMutationError::PlanetArmyCapacityExceeded {
                planet_index_1_based: planet_record_index_1_based,
                requested: qty,
                available: available_planet_capacity,
            });
        }

        let planet = self
            .planets
            .get_mut(planet_record_index_1_based - 1)
            .ok_or(GameStateMutationError::MissingPlanetRecord {
                index_1_based: planet_record_index_1_based,
            })?;
        let fleet = self.fleets.get_mut(fleet_record_index_1_based - 1).ok_or(
            GameStateMutationError::MissingFleetRecord {
                index_1_based: fleet_record_index_1_based,
            },
        )?;

        planet.armies = u16::from(
            u8::try_from(planet.armies)
                .unwrap_or(u8::MAX)
                .saturating_add(qty as u8),
        );
        set_armies_aboard(fleet, fleet.ships.loaded_transports.saturating_sub(qty));
        normalize_post_composition_fleet_state(fleet);
        Ok(())
    }
}
