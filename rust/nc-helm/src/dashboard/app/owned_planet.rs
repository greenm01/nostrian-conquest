use nc_data::{
    AutoCommissionEntry, AutoCommissionFleetEntry, AutoCommissionStarbaseEntry, CommissionResult,
    EmpirePlanetEconomyRow, Planet,
};
use nc_engine::{
    ArmyTransportMode, PlanetCommissionSlotEntry, planet_commission_slot_entries,
    resolve_planet_transport_fleet_selection, transport_fleet_candidates_for_planet,
};

use super::state::{ActiveMouseGesture, ActivePopup, DashApp, OwnedPlanetPopupMode};
use crate::dashboard::overlays::planet_list;

impl DashApp {
    pub(crate) fn open_owned_planet_popup(&mut self, planet_record_index_1_based: usize) {
        self.popup_position = None;
        self.mouse_gesture = ActiveMouseGesture::None;
        self.owned_planet_popup.reset();
        self.popup = ActivePopup::OwnedPlanet {
            planet_record_index_1_based,
        };
    }

    pub(crate) fn open_selected_planet_status_popup(&mut self) {
        let Some(record) = self.selected_planet_overlay_record_index_1_based() else {
            return;
        };
        self.open_owned_planet_popup(record);
    }

    pub(crate) fn open_selected_planet_scorch_confirm(&mut self) {
        let Some(record) = self.selected_planet_overlay_record_index_1_based() else {
            return;
        };
        self.open_owned_planet_popup(record);
        self.open_owned_planet_scorch_confirm();
    }

    pub(crate) fn close_owned_planet_popup(&mut self) {
        self.popup = ActivePopup::None;
        self.popup_position = None;
        self.mouse_gesture = ActiveMouseGesture::None;
        self.owned_planet_popup.reset();
    }

    pub(crate) fn owned_planet_popup_record_index_1_based(&self) -> Option<usize> {
        match self.popup {
            ActivePopup::OwnedPlanet {
                planet_record_index_1_based,
            } => Some(planet_record_index_1_based),
            _ => None,
        }
    }

    pub(crate) fn set_owned_planet_popup_mode(&mut self, mode: OwnedPlanetPopupMode) {
        self.owned_planet_popup.mode = mode;
        self.owned_planet_popup.input.clear();
        self.owned_planet_popup.default.clear();
        self.owned_planet_popup.status = None;
        self.owned_planet_popup
            .transport_selected_fleet_record_index_1_based = None;
        self.owned_planet_popup.transport_selected_fleet_number = None;
        self.owned_planet_popup.transport_available_qty = 0;
        if !matches!(
            mode,
            OwnedPlanetPopupMode::CommissionResult | OwnedPlanetPopupMode::MassCommissionReport
        ) {
            self.owned_planet_popup.report_lines.clear();
        }
    }

    pub(crate) fn owned_planet_record(&self) -> Option<&Planet> {
        let record = self.owned_planet_popup_record_index_1_based()?;
        self.game_data.planets.get(record.saturating_sub(1))
    }

    pub(crate) fn owned_planet_row(&self) -> Option<EmpirePlanetEconomyRow> {
        let record = self.owned_planet_popup_record_index_1_based()?;
        self.game_data
            .empire_planet_economy_rows(self.player_record_index_1_based)
            .into_iter()
            .find(|row| row.planet_record_index_1_based == record)
    }

    pub(crate) fn planet_overlay_action_planet_record_index_1_based(&self) -> Option<usize> {
        self.planet_overlay
            .action_planet_record_index_1_based
            .or_else(|| self.selected_planet_overlay_record_index_1_based())
    }

    pub(crate) fn planet_overlay_action_planet_row(&self) -> Option<EmpirePlanetEconomyRow> {
        let record = self.planet_overlay_action_planet_record_index_1_based()?;
        self.game_data
            .empire_planet_economy_rows(self.player_record_index_1_based)
            .into_iter()
            .find(|row| row.planet_record_index_1_based == record)
    }

