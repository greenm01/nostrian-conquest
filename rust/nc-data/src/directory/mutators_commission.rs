use super::support::*;
use super::*;
use crate::Planet;
use crate::{
    Coords, recompute_max_speed, set_fleet_at_rest, set_mission_target, set_troop_transports,
    stardock_slot_kind,
};

impl CoreGameData {
    pub fn commission_planet_stardock_slot(
        &mut self,
        player_index_1_based: usize,
        planet_record_index_1_based: usize,
        slot_0_based: usize,
    ) -> Result<CommissionResult, GameStateMutationError> {
        self.commission_planet_stardock_slots(
            player_index_1_based,
            planet_record_index_1_based,
            &[slot_0_based],
        )
    }

    pub fn commission_planet_stardock_slots(
        &mut self,
        player_index_1_based: usize,
        planet_record_index_1_based: usize,
        slot_0_based_list: &[usize],
    ) -> Result<CommissionResult, GameStateMutationError> {
        if slot_0_based_list.is_empty() {
            return Err(GameStateMutationError::InvalidCommissionSelection);
        }

        let owner_empire = player_index_1_based as u8;
        let planet = self.planets.get(planet_record_index_1_based - 1).ok_or(
            GameStateMutationError::MissingPlanetRecord {
                index_1_based: planet_record_index_1_based,
            },
        )?;
        if planet.owner.0 != owner_empire {
            return Err(GameStateMutationError::PlanetOwnershipMismatch {
                player_index_1_based,
                planet_index_1_based: planet_record_index_1_based,
            });
        }
        let coords = planet.sector.to_array();
        let selected = collect_selected_stardock_slots(
            planet,
            planet_record_index_1_based,
            slot_0_based_list,
        )?;

        let starbase_count = selected
            .iter()
            .filter(|(_, kind_raw, _)| *kind_raw == 9)
            .count();
        if starbase_count > 1 || (starbase_count == 1 && selected.len() > 1) {
            return Err(GameStateMutationError::InvalidCommissionSelection);
        }

        if starbase_count == 1 {
            if self.players.get(player_index_1_based - 1).is_none() {
                return Err(GameStateMutationError::MissingPlayerRecord {
                    index_1_based: player_index_1_based,
                });
            }
            let record_idx = self.bases.len();
            self.bases.push(build_guard_starbase_base_record(
                coords,
                0,
                0,
                (record_idx + 1) as u16,
                owner_empire,
                Real48::HALF,
                Real48::HALF,
                Real48::ONE,
            ));
            // The new base takes the lowest free id in its owner's chain.
            self.link_new_base(record_idx);
            let planet = self
                .planets
                .get_mut(planet_record_index_1_based - 1)
                .ok_or(GameStateMutationError::MissingPlanetRecord {
                    index_1_based: planet_record_index_1_based,
                })?;
            // The starbase slot holds every docked base; one leaves.
            for (slot_0_based, _, _) in selected {
                let left = planet.stardock[slot_0_based].saturating_sub(1);
                planet.stardock[slot_0_based] = left;
            }
            Ok(CommissionResult::Starbase {
                base_record_index_1_based: self.bases.len(),
            })
        } else {
            self.commission_planet_stardock_slots_with_draft(
                player_index_1_based,
                planet_record_index_1_based,
                slot_0_based_list,
                full_ship_draft_from_selected(&selected)?,
            )
        }
    }

    pub fn commission_planet_stardock_slots_with_draft(
        &mut self,
        player_index_1_based: usize,
        planet_record_index_1_based: usize,
        slot_0_based_list: &[usize],
        draft: CommissionFleetDraft,
    ) -> Result<CommissionResult, GameStateMutationError> {
        if slot_0_based_list.is_empty() || draft.total_ships() == 0 {
            return Err(GameStateMutationError::InvalidCommissionSelection);
        }

        let owner_empire = player_index_1_based as u8;
        let planet = self.planets.get(planet_record_index_1_based - 1).ok_or(
            GameStateMutationError::MissingPlanetRecord {
                index_1_based: planet_record_index_1_based,
            },
        )?;
        if planet.owner.0 != owner_empire {
            return Err(GameStateMutationError::PlanetOwnershipMismatch {
                player_index_1_based,
                planet_index_1_based: planet_record_index_1_based,
            });
        }

        let coords = planet.sector.to_array();
        let selected = collect_selected_stardock_slots(
            planet,
            planet_record_index_1_based,
            slot_0_based_list,
        )?;
        if selected.iter().any(|(_, kind_raw, _)| *kind_raw == 9) {
            return Err(GameStateMutationError::InvalidCommissionSelection);
        }

        let available = full_ship_draft_from_selected(&selected)?;
        if !draft_fits_within_available(draft, available) {
            return Err(GameStateMutationError::InvalidCommissionSelection);
        }

        let result = build_commissioned_fleet(self, player_index_1_based, coords, draft)?;
        let planet = self
            .planets
            .get_mut(planet_record_index_1_based - 1)
            .ok_or(GameStateMutationError::MissingPlanetRecord {
                index_1_based: planet_record_index_1_based,
            })?;
        decrement_selected_ship_slots(planet, &selected, draft)?;
        Ok(result)
    }

