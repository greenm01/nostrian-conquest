use super::support::*;
use super::*;
use crate::PlanetRecord;

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
        let planet = self
            .planets
            .records
            .get(planet_record_index_1_based - 1)
            .ok_or(GameStateMutationError::MissingPlanetRecord {
                index_1_based: planet_record_index_1_based,
            })?;
        if planet.owner_empire_slot_raw() != owner_empire {
            return Err(GameStateMutationError::PlanetOwnershipMismatch {
                player_index_1_based,
                planet_index_1_based: planet_record_index_1_based,
            });
        }
        let coords = planet.coords_raw();
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
            if self.player.records.get(player_index_1_based - 1).is_none() {
                return Err(GameStateMutationError::MissingPlayerRecord {
                    index_1_based: player_index_1_based,
                });
            }
            let record_idx = self.bases.records.len();
            self.bases.records.push(build_guard_starbase_base_record(
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
                .records
                .get_mut(planet_record_index_1_based - 1)
                .ok_or(GameStateMutationError::MissingPlanetRecord {
                    index_1_based: planet_record_index_1_based,
                })?;
            // The starbase slot holds every docked base; one leaves.
            for (slot_0_based, _, _) in selected {
                let left = planet.stardock_count(slot_0_based).saturating_sub(1);
                planet.set_stardock_count(slot_0_based, left);
            }
            Ok(CommissionResult::Starbase {
                base_record_index_1_based: self.bases.records.len(),
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
        let planet = self
            .planets
            .records
            .get(planet_record_index_1_based - 1)
            .ok_or(GameStateMutationError::MissingPlanetRecord {
                index_1_based: planet_record_index_1_based,
            })?;
        if planet.owner_empire_slot_raw() != owner_empire {
            return Err(GameStateMutationError::PlanetOwnershipMismatch {
                player_index_1_based,
                planet_index_1_based: planet_record_index_1_based,
            });
        }

        let coords = planet.coords_raw();
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
            .records
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
            .records
            .iter()
            .enumerate()
            .filter(|(_, planet)| planet.owner_empire_slot_raw() == owner_empire)
            .map(|(idx, _)| idx + 1)
            .collect();

        for planet_index_1_based in planet_indices {
            let Some(planet) = self.planets.records.get(planet_index_1_based - 1) else {
                return Err(GameStateMutationError::MissingPlanetRecord {
                    index_1_based: planet_index_1_based,
                });
            };
            let planet_name = planet.planet_name();
            let coords = planet.coords_raw();
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
                let count = u32::from(planet.stardock_count_raw(slot));
                if count == 0 {
                    continue;
                }
                match ProductionItemKind::from_raw(planet.stardock_kind_raw(slot)) {
                    ProductionItemKind::Destroyer
                    | ProductionItemKind::Cruiser
                    | ProductionItemKind::Battleship
                    | ProductionItemKind::Scout
                    | ProductionItemKind::Transport
                    | ProductionItemKind::Etac => {
                        ship_slots.push(slot);
                        ship_count = ship_count.saturating_add(count);
                        match ProductionItemKind::from_raw(planet.stardock_kind_raw(slot)) {
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
                            .records
                            .get(fleet_record_index_1_based - 1)
                            .ok_or(GameStateMutationError::MissingFleetRecord {
                                index_1_based: fleet_record_index_1_based,
                            })?
                            .local_slot_word_raw();
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
                            .records
                            .get(base_record_index_1_based - 1)
                            .ok_or(GameStateMutationError::MissingBaseRecord {
                                index_1_based: base_record_index_1_based,
                            })?
                            .base_id_raw();
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
        let player = self
            .player
            .records
            .get_mut(player_index_1_based - 1)
            .ok_or(GameStateMutationError::MissingPlayerRecord {
                index_1_based: player_index_1_based,
            })?;
        player.set_first_base_record_raw(1);

        let fleet = self.fleets.records.get_mut(fleet_index_1_based - 1).ok_or(
            GameStateMutationError::MissingFleetRecord {
                index_1_based: fleet_index_1_based,
            },
        )?;
        fleet.set_standing_order_kind(Order::GuardStarbase);
        fleet.set_standing_order_target_coords_raw(target);
        fleet.set_mission_target(1, base_id);

        self.bases = BaseDat {
            records: vec![build_guard_starbase_base_record(
                target,
                base_id,
                fleet.local_slot_word_raw(),
                fleet.fleet_id_word_raw(),
                owner_empire,
                fleet.x_fraction(),
                fleet.y_fraction(),
                fleet.depth(),
            )],
        };

        Ok(())
    }

    pub fn set_ipbm_zero_records(&mut self, count: u16) {
        if let Some(player1) = self.player.records.first_mut() {
            player1.set_ipbm_count_raw(count);
        }

        self.ipbm = IpbmDat {
            records: (0..count)
                .map(|_| IpbmRecord {
                    raw: [0u8; IPBM_RECORD_SIZE],
                })
                .collect(),
        };
    }

    pub fn set_ipbm_record_prefix(
        &mut self,
        record_index_1_based: usize,
        primary: u16,
        owner: u8,
        gate: u16,
        follow_on: u16,
    ) -> Result<(), GameStateMutationError> {
        let record = self.ipbm.records.get_mut(record_index_1_based - 1).ok_or(
            GameStateMutationError::MissingIpbmRecord {
                index_1_based: record_index_1_based,
            },
        )?;
        record.set_primary_word_raw(primary);
        record.set_owner_empire_raw(owner);
        record.set_gate_word_raw(gate);
        record.set_follow_on_word_raw(follow_on);
        Ok(())
    }
}

type SelectedStardockSlot = (usize, u8, u16);

fn collect_selected_stardock_slots(
    planet: &PlanetRecord,
    planet_index_1_based: usize,
    slot_0_based_list: &[usize],
) -> Result<Vec<SelectedStardockSlot>, GameStateMutationError> {
    let mut selected = Vec::with_capacity(slot_0_based_list.len());
    for &slot_0_based in slot_0_based_list {
        if slot_0_based >= crate::STARDOCK_SLOT_COUNT
            || planet.stardock_kind_raw(slot_0_based) == 0
            || planet.stardock_count_raw(slot_0_based) == 0
        {
            return Err(GameStateMutationError::EmptyStardockSlot {
                planet_index_1_based,
                slot_0_based,
            });
        }
        selected.push((
            slot_0_based,
            planet.stardock_kind_raw(slot_0_based),
            planet.stardock_count_raw(slot_0_based),
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
    planet: &mut PlanetRecord,
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
        let available = planet.stardock_count_raw(*slot_0_based);
        let consumed = available.min(*remaining_for_kind);
        let new_count = available.saturating_sub(consumed);
        planet.set_stardock_count_raw(*slot_0_based, new_count);
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
    let fleet_id = next_available_global_fleet_id(&game_data.fleets.records);
    let local_slot = next_available_owned_fleet_local_slot(&game_data.fleets.records, owner_empire);

    let mut fleet = FleetRecord::new_zeroed();
    fleet.set_local_slot_word_raw(local_slot);
    fleet.set_owner_empire_raw(owner_empire);
    fleet.set_fleet_id_word_raw(fleet_id);
    fleet.set_current_speed(0);
    fleet.set_current_location_coords_raw(coords);
    fleet.set_tuple_a_payload_raw([0x80, 0, 0, 0, 0]);
    fleet.set_tuple_b_payload_raw([0x80, 0, 0, 0, 0]);
    fleet.set_tuple_c_payload_raw([0x81, 0, 0, 0, 0]);
    fleet.set_standing_order_kind(Order::HoldPosition);
    fleet.set_standing_order_target_coords_raw(coords);
    fleet.set_mission_target(0, 0);
    fleet.set_rules_of_engagement(6);
    fleet.set_destroyer_count(draft.destroyers);
    fleet.set_cruiser_count(draft.cruisers);
    fleet.set_battleship_count(draft.battleships);
    fleet.set_scout_count(draft.scouts);
    fleet.set_troop_transport_count(draft.transports);
    fleet.set_etac_count(draft.etacs);
    fleet.recompute_max_speed_from_composition();
    normalize_fleet_roe_for_composition(&mut fleet);

    let player = game_data
        .player
        .records
        .get_mut(player_index_1_based - 1)
        .ok_or(GameStateMutationError::MissingPlayerRecord {
            index_1_based: player_index_1_based,
        })?;
    let fleet_record_index_1_based =
        append_fleet_to_owner_chain(&mut game_data.fleets.records, player, fleet);
    Ok(CommissionResult::Fleet {
        fleet_record_index_1_based,
    })
}