    pub(crate) fn open_owned_planet_build_specify(&mut self) {
        let Some(record) = self.owned_planet_popup_record_index_1_based() else {
            self.close_owned_planet_popup();
            return;
        };
        self.owned_planet_popup.status = None;
        self.open_planet_build_specify_for_record(record);
    }

    pub(crate) fn append_owned_planet_input_char(&mut self, ch: char) {
        self.owned_planet_popup.input.push(ch);
        self.owned_planet_popup.status = None;
    }

    pub(crate) fn backspace_owned_planet_input(&mut self) {
        self.owned_planet_popup.input.pop();
        self.owned_planet_popup.status = None;
    }

    pub(crate) fn open_owned_planet_commission_select(&mut self) {
        if self.owned_planet_commission_entries().is_empty() {
            self.show_owned_planet_status("Stardock empty");
            return;
        }
        self.set_owned_planet_popup_mode(OwnedPlanetPopupMode::CommissionSelect);
        if let Some(entry) = self.owned_planet_commission_entries().first() {
            self.owned_planet_popup.default = format!("{:02}", entry.slot_0_based + 1);
        }
    }

    pub(crate) fn submit_owned_planet_commission(
        &mut self,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let raw = if self.owned_planet_popup.input.trim().is_empty() {
            self.owned_planet_popup.default.trim()
        } else {
            self.owned_planet_popup.input.trim()
        };
        let slot = raw
            .parse::<usize>()
            .map_err(|_| "Enter a valid slot number.")?;
        let Some(slot_0_based) = slot.checked_sub(1) else {
            return Err("Slot numbers are 1-based.".into());
        };
        let Some(record) = self.owned_planet_popup_record_index_1_based() else {
            self.close_owned_planet_popup();
            return Ok(());
        };
        let result = self.game_data.commission_planet_stardock_slot(
            self.player_record_index_1_based,
            record,
            slot_0_based,
        )?;
        self.stage_hosted_planet_commission(record, slot_0_based);
        self.save_and_refresh_runtime()?;
        self.owned_planet_popup.report_lines = vec![self.format_commission_result_line(result)];
        self.owned_planet_popup.mode = OwnedPlanetPopupMode::CommissionResult;
        self.owned_planet_popup.input.clear();
        self.owned_planet_popup.default.clear();
        self.owned_planet_popup.status = None;
        Ok(())
    }

    pub(crate) fn open_planet_overlay_commission_select(&mut self) {
        let Some(record) = self.selected_planet_overlay_record_index_1_based() else {
            return;
        };
        let entries = self.planet_overlay_commission_entries_for_record(record);
        let Some(entry) = entries.first() else {
            self.show_planet_overlay_footer_notice("Stardock empty");
            return;
        };
        self.planet_overlay.clear_prompt();
        self.clear_planet_overlay_footer_notice();
        self.planet_overlay.action_planet_record_index_1_based = Some(record);
        self.planet_overlay.prompt_default = format!("{:02}", entry.slot_0_based + 1);
        self.planet_overlay
            .open_prompt(super::state::PlanetOverlayPromptMode::CommissionSelect);
    }

    pub(crate) fn submit_planet_overlay_commission(
        &mut self,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let raw = prompt_raw_value(
            &self.planet_overlay.prompt_input,
            &self.planet_overlay.prompt_default,
        );
        let slot = raw
            .parse::<usize>()
            .map_err(|_| "Enter a valid slot number.")?;
        let Some(slot_0_based) = slot.checked_sub(1) else {
            return Err("Slot numbers are 1-based.".into());
        };
        let Some(record) = self.planet_overlay_action_planet_record_index_1_based() else {
            self.planet_overlay.clear_prompt();
            return Ok(());
        };
        let result = self.game_data.commission_planet_stardock_slot(
            self.player_record_index_1_based,
            record,
            slot_0_based,
        )?;
        self.stage_hosted_planet_commission(record, slot_0_based);
        self.save_and_refresh_runtime()?;
        let message = self.format_commission_result_line(result);
        self.close_planet_overlay_action_with_notice(message);
        Ok(())
    }