    pub fn auto_commission_all_stardock_units(
        &mut self,
        player_index_1_based: usize,
    ) -> Result<AutoCommissionReport, GameStateMutationError> {
        let owner_empire = player_index_1_based as u8;
        let mut report = AutoCommissionReport::default();
        let planet_indices: Vec<usize> = self
            .planets
            .iter()
            .enumerate()
            .filter(|(_, planet)| planet.owner.0 == owner_empire)
            .map(|(idx, _)| idx + 1)
            .collect();

        for planet_index_1_based in planet_indices {
            let Some(planet) = self.planets.get(planet_index_1_based - 1) else {
                return Err(GameStateMutationError::MissingPlanetRecord {
                    index_1_based: planet_index_1_based,
                });
            };
            let planet_name = planet.name.clone();
            let coords = planet.sector.to_array();
            let mut ship_slots = Vec::new();
            let mut starbase_slots = Vec::new();
            let mut ship_count = 0u32;
            let mut destroyers = 0u32;
            let mut cruisers = 0u32;
            let mut battleships = 0u32;
            let mut scouts = 0u32;
            let mut transports = 0u32;
            let mut etacs = 0u32;
            for slot in 0..crate::STARDOCK_SLOT_COUNT {
                let count = u32::from(u16::try_from(planet.stardock[slot]).unwrap_or(u16::MAX));
                if count == 0 {
                    continue;
                }
                match stardock_slot_kind(planet, slot) {
                    ProductionItemKind::Destroyer
                    | ProductionItemKind::Cruiser
                    | ProductionItemKind::Battleship
                    | ProductionItemKind::Scout
                    | ProductionItemKind::Transport
                    | ProductionItemKind::Etac => {
                        ship_slots.push(slot);
                        ship_count = ship_count.saturating_add(count);
                        match stardock_slot_kind(planet, slot) {
                            ProductionItemKind::Destroyer => {
                                destroyers = destroyers.saturating_add(count)
                            }
                            ProductionItemKind::Cruiser => {
                                cruisers = cruisers.saturating_add(count)
                            }
                            ProductionItemKind::Battleship => {
                                battleships = battleships.saturating_add(count)
                            }
                            ProductionItemKind::Scout => scouts = scouts.saturating_add(count),
                            ProductionItemKind::Transport => {
                                transports = transports.saturating_add(count)
                            }
                            ProductionItemKind::Etac => etacs = etacs.saturating_add(count),
                            _ => {}
                        }
                    }
                    // One commission per docked base.
                    ProductionItemKind::Starbase => {
                        starbase_slots.extend(std::iter::repeat_n(slot, count as usize))
                    }
                    _ => {}
                }
            }
            if ship_slots.is_empty() && starbase_slots.is_empty() {
                continue;
            }

            report.planets_used += 1;

            if !ship_slots.is_empty() {
                match self.commission_planet_stardock_slots(
                    player_index_1_based,
                    planet_index_1_based,
                    &ship_slots,
                )? {
                    CommissionResult::Fleet {
                        fleet_record_index_1_based,
                    } => {
                        let fleet_number = self
                            .fleets
                            .get(fleet_record_index_1_based - 1)
                            .ok_or(GameStateMutationError::MissingFleetRecord {
                                index_1_based: fleet_record_index_1_based,
                            })?
                            .fleet_number
                            .0;
                        report.fleets_created += 1;
                        report.ships_commissioned =
                            report.ships_commissioned.saturating_add(ship_count);
                        report
                            .entries
                            .push(AutoCommissionEntry::Fleet(AutoCommissionFleetEntry {
                                fleet_number,
                                planet_name: planet_name.clone(),
                                coords,
                                destroyers,
                                cruisers,
                                battleships,
                                scouts,
                                transports,
                                etacs,
                            }));
                    }
                    CommissionResult::Starbase { .. } => {
                        return Err(GameStateMutationError::InvalidCommissionSelection);
                    }
                }
            }

            for slot in starbase_slots {
                match self.commission_planet_stardock_slot(
                    player_index_1_based,
                    planet_index_1_based,
                    slot,
                )? {
                    CommissionResult::Starbase {
                        base_record_index_1_based,
                    } => {
                        let starbase_number = self
                            .bases
                            .get(base_record_index_1_based - 1)
                            .ok_or(GameStateMutationError::MissingBaseRecord {
                                index_1_based: base_record_index_1_based,
                            })?
                            .base_id
                            .0;
                        report.starbases_commissioned += 1;
                        report.entries.push(AutoCommissionEntry::Starbase(
                            AutoCommissionStarbaseEntry {
                                starbase_number,
                                planet_name: planet_name.clone(),
                                coords,
                            },
                        ));
                    }
                    CommissionResult::Fleet { .. } => {
                        return Err(GameStateMutationError::InvalidCommissionSelection);
                    }
                }
            }
        }

        Ok(report)
    }
    pub fn set_guard_starbase(
        &mut self,
        player_index_1_based: usize,
        fleet_index_1_based: usize,
        target: [u8; 2],
        base_id: u16,
        owner_empire: u8,
    ) -> Result<(), GameStateMutationError> {
        let player = self.players.get_mut(player_index_1_based - 1).ok_or(
            GameStateMutationError::MissingPlayerRecord {
                index_1_based: player_index_1_based,
            },
        )?;
        player.bases_first.0 = 1;

        let fleet = self.fleets.get_mut(fleet_index_1_based - 1).ok_or(
            GameStateMutationError::MissingFleetRecord {
                index_1_based: fleet_index_1_based,
            },
        )?;
        fleet.order = Order::GuardStarbase;
        fleet.target = Coords::from_array(target);
        set_mission_target(fleet, 1, base_id);

        self.bases = vec![build_guard_starbase_base_record(
            target,
            base_id,
            fleet.fleet_number.0,
            fleet.record.0,
            owner_empire,
            fleet.x_fraction,
            fleet.y_fraction,
            fleet.depth,
        )];

        Ok(())
    }