    pub(crate) fn open_owned_planet_mass_commission_confirm(&mut self) {
        if !self.any_owned_planet_commission_entries() {
            self.show_owned_planet_status("Stardock empty");
            return;
        }
        self.set_owned_planet_popup_mode(OwnedPlanetPopupMode::MassCommissionConfirm);
    }

    pub(crate) fn confirm_owned_planet_mass_commission(
        &mut self,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let report = self
            .game_data
            .auto_commission_all_stardock_units(self.player_record_index_1_based)?;
        let Some(record) = self.owned_planet_popup_record_index_1_based() else {
            self.close_owned_planet_popup();
            return Ok(());
        };
        self.stage_hosted_planet_auto_commission(record);
        self.save_and_refresh_runtime()?;
        let lines = self.format_auto_commission_report_lines(&report);
        if lines.is_empty() {
            self.set_owned_planet_popup_mode(OwnedPlanetPopupMode::Browse);
            self.show_owned_planet_status("Nothing was available to commission.");
            return Ok(());
        }
        self.owned_planet_popup.report_lines = lines;
        self.owned_planet_popup.mode = OwnedPlanetPopupMode::MassCommissionReport;
        Ok(())
    }

    pub(crate) fn open_planet_overlay_mass_commission_confirm(&mut self) {
        let Some(record) = self.selected_planet_overlay_record_index_1_based() else {
            return;
        };
        if !self.any_owned_planet_commission_entries() {
            self.show_planet_overlay_footer_notice("Stardocks empty");
            return;
        }
        self.planet_overlay.clear_prompt();
        self.clear_planet_overlay_footer_notice();
        self.planet_overlay.action_planet_record_index_1_based = Some(record);
        self.planet_overlay
            .open_prompt(super::state::PlanetOverlayPromptMode::MassCommissionConfirm);
    }

    pub(crate) fn confirm_planet_overlay_mass_commission(
        &mut self,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let report = self
            .game_data
            .auto_commission_all_stardock_units(self.player_record_index_1_based)?;
        let Some(record) = self.planet_overlay_action_planet_record_index_1_based() else {
            self.planet_overlay.clear_prompt();
            return Ok(());
        };
        self.stage_hosted_planet_auto_commission(record);
        self.save_and_refresh_runtime()?;
        if report.entries.is_empty() {
            self.close_planet_overlay_action_with_notice("Nothing was available to commission.");
            return Ok(());
        }
        self.close_planet_overlay_action_with_notice(format!(
            "Mass commissioned {} ships and {} starbases.",
            report.ships_commissioned, report.starbases_commissioned
        ));
        Ok(())
    }

    pub(crate) fn open_owned_planet_transport_fleet_select(&mut self, mode: ArmyTransportMode) {
        let Some(planet) = self.owned_planet_row() else {
            self.close_owned_planet_popup();
            return;
        };
        let candidates = transport_fleet_candidates_for_planet(
            &self.game_data,
            self.player_record_index_1_based as u8,
            mode,
            &planet,
        )
        .into_iter()
        .filter(|fleet| fleet.available_qty > 0)
        .collect::<Vec<_>>();
        let Some(first) = candidates.first() else {
            self.show_owned_planet_status(match mode {
                ArmyTransportMode::Load => "No TT available",
                ArmyTransportMode::Unload => "No TT* available",
            });
            return;
        };
        self.set_owned_planet_popup_mode(OwnedPlanetPopupMode::TransportFleetSelect { mode });
        self.owned_planet_popup.default = first.fleet_number.to_string();
    }

    pub(crate) fn open_planet_overlay_transport_fleet_select(&mut self, mode: ArmyTransportMode) {
        let Some(record) = self.selected_planet_overlay_record_index_1_based() else {
            return;
        };
        let Some(planet) = self
            .game_data
            .empire_planet_economy_rows(self.player_record_index_1_based)
            .into_iter()
            .find(|row| row.planet_record_index_1_based == record)
        else {
            return;
        };
        let candidates = transport_fleet_candidates_for_planet(
            &self.game_data,
            self.player_record_index_1_based as u8,
            mode,
            &planet,
        )
        .into_iter()
        .filter(|fleet| fleet.available_qty > 0)
        .collect::<Vec<_>>();
        let Some(first) = candidates.first() else {
            self.show_planet_overlay_footer_notice(match mode {
                ArmyTransportMode::Load => "No TT available",
                ArmyTransportMode::Unload => "No TT* available",
            });
            return;
        };
        self.planet_overlay.clear_prompt();
        self.clear_planet_overlay_footer_notice();
        self.planet_overlay.action_planet_record_index_1_based = Some(record);
        self.planet_overlay.prompt_default = first.fleet_number.to_string();
        self.planet_overlay
            .open_prompt(super::state::PlanetOverlayPromptMode::TransportFleetSelect { mode });
    }

    pub(crate) fn submit_planet_overlay_transport_fleet(
        &mut self,
        mode: ArmyTransportMode,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let raw = prompt_raw_value(
            &self.planet_overlay.prompt_input,
            &self.planet_overlay.prompt_default,
        );
        let fleet_number = raw
            .parse::<u16>()
            .map_err(|_| "Enter one of your fleet numbers.")?;
        let Some(planet) = self.planet_overlay_action_planet_row() else {
            self.planet_overlay.clear_prompt();
            return Ok(());
        };
        let Some(fleet) = transport_fleet_candidates_for_planet(
            &self.game_data,
            self.player_record_index_1_based as u8,
            mode,
            &planet,
        )
        .into_iter()
        .filter(|fleet| fleet.available_qty > 0)
        .find(|fleet| fleet.fleet_number == fleet_number) else {
            return Err("Enter one of your fleet numbers.".into());
        };
        self.planet_overlay.prompt_mode =
            super::state::PlanetOverlayPromptMode::TransportQuantity { mode };
        self.planet_overlay.prompt_input.clear();
        self.planet_overlay.prompt_default = fleet.available_qty.to_string();
        self.planet_overlay.prompt_status = None;
        self.planet_overlay
            .transport_selected_fleet_record_index_1_based = Some(fleet.fleet_record_index_1_based);
        self.planet_overlay.transport_selected_fleet_number = Some(fleet.fleet_number);
        self.planet_overlay.transport_available_qty = fleet.available_qty;
        Ok(())
    }

    pub(crate) fn submit_planet_overlay_transport_quantity(
        &mut self,
        mode: ArmyTransportMode,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let Some(planet_record_index_1_based) =
            self.planet_overlay_action_planet_record_index_1_based()
        else {
            self.planet_overlay.clear_prompt();
            return Ok(());
        };
        let Some(fleet_record_index_1_based) = self
            .planet_overlay
            .transport_selected_fleet_record_index_1_based
        else {
            self.planet_overlay.clear_prompt();
            return Ok(());
        };
        let qty = if self.planet_overlay.prompt_input.trim().is_empty() {
            self.planet_overlay.transport_available_qty
        } else {
            self.planet_overlay
                .prompt_input
                .trim()
                .parse::<u16>()
                .map_err(|_| "Enter a valid quantity.")?
        };
        if qty == 0 {
            self.planet_overlay.clear_prompt();
            return Ok(());
        }
        if qty > self.planet_overlay.transport_available_qty {
            return Err(format!(
                "Enter a quantity from 0 to {}.",
                self.planet_overlay.transport_available_qty
            )
            .into());
        }
        match mode {
            ArmyTransportMode::Load => {
                self.game_data.load_planet_armies_onto_fleet(
                    self.player_record_index_1_based,
                    planet_record_index_1_based,
                    fleet_record_index_1_based,
                    qty,
                )?;
                self.stage_hosted_fleet_load_armies(
                    fleet_record_index_1_based,
                    planet_record_index_1_based,
                    qty,
                );
            }
            ArmyTransportMode::Unload => {
                self.game_data.unload_fleet_armies_to_planet(
                    self.player_record_index_1_based,
                    planet_record_index_1_based,
                    fleet_record_index_1_based,
                    qty,
                )?;
                self.stage_hosted_fleet_unload_armies(
                    fleet_record_index_1_based,
                    planet_record_index_1_based,
                    qty,
                );
            }
        }
        self.save_and_refresh_runtime()?;
        let fleet_number = self
            .planet_overlay
            .transport_selected_fleet_number
            .unwrap_or_default();
        self.close_planet_overlay_action_with_notice(match mode {
            ArmyTransportMode::Load => {
                format!("Loaded {qty} armies onto Fleet {fleet_number:02}.")
            }
            ArmyTransportMode::Unload => {
                format!("Unloaded {qty} armies from Fleet {fleet_number:02}.")
            }
        });
        Ok(())
    }