    pub fn set_ipbm_zero_records(&mut self, count: u16) {
        if let Some(player1) = self.players.first_mut() {
            player1.ipbms_first.0 = count;
        }

        self.ipbms = (0..count).map(|_| Ipbm::default()).collect();
    }

    pub fn set_ipbm_record_prefix(
        &mut self,
        record_index_1_based: usize,
        primary: u16,
        owner: u8,
        gate: u16,
        follow_on: u16,
    ) -> Result<(), GameStateMutationError> {
        let record = self.ipbms.get_mut(record_index_1_based - 1).ok_or(
            GameStateMutationError::MissingIpbmRecord {
                index_1_based: record_index_1_based,
            },
        )?;
        record.ipbm_number = primary;
        record.owner.0 = owner;
        record.next.0 = gate;
        record.record.0 = follow_on;
        Ok(())
    }
}

type SelectedStardockSlot = (usize, u8, u16);

fn collect_selected_stardock_slots(
    planet: &Planet,
    planet_index_1_based: usize,
    slot_0_based_list: &[usize],
) -> Result<Vec<SelectedStardockSlot>, GameStateMutationError> {
    let mut selected = Vec::with_capacity(slot_0_based_list.len());
    for &slot_0_based in slot_0_based_list {
        if slot_0_based >= crate::STARDOCK_SLOT_COUNT
            || stardock_slot_kind(planet, slot_0_based).to_raw() == 0
            || planet.stardock[slot_0_based] == 0
        {
            return Err(GameStateMutationError::EmptyStardockSlot {
                planet_index_1_based,
                slot_0_based,
            });
        }
        selected.push((
            slot_0_based,
            stardock_slot_kind(planet, slot_0_based).to_raw(),
            u16::try_from(planet.stardock[slot_0_based]).unwrap_or(u16::MAX),
        ));
    }
    Ok(selected)
}

fn full_ship_draft_from_selected(
    selected: &[SelectedStardockSlot],
) -> Result<CommissionFleetDraft, GameStateMutationError> {
    let mut draft = CommissionFleetDraft::default();
    for (_, kind_raw, count) in selected {
        match ProductionItemKind::from_raw(*kind_raw) {
            ProductionItemKind::Destroyer => {
                draft.destroyers = draft.destroyers.saturating_add(*count)
            }
            ProductionItemKind::Cruiser => draft.cruisers = draft.cruisers.saturating_add(*count),
            ProductionItemKind::Battleship => {
                draft.battleships = draft.battleships.saturating_add(*count)
            }
            ProductionItemKind::Scout => draft.scouts = draft.scouts.saturating_add(*count),
            ProductionItemKind::Transport => {
                draft.transports = draft.transports.saturating_add(*count)
            }
            ProductionItemKind::Etac => draft.etacs = draft.etacs.saturating_add(*count),
            _ => return Err(GameStateMutationError::InvalidCommissionSelection),
        }
    }
    Ok(draft)
}

fn draft_fits_within_available(
    draft: CommissionFleetDraft,
    available: CommissionFleetDraft,
) -> bool {
    draft.destroyers <= available.destroyers
        && draft.cruisers <= available.cruisers
        && draft.battleships <= available.battleships
        && draft.scouts <= available.scouts
        && draft.transports <= available.transports
        && draft.etacs <= available.etacs
}

fn decrement_selected_ship_slots(
    planet: &mut Planet,
    selected: &[SelectedStardockSlot],
    draft: CommissionFleetDraft,
) -> Result<(), GameStateMutationError> {
    let mut remaining_destroyers = draft.destroyers;
    let mut remaining_cruisers = draft.cruisers;
    let mut remaining_battleships = draft.battleships;
    let mut remaining_scouts = draft.scouts;
    let mut remaining_transports = draft.transports;
    let mut remaining_etacs = draft.etacs;

    for (slot_0_based, kind_raw, _) in selected {
        let remaining_for_kind = match ProductionItemKind::from_raw(*kind_raw) {
            ProductionItemKind::Destroyer => &mut remaining_destroyers,
            ProductionItemKind::Cruiser => &mut remaining_cruisers,
            ProductionItemKind::Battleship => &mut remaining_battleships,
            ProductionItemKind::Scout => &mut remaining_scouts,
            ProductionItemKind::Transport => &mut remaining_transports,
            ProductionItemKind::Etac => &mut remaining_etacs,
            _ => return Err(GameStateMutationError::InvalidCommissionSelection),
        };
        if *remaining_for_kind == 0 {
            continue;
        }
        let available = u16::try_from(planet.stardock[*slot_0_based]).unwrap_or(u16::MAX);
        let consumed = available.min(*remaining_for_kind);
        let new_count = available.saturating_sub(consumed);
        planet.stardock[*slot_0_based] = u32::from(new_count);
        *remaining_for_kind = remaining_for_kind.saturating_sub(consumed);
    }

    if remaining_destroyers != 0
        || remaining_cruisers != 0
        || remaining_battleships != 0
        || remaining_scouts != 0
        || remaining_transports != 0
        || remaining_etacs != 0
    {
        return Err(GameStateMutationError::InvalidCommissionSelection);
    }

    Ok(())
}

fn build_commissioned_fleet(
    game_data: &mut CoreGameData,
    player_index_1_based: usize,
    coords: [u8; 2],
    draft: CommissionFleetDraft,
) -> Result<CommissionResult, GameStateMutationError> {
    let owner_empire = player_index_1_based as u8;
    let fleet_id = next_available_global_fleet_id(&game_data.fleets);
    let local_slot = next_available_owned_fleet_local_slot(&game_data.fleets, owner_empire);

    let mut fleet = Fleet::default();
    fleet.fleet_number.0 = local_slot;
    fleet.owner.0 = owner_empire;
    fleet.record.0 = fleet_id;
    fleet.speed = 0;
    fleet.sector = Coords::from_array(coords);
    set_fleet_at_rest(&mut fleet);
    fleet.order = Order::HoldPosition;
    fleet.target = Coords::from_array(coords);
    set_mission_target(&mut fleet, 0, 0);
    fleet.rules_of_engagement = crate::NEW_FLEET_DEFAULT_ROE;
    fleet.ships.destroyers = draft.destroyers;
    fleet.ships.cruisers = draft.cruisers;
    fleet.ships.battleships = draft.battleships;
    fleet.ships.scouts = draft.scouts;
    set_troop_transports(&mut fleet, draft.transports);
    fleet.ships.etacs = draft.etacs;
    recompute_max_speed(&mut fleet);
    normalize_fleet_roe_for_composition(&mut fleet);

    let player = game_data.players.get_mut(player_index_1_based - 1).ok_or(
        GameStateMutationError::MissingPlayerRecord {
            index_1_based: player_index_1_based,
        },
    )?;
    let fleet_record_index_1_based =
        append_fleet_to_owner_chain(&mut game_data.fleets, player, fleet);
    Ok(CommissionResult::Fleet {
        fleet_record_index_1_based,
    })
}