    pub(crate) fn submit_owned_planet_transport_fleet(
        &mut self,
        mode: ArmyTransportMode,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let raw = if self.owned_planet_popup.input.trim().is_empty() {
            self.owned_planet_popup.default.trim()
        } else {
            self.owned_planet_popup.input.trim()
        };
        let fleet_number = raw
            .parse::<u16>()
            .map_err(|_| "Enter one of your fleet numbers.")?;
        let owned_rows = self
            .game_data
            .empire_planet_economy_rows(self.player_record_index_1_based);
        let (fleet, _planet) = resolve_planet_transport_fleet_selection(
            &self.game_data,
            self.player_record_index_1_based as u8,
            mode,
            fleet_number,
            &owned_rows,
        )
        .map_err(|err| err.message())?;
        self.owned_planet_popup.mode = OwnedPlanetPopupMode::TransportQuantity { mode };
        self.owned_planet_popup.input.clear();
        self.owned_planet_popup.default = fleet.available_qty.to_string();
        self.owned_planet_popup.status = None;
        self.owned_planet_popup
            .transport_selected_fleet_record_index_1_based = Some(fleet.fleet_record_index_1_based);
        self.owned_planet_popup.transport_selected_fleet_number = Some(fleet.fleet_number);
        self.owned_planet_popup.transport_available_qty = fleet.available_qty;
        Ok(())
    }

    pub(crate) fn submit_owned_planet_transport_quantity(
        &mut self,
        mode: ArmyTransportMode,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let Some(planet_record_index_1_based) = self.owned_planet_popup_record_index_1_based()
        else {
            self.close_owned_planet_popup();
            return Ok(());
        };
        let Some(fleet_record_index_1_based) = self
            .owned_planet_popup
            .transport_selected_fleet_record_index_1_based
        else {
            self.set_owned_planet_popup_mode(OwnedPlanetPopupMode::Browse);
            return Ok(());
        };
        let qty = if self.owned_planet_popup.input.trim().is_empty() {
            self.owned_planet_popup.transport_available_qty
        } else {
            self.owned_planet_popup
                .input
                .trim()
                .parse::<u16>()
                .map_err(|_| "Enter a valid quantity.")?
        };
        if qty == 0 {
            self.set_owned_planet_popup_mode(OwnedPlanetPopupMode::Browse);
            return Ok(());
        }
        if qty > self.owned_planet_popup.transport_available_qty {
            return Err(format!(
                "Enter a quantity from 0 to {}.",
                self.owned_planet_popup.transport_available_qty
            )
            .into());
        }
        match mode {
            ArmyTransportMode::Load => {
                self.game_data.load_planet_armies_onto_fleet(
                    self.player_record_index_1_based,
                    planet_record_index_1_based,
                    fleet_record_index_1_based,
                    qty,
                )?;
                self.stage_hosted_fleet_load_armies(
                    fleet_record_index_1_based,
                    planet_record_index_1_based,
                    qty,
                );
            }
            ArmyTransportMode::Unload => {
                self.game_data.unload_fleet_armies_to_planet(
                    self.player_record_index_1_based,
                    planet_record_index_1_based,
                    fleet_record_index_1_based,
                    qty,
                )?;
                self.stage_hosted_fleet_unload_armies(
                    fleet_record_index_1_based,
                    planet_record_index_1_based,
                    qty,
                );
            }
        }
        self.save_and_refresh_runtime()?;
        let fleet_number = self
            .owned_planet_popup
            .transport_selected_fleet_number
            .unwrap_or_default();
        self.set_owned_planet_popup_mode(OwnedPlanetPopupMode::Browse);
        self.show_owned_planet_status(match mode {
            ArmyTransportMode::Load => {
                format!("Loaded {qty} armies onto Fleet {fleet_number:02}.")
            }
            ArmyTransportMode::Unload => {
                format!("Unloaded {qty} armies from Fleet {fleet_number:02}.")
            }
        });
        Ok(())
    }

    pub(crate) fn open_owned_planet_scorch_confirm(&mut self) {
        if self.owned_planet_popup_record_index_1_based().is_none() {
            self.close_owned_planet_popup();
            return;
        }
        self.set_owned_planet_popup_mode(OwnedPlanetPopupMode::ScorchConfirm1);
    }

    pub(crate) fn submit_owned_planet_scorch(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let Some(record) = self.owned_planet_popup_record_index_1_based() else {
            self.close_owned_planet_popup();
            return Ok(());
        };
        let scorched = self
            .game_data
            .planets
            .get(record.saturating_sub(1))
            .is_some_and(|planet| planet.scorch_earth);
        match self.owned_planet_popup.mode {
            // One confirmation rescinds the order the same command set.
            OwnedPlanetPopupMode::ScorchConfirm1 if scorched => {
                self.finish_owned_planet_scorch(record, false)?;
            }
            OwnedPlanetPopupMode::ScorchConfirm1 => {
                self.owned_planet_popup.mode = OwnedPlanetPopupMode::ScorchConfirm2;
            }
            OwnedPlanetPopupMode::ScorchConfirm2 => {
                self.owned_planet_popup.mode = OwnedPlanetPopupMode::ScorchConfirm3;
            }
            OwnedPlanetPopupMode::ScorchConfirm3 => {
                self.finish_owned_planet_scorch(record, true)?;
            }
            _ => {}
        }
        Ok(())
    }

    /// Set or rescind the planet's Scorch-Earth order and say so.
    fn finish_owned_planet_scorch(
        &mut self,
        record: usize,
        scorched: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let planet_name = self
            .game_data
            .planets
            .get(record.saturating_sub(1))
            .map(|planet| planet.name.trim().to_string())
            .unwrap_or_else(|| "Planet".to_string());
        self.game_data.set_planet_scorch_earth(record, scorched)?;
        self.stage_hosted_planet_scorch(record, scorched);
        self.save_and_refresh_runtime()?;
        self.set_owned_planet_popup_mode(OwnedPlanetPopupMode::Browse);
        self.show_owned_planet_status(if scorched {
            format!("Planet \"{planet_name}\" will be scorched at maintenance.")
        } else {
            format!("Scorch-Earth rescinded on planet \"{planet_name}\".")
        });
        Ok(())
    }

    pub(crate) fn owned_planet_commission_entries(&self) -> Vec<PlanetCommissionSlotEntry> {
        self.owned_planet_record()
            .map(planet_commission_slot_entries)
            .unwrap_or_default()
    }

    fn any_owned_planet_commission_entries(&self) -> bool {
        self.game_data
            .planets
            .iter()
            .filter(|planet| planet.owner.0 as usize == self.player_record_index_1_based)
            .any(|planet| !planet_commission_slot_entries(planet).is_empty())
    }

    pub(crate) fn show_owned_planet_status(&mut self, message: impl Into<String>) {
        self.owned_planet_popup.mode = OwnedPlanetPopupMode::Browse;
        self.owned_planet_popup.status = Some(message.into());
        self.owned_planet_popup.input.clear();
        self.owned_planet_popup.default.clear();
    }

    pub(crate) fn close_planet_overlay_action_with_notice(&mut self, message: impl Into<String>) {
        self.planet_overlay.clear_prompt();
        self.show_planet_overlay_footer_notice(message);
    }

    fn format_commission_result_line(&self, result: CommissionResult) -> String {
        match result {
            CommissionResult::Fleet {
                fleet_record_index_1_based,
            } => {
                let fleet_number = self
                    .game_data
                    .fleets
                    .get(fleet_record_index_1_based.saturating_sub(1))
                    .map(|fleet| fleet.fleet_number.0)
                    .unwrap_or_default();
                format!("Commissioned Fleet #{fleet_number:02}.")
            }
            CommissionResult::Starbase {
                base_record_index_1_based,
            } => {
                let base_number = self
                    .game_data
                    .bases
                    .get(base_record_index_1_based.saturating_sub(1))
                    .map(|base| base.base_id.0)
                    .unwrap_or_default();
                format!("Commissioned Starbase #{base_number:02}.")
            }
        }
    }

    fn selected_planet_overlay_record_index_1_based(&self) -> Option<usize> {
        let rows = planet_list::table_rows(self);
        let selected = self
            .planet_overlay
            .selected
            .min(rows.len().saturating_sub(1));
        rows.get(selected)
            .map(|row| row.planet_record_index_1_based)
    }

    fn planet_overlay_commission_entries_for_record(
        &self,
        record: usize,
    ) -> Vec<PlanetCommissionSlotEntry> {
        self.game_data
            .planets
            .get(record.saturating_sub(1))
            .map(planet_commission_slot_entries)
            .unwrap_or_default()
    }

    fn format_auto_commission_report_lines(
        &self,
        report: &nc_data::AutoCommissionReport,
    ) -> Vec<String> {
        let max_fleet_number = self
            .game_data
            .fleets
            .iter()
            .filter(|fleet| fleet.owner.0 as usize == self.player_record_index_1_based)
            .map(|fleet| fleet.fleet_number.0)
            .max()
            .unwrap_or(1);
        let mut lines = Vec::new();
        for (idx, entry) in report.entries.iter().enumerate() {
            if idx > 0 {
                lines.push(String::new());
            }
            lines.push(match entry {
                AutoCommissionEntry::Fleet(entry) => {
                    format_auto_commission_fleet_entry(entry, max_fleet_number)
                }
                AutoCommissionEntry::Starbase(entry) => {
                    format_auto_commission_starbase_entry(entry)
                }
            });
        }
        lines
    }
}

fn format_auto_commission_fleet_entry(
    entry: &AutoCommissionFleetEntry,
    max_fleet_number: u16,
) -> String {
    let width = max_fleet_number.max(1).to_string().len().max(2);
    format!(
        "Fleet {:0width$} commissioned from \"{}\" in sector ({:02},{:02}) with {}.",
        entry.fleet_number,
        entry.planet_name,
        entry.coords[0],
        entry.coords[1],
        nc_data::auto_commission_notation(entry),
        width = width,
    )
}

fn format_auto_commission_starbase_entry(entry: &AutoCommissionStarbaseEntry) -> String {
    format!(
        "Starbase {:02} commissioned to \"{}\" in sector ({:02},{:02}).",
        entry.starbase_number, entry.planet_name, entry.coords[0], entry.coords[1]
    )
}

fn prompt_raw_value<'a>(input: &'a str, default: &'a str) -> &'a str {
    if input.trim().is_empty() {
        default.trim()
    } else {
        input.trim()
    }
}
