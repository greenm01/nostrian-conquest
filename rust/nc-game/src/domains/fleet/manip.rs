use crate::app::state::App;
use crate::domains::fleet::FleetAction;
use crate::domains::fleet::state::{FleetCommandContext, FleetMenuPromptMode};
use crate::screen::layout::PromptFeedback;
use crate::screen::{
    CommandMenu, FleetDetachClass, FleetDetachMode, FleetRow, FleetTransferMode, ScreenId,
};
use nc_data::{
    CoreGameData, Fleet, FleetDetachSelection, NEW_FLEET_DEFAULT_ROE, ShipCategory,
    crippled_left_to_stage, resolve_selection,
};
use nc_engine::{
    fleet_eta_label as engine_fleet_eta_label, fleet_list_eta_label as engine_fleet_list_eta_label,
    fleet_record_supports_mission_code, fleet_target_eta_message, resolve_checked_fleet_merge_plan,
    resolve_checked_fleet_transfer_plan,
};

impl App {
    pub(crate) fn checked_merge_plan(&self) -> Result<nc_engine::CheckedFleetMergePlan, String> {
        resolve_checked_fleet_merge_plan(&self.checked_fleet_refs()).map_err(|err| err.to_string())
    }

    fn checked_transfer_plan(&self) -> Result<nc_engine::CheckedFleetTransferPlan, String> {
        let highlighted = self
            .fleet_list_rows()
            .get(self.fleet.cursor)
            .map(|row| row.fleet_record_index_1_based);
        resolve_checked_fleet_transfer_plan(&self.checked_fleet_refs(), highlighted)
            .map_err(|err| err.to_string())
    }

    pub(crate) fn submit_checked_fleet_merge(
        &mut self,
        plan: nc_engine::CheckedFleetMergePlan,
    ) -> Result<(), String> {
        let checked_rows = self.checked_fleet_rows();
        for source in checked_rows
            .iter()
            .filter(|row| row.fleet_record_index_1_based != plan.host_record_index_1_based)
        {
            self.game_data
                .set_join_fleet_order(
                    self.player.record_index_1_based,
                    source.fleet_record_index_1_based,
                    plan.host_record_index_1_based,
                )
                .map_err(|err| err.to_string())?;
        }
        self.save_game_data().map_err(|err| err.to_string())?;
        self.clear_checked_fleet_selection();
        self.show_fleet_context_success(
            format!(
                "{} fleets ordered to merge into Fleet #{}.",
                checked_rows.len().saturating_sub(1),
                plan.host_fleet_number
            ),
            true,
        );
        Ok(())
    }

    pub(crate) fn submit_inline_fleet_merge(
        &mut self,
        host_fleet_record_index_1_based: usize,
    ) -> Result<(), String> {
        let source_record_index_1_based = self
            .fleet
            .merge_source_record_index_1_based
            .ok_or_else(|| "Select the fleet that will merge first.".to_string())?;
        if source_record_index_1_based == host_fleet_record_index_1_based {
            return Err("Choose a different host fleet.".to_string());
        }
        let source_row = self
            .fleet_row_by_record_index(source_record_index_1_based)
            .ok_or_else(|| "Selected fleet is no longer available.".to_string())?;
        let host_row = self
            .fleet_row_by_record_index(host_fleet_record_index_1_based)
            .ok_or_else(|| "Host fleet is no longer available.".to_string())?;
        if host_row.coords != source_row.coords {
            return Err(format!(
                "Fleet #{} is not in the same sector as Fleet #{}.",
                host_row.fleet_number, source_row.fleet_number
            ));
        }
        let (
            source_record_index_1_based,
            source_fleet_number,
            host_fleet_record_index_1_based,
            host_fleet_number,
        ) = if source_row.fleet_number > host_row.fleet_number {
            (
                source_record_index_1_based,
                source_row.fleet_number,
                host_fleet_record_index_1_based,
                host_row.fleet_number,
            )
        } else {
            (
                host_fleet_record_index_1_based,
                host_row.fleet_number,
                source_record_index_1_based,
                source_row.fleet_number,
            )
        };
        self.game_data
            .set_join_fleet_order(
                self.player.record_index_1_based,
                source_record_index_1_based,
                host_fleet_record_index_1_based,
            )
            .map_err(|err| err.to_string())?;
        self.save_game_data().map_err(|err| err.to_string())?;
        self.fleet.merge_source_record_index_1_based = None;
        self.show_fleet_context_success(
            format!(
                "Fleet #{} ordered to join Fleet #{}.",
                source_fleet_number, host_fleet_number
            ),
            true,
        );
        Ok(())
    }

    pub fn open_fleet_merge(&mut self) {
        if self.current_screen == ScreenId::FleetList {
            if self.fleet_list_has_checked_selection() {
                match self.checked_merge_plan() {
                    Ok(_plan) => {
                        self.fleet.command_context = FleetCommandContext::List;
                        self.clear_command_menu_notice();
                        self.fleet.list_input.clear();
                        self.clear_fleet_list_dismiss_message();
                        self.fleet.menu_prompt_mode =
                            Some(FleetMenuPromptMode::MergeCheckedConfirm);
                        self.fleet.menu_prompt_input.clear();
                        self.fleet.menu_prompt_status = None;
                        self.fleet.menu_prompt_default_value = "Y".to_string();
                        return;
                    }
                    Err(err) => {
                        self.show_fleet_list_dismiss_message(err);
                        return;
                    }
                }
            }
            match self.fleet_selected_list_row() {
                Ok(row) => {
                    if let Err(err) = self.validate_merge_source_row(&row) {
                        let message = if err == "Selected fleet is no longer available." {
                            "Fleet unavailable"
                        } else {
                            "Unable to merge"
                        };
                        self.show_fleet_list_dismiss_message(message);
                        return;
                    }
                    self.fleet.command_context = FleetCommandContext::List;
                    self.fleet.merge_source_record_index_1_based =
                        Some(row.fleet_record_index_1_based);
                    self.open_fleet_menu_prompt(
                        FleetMenuPromptMode::MergeHost,
                        self.merge_host_default_value(),
                    );
                    return;
                }
                Err(err) => {
                    let message = if err == "You have no active fleets." {
                        err
                    } else {
                        "Fleet unavailable".to_string()
                    };
                    self.show_fleet_list_dismiss_message(message);
                    return;
                }
            }
        }
        let Some(default_fleet_number) = self.eligible_merge_source_fleet_number() else {
            self.show_command_menu_notice(
                CommandMenu::Fleet,
                "You need a fleet in the same sector as a lower-numbered one of your fleets to merge.",
            );
            return;
        };
        self.fleet.command_context = FleetCommandContext::Menu;
        self.fleet.merge_source_record_index_1_based = None;
        self.open_fleet_menu_prompt(
            FleetMenuPromptMode::MergeSource,
            default_fleet_number.to_string(),
        );
    }

    pub fn open_fleet_transfer(&mut self) {
        if self.current_screen == ScreenId::FleetList {
            if self.fleet_list_has_checked_selection() {
                match self.checked_transfer_plan() {
                    Ok(plan) => {
                        self.fleet.command_context = FleetCommandContext::List;
                        self.fleet.transfer_status = None;
                        self.fleet.transfer_input.clear();
                        self.fleet.transfer_donor_record_index_1_based =
                            Some(plan.donor_record_index_1_based);
                        self.fleet.transfer_host_record_index_1_based =
                            Some(plan.host_record_index_1_based);
                        self.fleet.transfer_selection = FleetDetachSelection::default();
                        self.clear_command_menu_notice();
                        self.clear_fleet_menu_prompt();
                        self.current_screen = ScreenId::FleetTransfer;
                        return;
                    }
                    Err(err) => {
                        self.show_fleet_list_dismiss_message(err);
                        return;
                    }
                }
            }
            match self.fleet_selected_list_row() {
                Ok(row) => {
                    if let Err(err) = self.validate_transfer_donor_row(&row) {
                        let message = if err == "Use merge instead" {
                            "Use merge instead"
                        } else {
                            "Unable to transfer"
                        };
                        self.show_fleet_list_dismiss_message(message);
                        return;
                    }
                    self.fleet.command_context = FleetCommandContext::List;
                    self.fleet.transfer_status = None;
                    self.fleet.transfer_input.clear();
                    self.fleet.transfer_donor_record_index_1_based =
                        Some(row.fleet_record_index_1_based);
                    self.fleet.transfer_host_record_index_1_based = None;
                    self.fleet.transfer_selection = FleetDetachSelection::default();
                    self.open_fleet_menu_prompt(
                        FleetMenuPromptMode::TransferHost,
                        self.transfer_host_default_value(),
                    );
                    return;
                }
                Err(err) => {
                    let message = if err == "You have no active fleets." {
                        err
                    } else {
                        "Fleet unavailable".to_string()
                    };
                    self.show_fleet_list_dismiss_message(message);
                    return;
                }
            }
        }
        if self.eligible_transfer_donor_fleet_number().is_none() {
            self.show_command_menu_notice(
                CommandMenu::Fleet,
                "You need a fleet with more than one ship in the same sector as another one of your fleets.",
            );
            return;
        }
        self.fleet.command_context = FleetCommandContext::Menu;
        self.fleet.transfer_status = None;
        self.fleet.transfer_input.clear();
        self.fleet.transfer_donor_record_index_1_based = None;
        self.fleet.transfer_host_record_index_1_based = None;
        self.fleet.transfer_selection = FleetDetachSelection::default();
        self.open_fleet_menu_prompt(
            FleetMenuPromptMode::TransferDonor,
            self.eligible_transfer_donor_fleet_number()
                .map(|value| value.to_string())
                .unwrap_or_default(),
        );
    }

    pub fn open_fleet_detach(&mut self) {
        if self.current_screen == ScreenId::FleetList {
            match self.fleet_selected_list_row() {
                Ok(row) => {
                    self.fleet.command_context = FleetCommandContext::List;
                    if let Err(err) =
                        self.open_fleet_detach_with_selected_record(row.fleet_record_index_1_based)
                    {
                        let message = if err == "You have no active fleets." {
                            err
                        } else {
                            "Unable to detach".to_string()
                        };
                        self.show_fleet_list_dismiss_message(message);
                    }
                    return;
                }
                Err(err) => {
                    let message = if err == "You have no active fleets." {
                        err
                    } else {
                        "Fleet unavailable".to_string()
                    };
                    self.show_fleet_list_dismiss_message(message);
                    return;
                }
            }
        }
        if self.fleet_rows().is_empty() {
            self.show_command_menu_notice(CommandMenu::Fleet, "You have no active fleets.");
            return;
        }
        self.fleet.command_context = FleetCommandContext::Menu;
        self.fleet.detach_status = None;
        self.fleet.detach_last_commissioned = None;
        self.fleet.detach_input.clear();
        self.fleet.detach_selection = FleetDetachSelection::default();
        self.fleet.detach_donor_speed = None;
        self.open_fleet_menu_prompt(
            FleetMenuPromptMode::DetachFleet,
            self.largest_owned_fleet_number_by_ship_total()
                .map(|value| value.to_string())
                .unwrap_or_default(),
        );
    }

    pub(crate) fn open_fleet_transfer_with_selected_records(
        &mut self,
        host_fleet_record_index_1_based: usize,
    ) {
        let Some(donor_record_index_1_based) = self.fleet.transfer_donor_record_index_1_based
        else {
            self.fleet.menu_prompt_status =
                Some(PromptFeedback::error("Select a donor fleet first."));
            return;
        };
        self.clear_command_menu_notice();
        self.clear_fleet_menu_prompt();
        self.fleet.transfer_status = None;
        self.fleet.transfer_host_record_index_1_based = Some(host_fleet_record_index_1_based);
        self.fleet.transfer_mode = FleetTransferMode::ChoosingClass;
        self.fleet.transfer_input.clear();
        self.fleet.transfer_selection = FleetDetachSelection::default();
        if donor_record_index_1_based == host_fleet_record_index_1_based {
            self.fleet.transfer_status = Some("Choose a different destination fleet.".to_string());
            self.current_screen = self.fleet_context_screen();
            return;
        }
        self.current_screen = ScreenId::FleetTransfer;
    }

    pub(crate) fn open_fleet_detach_with_selected_record(
        &mut self,
        fleet_record_index_1_based: usize,
    ) -> Result<(), String> {
        let row = self
            .fleet_row_by_record_index(fleet_record_index_1_based)
            .ok_or_else(|| "Selected fleet is no longer available.".to_string())?;
        self.clear_command_menu_notice();
        self.clear_fleet_menu_prompt();
        self.fleet.detach_status = None;
        self.fleet.detach_last_commissioned = None;
        self.fleet.detach_input.clear();
        if self.current_fleet_detach_ship_total(fleet_record_index_1_based) <= 1 {
            return Err(format!(
                "Fleet #{} has only one ship and is not eligible to detach any ships.",
                row.fleet_number
            ));
        }
        self.fleet.detach_donor_record_index_1_based = Some(fleet_record_index_1_based);
        self.reset_fleet_detach_staging();
        self.fleet.detach_mode = FleetDetachMode::ChoosingClass;
        self.current_screen = ScreenId::FleetDetach;
        Ok(())
    }

    pub fn append_fleet_transfer_char(&mut self, ch: char) {
        if self.current_screen != ScreenId::FleetTransfer {
            return;
        }
        match self.fleet.transfer_mode {
            FleetTransferMode::ChoosingClass => {
                if !(ch.is_ascii_alphanumeric() || ch == '*') {
                    return;
                }
                if self.fleet.transfer_input.len() >= CLASS_CODE_MAX_LEN {
                    return;
                }
                self.fleet.transfer_input.push(ch.to_ascii_uppercase());
            }
            FleetTransferMode::Confirming => {
                if ch.eq_ignore_ascii_case(&'n') {
                    self.fleet.transfer_mode = FleetTransferMode::ChoosingClass;
                    self.fleet.transfer_input.clear();
                }
            }
            FleetTransferMode::EnteringQuantity(_) => {
                if !quantity_char_fits(&self.fleet.transfer_input, ch) {
                    return;
                }
                self.fleet.transfer_input.push(ch);
            }
        }
        self.fleet.transfer_status = None;
    }

    pub fn append_fleet_detach_char(&mut self, ch: char) {
        if self.current_screen != ScreenId::FleetDetach {
            return;
        }
        match self.fleet.detach_mode {
            FleetDetachMode::ChoosingClass => {
                if !(ch.is_ascii_alphanumeric() || ch == '*') {
                    return;
                }
                if self.fleet.detach_input.len() >= CLASS_CODE_MAX_LEN {
                    return;
                }
                self.fleet.detach_input.push(ch.to_ascii_uppercase());
            }
            FleetDetachMode::Confirming => {
                if ch.eq_ignore_ascii_case(&'n') {
                    self.fleet.detach_mode = FleetDetachMode::ChoosingClass;
                    self.fleet.detach_input.clear();
                }
            }
            FleetDetachMode::EnteringQuantity(_) => {
                if !quantity_char_fits(&self.fleet.detach_input, ch) {
                    return;
                }
                self.fleet.detach_input.push(ch);
            }
            FleetDetachMode::AdjustingDonorSpeed => {
                if !ch.is_ascii_digit() || self.fleet.detach_input.len() >= 3 {
                    return;
                }
                self.fleet.detach_input.push(ch);
            }
        }
        self.fleet.detach_status = None;
    }

    pub fn backspace_fleet_transfer_input(&mut self) {
        if self.current_screen != ScreenId::FleetTransfer {
            return;
        }
        self.fleet.transfer_input.pop();
        self.fleet.transfer_status = None;
    }

    pub fn cancel_fleet_transfer(&mut self) {
        if self.current_screen != ScreenId::FleetTransfer {
            return;
        }
        self.fleet.transfer_input.clear();
        self.fleet.transfer_status = None;
        match self.fleet.transfer_mode {
            FleetTransferMode::ChoosingClass | FleetTransferMode::Confirming => {
                self.reset_fleet_transfer_staging();
                self.fleet.transfer_donor_record_index_1_based = None;
                self.fleet.transfer_host_record_index_1_based = None;
                self.current_screen = self.fleet_context_screen();
            }
            FleetTransferMode::EnteringQuantity(_) => {
                self.fleet.transfer_mode = FleetTransferMode::ChoosingClass;
            }
        }
    }

    pub fn clear_fleet_transfer_selection(&mut self) {
        if self.current_screen != ScreenId::FleetTransfer {
            return;
        }
        self.reset_fleet_transfer_staging();
        self.fleet.transfer_mode = FleetTransferMode::ChoosingClass;
    }

    pub fn backspace_fleet_detach_input(&mut self) {
        if self.current_screen != ScreenId::FleetDetach {
            return;
        }
        self.fleet.detach_input.pop();
        self.fleet.detach_status = None;
    }

    pub fn cancel_fleet_detach(&mut self) {
        if self.current_screen != ScreenId::FleetDetach {
            return;
        }
        self.fleet.detach_input.clear();
        self.fleet.detach_status = None;
        match self.fleet.detach_mode {
            FleetDetachMode::ChoosingClass | FleetDetachMode::Confirming => {
                self.reset_fleet_detach_staging();
                self.fleet.detach_last_commissioned = None;
                self.fleet.detach_donor_record_index_1_based = None;
                self.current_screen = self.fleet_context_screen();
            }
            FleetDetachMode::EnteringQuantity(_) | FleetDetachMode::AdjustingDonorSpeed => {
                self.fleet.detach_mode = FleetDetachMode::ChoosingClass;
            }
        }
    }

    pub fn clear_fleet_detach_selection(&mut self) {
        if self.current_screen != ScreenId::FleetDetach {
            return;
        }
        self.reset_fleet_detach_staging();
        self.fleet.detach_mode = FleetDetachMode::ChoosingClass;
    }

    pub fn submit_fleet_transfer(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.current_screen != ScreenId::FleetTransfer {
            return Ok(());
        }
        if self.fleet.transfer_donor_record_index_1_based.is_none()
            || self.fleet.transfer_host_record_index_1_based.is_none()
        {
            self.fleet.transfer_status =
                Some("Select transfer fleets from the command menu.".to_string());
            self.open_fleet_transfer();
            return Ok(());
        }
        match self.fleet.transfer_mode {
            FleetTransferMode::ChoosingClass => {
                let raw = self.fleet.transfer_input.trim().to_ascii_uppercase();
                match raw.as_str() {
                    "T" | "" => {
                        if self.fleet.transfer_selection.total_ships() == 0 {
                            self.fleet.transfer_status =
                                Some("Stage at least one ship before transferring.".to_string());
                            return Ok(());
                        }
                        self.fleet.transfer_input.clear();
                        self.fleet.transfer_status = None;
                        self.fleet.transfer_mode = FleetTransferMode::Confirming;
                    }
                    "X" => {
                        self.clear_fleet_transfer_selection();
                    }
                    "Q" => {
                        self.cancel_fleet_transfer();
                    }
                    _ => {
                        let Some(class) = self.parse_fleet_detach_class_code(&raw) else {
                            self.fleet.transfer_status =
                                Some("Choose a listed class, T, X, or Q.".to_string());
                            return Ok(());
                        };
                        if self.fleet_transfer_available_for_class(class) == 0 {
                            self.fleet.transfer_status =
                                Some("That class is not available for transfer.".to_string());
                            return Ok(());
                        }
                        self.fleet.transfer_mode = FleetTransferMode::EnteringQuantity(class);
                        self.fleet.transfer_input.clear();
                        self.fleet.transfer_status = None;
                    }
                }
            }
            FleetTransferMode::EnteringQuantity(class) => {
                let available = self.fleet_transfer_available_for_class(class);
                if available == 0 {
                    self.fleet.transfer_status =
                        Some("That class is not available for transfer.".to_string());
                    self.fleet.transfer_mode = FleetTransferMode::ChoosingClass;
                    self.fleet.transfer_input.clear();
                    return Ok(());
                }
                let default_qty = 1u16.min(available);
                let Some((value, crippled)) =
                    parse_quantity_input(&self.fleet.transfer_input, default_qty)
                else {
                    self.fleet.transfer_status = Some("Enter an integer value.".to_string());
                    return Ok(());
                };
                let Some(donor) = self
                    .fleet
                    .transfer_donor_record_index_1_based
                    .and_then(|idx| self.game_data.fleets.get(idx - 1))
                else {
                    return Ok(());
                };
                let category = ShipCategory::from(class);
                if let Some(message) = staging_error(
                    donor,
                    &self.fleet.transfer_selection,
                    category,
                    value,
                    crippled,
                    available,
                ) {
                    self.fleet.transfer_status = Some(message);
                    return Ok(());
                }
                self.fleet
                    .transfer_selection
                    .stage(category, value, crippled);
                self.fleet.transfer_input.clear();
                self.fleet.transfer_status = None;
                self.fleet.transfer_mode =
                    if self.fleet_transfer_remaining_total_after_selection() == 1 {
                        FleetTransferMode::Confirming
                    } else {
                        FleetTransferMode::ChoosingClass
                    };
            }
            FleetTransferMode::Confirming => {
                self.finish_fleet_transfer()?;
            }
        }
        Ok(())
    }

    pub fn submit_fleet_detach(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.current_screen != ScreenId::FleetDetach {
            return Ok(());
        }
        let Some(donor_record_index_1_based) = self.fleet.detach_donor_record_index_1_based else {
            self.current_screen = self.fleet_context_screen();
            return Ok(());
        };
        let Some(selected_row) = self.fleet_row_by_record_index(donor_record_index_1_based) else {
            self.current_screen = self.fleet_context_screen();
            return Ok(());
        };

        if self
            .game_data
            .fleets
            .get(donor_record_index_1_based - 1)
            .is_none()
        {
            self.current_screen = self.fleet_context_screen();
            return Ok(());
        }

        match self.fleet.detach_mode {
            FleetDetachMode::ChoosingClass => {
                let raw = self.fleet.detach_input.trim().to_ascii_uppercase();
                match raw.as_str() {
                    "D" | "" => {
                        if self.fleet.detach_selection.total_ships() == 0 {
                            self.fleet.detach_status =
                                Some("Stage at least one ship before detaching.".to_string());
                            return Ok(());
                        }
                        self.fleet.detach_input.clear();
                        self.fleet.detach_status = None;
                        self.fleet.detach_mode = FleetDetachMode::Confirming;
                    }
                    "X" => {
                        self.clear_fleet_detach_selection();
                    }
                    "Q" => {
                        self.cancel_fleet_detach();
                    }
                    _ => {
                        let Some(class) = self.parse_fleet_detach_class_code(&raw) else {
                            self.fleet.detach_status =
                                Some("Choose a listed class, D, X, or Q.".to_string());
                            return Ok(());
                        };
                        if self.fleet_detach_available_for_class(class) == 0 {
                            self.fleet.detach_status =
                                Some("That class is not available for detach.".to_string());
                            return Ok(());
                        }
                        self.fleet.detach_mode = FleetDetachMode::EnteringQuantity(class);
                        self.fleet.detach_input.clear();
                        self.fleet.detach_status = None;
                    }
                }
            }
            FleetDetachMode::EnteringQuantity(class) => {
                let available = self.fleet_detach_available_for_class(class);
                if available == 0 {
                    self.fleet.detach_status =
                        Some("That class is not available for detach.".to_string());
                    self.fleet.detach_mode = FleetDetachMode::ChoosingClass;
                    self.fleet.detach_input.clear();
                    return Ok(());
                }
                let default_qty = 1u16.min(available);
                let Some((value, crippled)) =
                    parse_quantity_input(&self.fleet.detach_input, default_qty)
                else {
                    self.fleet.detach_status = Some("Enter an integer value.".to_string());
                    return Ok(());
                };
                let Some(donor) = self.game_data.fleets.get(donor_record_index_1_based - 1) else {
                    return Ok(());
                };
                let category = ShipCategory::from(class);
                if let Some(message) = staging_error(
                    donor,
                    &self.fleet.detach_selection,
                    category,
                    value,
                    crippled,
                    available,
                ) {
                    self.fleet.detach_status = Some(message);
                    return Ok(());
                }
                self.fleet.detach_selection.stage(category, value, crippled);
                self.fleet.detach_input.clear();
                self.fleet.detach_status = None;
                self.fleet.detach_mode = if self.fleet_detach_remaining_total_after_selection() == 1
                {
                    FleetDetachMode::Confirming
                } else {
                    FleetDetachMode::ChoosingClass
                };
            }
            FleetDetachMode::Confirming => {
                self.fleet.detach_input.clear();
                self.fleet.detach_status = None;
                self.fleet.detach_donor_speed = None;
                if self.fleet_detach_requires_speed_prompt() {
                    self.fleet.detach_mode = FleetDetachMode::AdjustingDonorSpeed;
                } else {
                    self.commit_fleet_detach_staged_selection(
                        donor_record_index_1_based,
                        selected_row.fleet_number,
                    )?;
                }
            }
            FleetDetachMode::AdjustingDonorSpeed => {
                let default_speed = self.fleet_detach_donor_default_speed().max(1);
                let speed = self.resolve_fleet_detach_numeric_input(default_speed as u16)? as u8;
                let max_speed = self.fleet_detach_donor_default_speed();
                if speed == 0 || speed > max_speed {
                    self.fleet.detach_status =
                        Some(format!("Enter a speed from 1 to {max_speed}."));
                    return Ok(());
                }
                self.fleet.detach_donor_speed = Some(speed);
                self.fleet.detach_input.clear();
                self.fleet.detach_status = None;
                self.commit_fleet_detach_staged_selection(
                    donor_record_index_1_based,
                    selected_row.fleet_number,
                )?;
            }
        }
        Ok(())
    }

    pub(crate) fn handle_fleet_detach_key(
        &self,
        key: crossterm::event::KeyEvent,
    ) -> crate::app::Action {
        use crossterm::event::KeyCode;

        if self.fleet.detach_mode == FleetDetachMode::Confirming {
            return match key.code {
                KeyCode::Enter | KeyCode::Char('y' | 'Y') => {
                    crate::app::Action::Fleet(FleetAction::SubmitDetach)
                }
                KeyCode::Char('n' | 'N') => {
                    crate::app::Action::Fleet(FleetAction::AppendDetachChar('n'))
                }
                KeyCode::Char('q' | 'Q') | KeyCode::Esc => {
                    crate::app::Action::Fleet(FleetAction::CancelDetach)
                }
                _ => crate::app::Action::Noop,
            };
        }
        match key.code {
            KeyCode::Enter => crate::app::Action::Fleet(FleetAction::SubmitDetach),
            KeyCode::Backspace => crate::app::Action::Fleet(FleetAction::BackspaceDetachInput),
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                crate::app::Action::Fleet(FleetAction::CancelDetach)
            }
            KeyCode::Char('x') | KeyCode::Char('X') => {
                crate::app::Action::Fleet(FleetAction::ClearDetachSelection)
            }
            KeyCode::Char(ch)
                if match self.fleet.detach_mode {
                    FleetDetachMode::ChoosingClass => ch.is_ascii_alphanumeric() || ch == '*',
                    FleetDetachMode::Confirming => false,
                    FleetDetachMode::EnteringQuantity(_) => {
                        quantity_char_fits(&self.fleet.detach_input, ch)
                    }
                    FleetDetachMode::AdjustingDonorSpeed => ch.is_ascii_digit(),
                } =>
            {
                crate::app::Action::Fleet(FleetAction::AppendDetachChar(ch))
            }
            _ => crate::app::Action::Noop,
        }
    }

    pub(crate) fn handle_fleet_transfer_key(
        &self,
        key: crossterm::event::KeyEvent,
    ) -> crate::app::Action {
        use crossterm::event::KeyCode;

        if self.fleet.transfer_mode == FleetTransferMode::Confirming {
            return match key.code {
                KeyCode::Enter | KeyCode::Char('y' | 'Y') => {
                    crate::app::Action::Fleet(FleetAction::SubmitTransfer)
                }
                KeyCode::Char('n' | 'N') => {
                    crate::app::Action::Fleet(FleetAction::AppendTransferChar('n'))
                }
                KeyCode::Char('q' | 'Q') | KeyCode::Esc => {
                    crate::app::Action::Fleet(FleetAction::CancelTransfer)
                }
                _ => crate::app::Action::Noop,
            };
        }
        match key.code {
            KeyCode::Enter => crate::app::Action::Fleet(FleetAction::SubmitTransfer),
            KeyCode::Backspace => crate::app::Action::Fleet(FleetAction::BackspaceTransferInput),
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                crate::app::Action::Fleet(FleetAction::CancelTransfer)
            }
            KeyCode::Char('x') | KeyCode::Char('X') => {
                crate::app::Action::Fleet(FleetAction::ClearTransferSelection)
            }
            KeyCode::Char(ch)
                if match self.fleet.transfer_mode {
                    FleetTransferMode::ChoosingClass => ch.is_ascii_alphanumeric() || ch == '*',
                    FleetTransferMode::Confirming => false,
                    FleetTransferMode::EnteringQuantity(_) => {
                        quantity_char_fits(&self.fleet.transfer_input, ch)
                    }
                } =>
            {
                crate::app::Action::Fleet(FleetAction::AppendTransferChar(ch))
            }
            _ => crate::app::Action::Noop,
        }
    }

    pub(crate) fn fleet_detach_prompt_and_default(&self) -> (String, String) {
        let fleet_number = self
            .fleet
            .detach_donor_record_index_1_based
            .and_then(|idx| self.fleet_number_for_record_index(idx))
            .unwrap_or(1);
        match self.fleet.detach_mode {
            FleetDetachMode::ChoosingClass => (
                class_prompt(
                    ShipCategory::ALL
                        .into_iter()
                        .filter(|class| self.fleet_detach_available_for_class((*class).into()) > 0)
                        .map(ShipCategory::code),
                ),
                String::new(),
            ),
            FleetDetachMode::EnteringQuantity(class) => (
                quantity_prompt(class, self.fleet_detach_available_for_class(class)),
                "1".to_string(),
            ),
            FleetDetachMode::Confirming => (
                "Detach these ships? [Y]/N <Q> -> ".to_string(),
                String::new(),
            ),
            FleetDetachMode::AdjustingDonorSpeed => (
                format!("Fleet #{} new speed ", fleet_number),
                self.fleet_detach_donor_default_speed().to_string(),
            ),
        }
    }

    pub(crate) fn fleet_detach_staged_summary(&self) -> String {
        self.format_fleet_detach_summary_from_selection(
            self.fleet.detach_donor_record_index_1_based,
            &self.fleet.detach_selection,
        )
    }

    pub(crate) fn fleet_detach_remaining_summary(&self) -> String {
        self.fleet_detach_donor_after_selection_record()
            .map(|fleet| self.format_fleet_detach_summary_from_record(&fleet))
            .unwrap_or_else(|| "none".to_string())
    }

    fn current_fleet_detach_ship_total(&self, fleet_record_index_1_based: usize) -> u32 {
        self.game_data
            .fleets
            .get(fleet_record_index_1_based - 1)
            .map(|fleet| {
                u32::from(fleet.ships.battleships)
                    + u32::from(fleet.ships.cruisers)
                    + u32::from(fleet.ships.destroyers)
                    + u32::from(nc_data::troop_transports(&fleet))
                    + u32::from(fleet.ships.scouts)
                    + u32::from(fleet.ships.etacs)
            })
            .unwrap_or(0)
    }

    fn reset_fleet_detach_staging(&mut self) {
        self.fleet.detach_input.clear();
        self.fleet.detach_status = None;
        self.fleet.detach_selection = FleetDetachSelection::default();
        self.fleet.detach_donor_speed = None;
    }

    fn fleet_detach_remaining_total_after_selection(&self) -> u32 {
        let Some(donor_record_index_1_based) = self.fleet.detach_donor_record_index_1_based else {
            return 0;
        };
        self.current_fleet_detach_ship_total(donor_record_index_1_based)
            .saturating_sub(self.fleet.detach_selection.total_ships())
    }

    fn fleet_detach_available_for_class(&self, class: FleetDetachClass) -> u16 {
        let Some(fleet_record_index_1_based) = self.fleet.detach_donor_record_index_1_based else {
            return 0;
        };
        let Some(fleet) = self.game_data.fleets.get(fleet_record_index_1_based - 1) else {
            return 0;
        };
        let class_available = match class {
            FleetDetachClass::Battleships => fleet
                .ships
                .battleships
                .saturating_sub(self.fleet.detach_selection.battleships),
            FleetDetachClass::Cruisers => fleet
                .ships
                .cruisers
                .saturating_sub(self.fleet.detach_selection.cruisers),
            FleetDetachClass::Destroyers => fleet
                .ships
                .destroyers
                .saturating_sub(self.fleet.detach_selection.destroyers),
            FleetDetachClass::FullTransports => fleet
                .ships
                .loaded_transports
                .saturating_sub(self.fleet.detach_selection.full_transports),
            FleetDetachClass::EmptyTransports => nc_data::troop_transports(&fleet)
                .saturating_sub(fleet.ships.loaded_transports)
                .saturating_sub(self.fleet.detach_selection.empty_transports),
            FleetDetachClass::Scouts => fleet
                .ships
                .scouts
                .saturating_sub(self.fleet.detach_selection.scouts),
            FleetDetachClass::Etacs => fleet
                .ships
                .etacs
                .saturating_sub(self.fleet.detach_selection.etacs),
        };
        let total_limit = self
            .fleet_detach_remaining_total_after_selection()
            .saturating_sub(1);
        class_available.min(total_limit as u16)
    }

    fn parse_fleet_detach_class_code(&self, raw: &str) -> Option<FleetDetachClass> {
        ShipCategory::from_code(raw).map(FleetDetachClass::from)
    }

    /// The ships a selection would move, in notation, as the move itself
    /// resolves them. `none` before anything is staged.
    fn format_fleet_detach_summary_from_selection(
        &self,
        donor_record_index_1_based: Option<usize>,
        selection: &FleetDetachSelection,
    ) -> String {
        donor_record_index_1_based
            .and_then(|idx| self.game_data.fleets.get(idx - 1))
            .and_then(|donor| resolve_selection(donor, selection).ok())
            .map_or_else(|| "none".to_string(), |resolved| resolved.notation())
    }

    fn format_fleet_detach_summary_from_record(&self, record: &Fleet) -> String {
        match nc_data::fleet_notation(record) {
            notation if notation.is_empty() => "none".to_string(),
            notation => notation,
        }
    }

    fn fleet_detach_donor_after_selection_record(&self) -> Option<Fleet> {
        let fleet_record_index_1_based = self.fleet.detach_donor_record_index_1_based?;
        let donor = self.game_data.fleets.get(fleet_record_index_1_based - 1)?;
        let resolved = resolve_selection(donor, &self.fleet.detach_selection).ok()?;
        let mut donor_after = donor.clone();
        donor_after.crippled = resolved.remaining;
        donor_after.ships.battleships = donor_after
            .ships
            .battleships
            .saturating_sub(self.fleet.detach_selection.battleships);
        donor_after.ships.cruisers = donor_after
            .ships
            .cruisers
            .saturating_sub(self.fleet.detach_selection.cruisers);
        donor_after.ships.destroyers = donor_after
            .ships
            .destroyers
            .saturating_sub(self.fleet.detach_selection.destroyers);
        let transports = nc_data::troop_transports(&donor_after).saturating_sub(
            self.fleet.detach_selection.full_transports
                + self.fleet.detach_selection.empty_transports,
        );
        nc_data::set_troop_transports(&mut donor_after, transports);
        let armies = donor_after
            .ships
            .loaded_transports
            .saturating_sub(self.fleet.detach_selection.full_transports);
        nc_data::set_armies_aboard(&mut donor_after, armies);
        donor_after.ships.scouts = donor_after
            .ships
            .scouts
            .saturating_sub(self.fleet.detach_selection.scouts);
        donor_after.ships.etacs = donor_after
            .ships
            .etacs
            .saturating_sub(self.fleet.detach_selection.etacs);
        Some(donor_after)
    }

    fn fleet_detach_requires_speed_prompt(&self) -> bool {
        let Some(fleet_record_index_1_based) = self.fleet.detach_donor_record_index_1_based else {
            return false;
        };
        let Some(fleet) = self.game_data.fleets.get(fleet_record_index_1_based - 1) else {
            return false;
        };
        let Some(mut donor_after) = self.fleet_detach_donor_after_selection_record() else {
            return false;
        };
        nc_data::recompute_max_speed(&mut donor_after);
        if !fleet_record_supports_mission_code(&donor_after, donor_after.order.code()) {
            return false;
        }
        donor_after.max_speed > 0 && fleet.speed > donor_after.max_speed
    }

    fn fleet_detach_donor_default_speed(&self) -> u8 {
        let Some(mut donor_after) = self.fleet_detach_donor_after_selection_record() else {
            return 1;
        };
        nc_data::recompute_max_speed(&mut donor_after);
        donor_after.max_speed.max(1)
    }

    fn commit_fleet_detach_staged_selection(
        &mut self,
        donor_record_index_1_based: usize,
        donor_fleet_number: u16,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let donor_speed = if self.fleet_detach_requires_speed_prompt() {
            Some(
                self.fleet
                    .detach_donor_speed
                    .unwrap_or(self.fleet_detach_donor_default_speed()),
            )
        } else {
            None
        };
        let result = self.game_data.detach_ships_to_new_fleet(
            self.player.record_index_1_based,
            donor_record_index_1_based,
            self.fleet.detach_selection,
            donor_speed,
            // ECGAME's default for the new fleet; the detach screen has no
            // ROE prompt, so the player sets another with <C> afterwards.
            NEW_FLEET_DEFAULT_ROE,
        )?;
        self.save_game_data()?;
        self.reset_fleet_detach_staging();
        // The chain sort can move the donor to another record.
        self.fleet.detach_donor_record_index_1_based =
            Some(result.donor_fleet_record_index_1_based);
        if self.current_fleet_detach_ship_total(result.donor_fleet_record_index_1_based) > 1 {
            let new_fleet_number = self
                .fleet_number_for_record_index(result.new_fleet_record_index_1_based)
                .unwrap_or(0);
            self.fleet.detach_last_commissioned = Some(format!(
                "Commissioned Fleet #{new_fleet_number:02} from Fleet #{donor_fleet_number:02}."
            ));
            self.fleet.detach_mode = FleetDetachMode::ChoosingClass;
            self.current_screen = ScreenId::FleetDetach;
        } else {
            let new_fleet_number = self
                .fleet_number_for_record_index(result.new_fleet_record_index_1_based)
                .unwrap_or(0);
            self.fleet.detach_donor_record_index_1_based = None;
            self.fleet.detach_last_commissioned = None;
            self.show_fleet_context_success(
                format!(
                    "Detached ships from Fleet #{donor_fleet_number:02} into Fleet #{new_fleet_number:02}."
                ),
                true,
            );
        }
        Ok(())
    }

    pub(crate) fn fleet_number_for_record_index(&self, record_index_1_based: usize) -> Option<u16> {
        self.game_data
            .fleets
            .get(record_index_1_based - 1)
            .map(|fleet| fleet.fleet_number.0)
    }

    pub(crate) fn fleet_transfer_donor_row(&self) -> Option<crate::screen::FleetRow> {
        self.fleet
            .transfer_donor_record_index_1_based
            .and_then(|idx| self.fleet_row_by_record_index(idx))
    }

    pub(crate) fn fleet_transfer_host_row(&self) -> Option<crate::screen::FleetRow> {
        self.fleet
            .transfer_host_record_index_1_based
            .and_then(|idx| self.fleet_row_by_record_index(idx))
    }

    pub(crate) fn fleet_transfer_source_summary(&self) -> String {
        self.fleet
            .transfer_donor_record_index_1_based
            .and_then(|idx| self.game_data.fleets.get(idx - 1))
            .map(|fleet| self.format_fleet_detach_summary_from_record(fleet))
            .unwrap_or_else(|| "none".to_string())
    }

    pub(crate) fn fleet_transfer_destination_summary(&self) -> String {
        self.fleet
            .transfer_host_record_index_1_based
            .and_then(|idx| self.game_data.fleets.get(idx - 1))
            .map(|fleet| self.format_fleet_detach_summary_from_record(fleet))
            .unwrap_or_else(|| "none".to_string())
    }

    pub(crate) fn fleet_transfer_staged_summary(&self) -> String {
        self.format_fleet_detach_summary_from_selection(
            self.fleet.transfer_donor_record_index_1_based,
            &self.fleet.transfer_selection,
        )
    }

    pub(crate) fn fleet_transfer_remaining_summary(&self) -> String {
        self.fleet_transfer_donor_after_selection_record()
            .map(|fleet| self.format_fleet_detach_summary_from_record(&fleet))
            .unwrap_or_else(|| "none".to_string())
    }

    pub(crate) fn fleet_transfer_projected_destination_summary(&self) -> String {
        self.fleet_transfer_host_after_selection_record()
            .map(|fleet| self.format_fleet_detach_summary_from_record(&fleet))
            .unwrap_or_else(|| "none".to_string())
    }

    pub(crate) fn fleet_detach_donor_row(&self) -> Option<crate::screen::FleetRow> {
        self.fleet
            .detach_donor_record_index_1_based
            .and_then(|idx| self.fleet_row_by_record_index(idx))
    }

    pub(crate) fn fleet_transfer_prompt_and_default(&self) -> (String, String) {
        match self.fleet.transfer_mode {
            FleetTransferMode::ChoosingClass => (
                class_prompt(
                    ShipCategory::ALL
                        .into_iter()
                        .filter(|class| {
                            self.fleet_transfer_available_for_class((*class).into()) > 0
                        })
                        .map(ShipCategory::code),
                ),
                String::new(),
            ),
            FleetTransferMode::Confirming => (
                "Transfer these ships? [Y]/N <Q> -> ".to_string(),
                String::new(),
            ),
            FleetTransferMode::EnteringQuantity(class) => (
                quantity_prompt(class, self.fleet_transfer_available_for_class(class)),
                "1".to_string(),
            ),
        }
    }

    fn finish_fleet_transfer(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let used_checked_selection = self.fleet.command_context == FleetCommandContext::List
            && !self.fleet.group_selected_fleets.is_empty();
        let Some(donor_record_index_1_based) = self.fleet.transfer_donor_record_index_1_based
        else {
            self.fleet.transfer_status = Some("Select two fleets for transfer.".to_string());
            return Ok(());
        };
        let Some(host_record_index_1_based) = self.fleet.transfer_host_record_index_1_based else {
            self.fleet.transfer_status = Some("Select two fleets for transfer.".to_string());
            return Ok(());
        };
        self.game_data.transfer_ships_between_fleets(
            self.player.record_index_1_based,
            donor_record_index_1_based,
            host_record_index_1_based,
            self.fleet.transfer_selection.clone(),
        )?;
        self.save_game_data()?;
        let donor_fleet_number = self
            .fleet_number_for_record_index(donor_record_index_1_based)
            .unwrap_or(0);
        let host_fleet_number = self
            .fleet_number_for_record_index(host_record_index_1_based)
            .unwrap_or(0);
        self.fleet.transfer_mode = FleetTransferMode::ChoosingClass;
        self.fleet.transfer_donor_record_index_1_based = None;
        self.fleet.transfer_host_record_index_1_based = None;
        self.fleet.transfer_input.clear();
        self.fleet.transfer_selection = FleetDetachSelection::default();
        if used_checked_selection {
            self.clear_checked_fleet_selection();
        }
        self.show_fleet_context_success(
            format!(
                "Transferred ships from Fleet #{} to Fleet #{}.",
                donor_fleet_number, host_fleet_number
            ),
            true,
        );
        Ok(())
    }

    fn resolve_fleet_detach_numeric_input(
        &mut self,
        default: u16,
    ) -> Result<u16, Box<dyn std::error::Error>> {
        let raw = self.fleet.detach_input.trim();
        if raw.is_empty() {
            return Ok(default);
        }
        match raw.parse::<u16>() {
            Ok(value) => Ok(value),
            Err(_) => {
                self.fleet.detach_status = Some("Enter an integer value.".to_string());
                Err("invalid detach numeric input".into())
            }
        }
    }

    fn reset_fleet_transfer_staging(&mut self) {
        self.fleet.transfer_input.clear();
        self.fleet.transfer_status = None;
        self.fleet.transfer_selection = FleetDetachSelection::default();
    }

    fn fleet_transfer_remaining_total_after_selection(&self) -> u32 {
        let Some(donor_record_index_1_based) = self.fleet.transfer_donor_record_index_1_based
        else {
            return 0;
        };
        self.current_fleet_detach_ship_total(donor_record_index_1_based)
            .saturating_sub(self.fleet.transfer_selection.total_ships())
    }

    fn fleet_transfer_available_for_class(&self, class: FleetDetachClass) -> u16 {
        let Some(fleet_record_index_1_based) = self.fleet.transfer_donor_record_index_1_based
        else {
            return 0;
        };
        let Some(fleet) = self.game_data.fleets.get(fleet_record_index_1_based - 1) else {
            return 0;
        };
        let class_available = match class {
            FleetDetachClass::Battleships => fleet
                .ships
                .battleships
                .saturating_sub(self.fleet.transfer_selection.battleships),
            FleetDetachClass::Cruisers => fleet
                .ships
                .cruisers
                .saturating_sub(self.fleet.transfer_selection.cruisers),
            FleetDetachClass::Destroyers => fleet
                .ships
                .destroyers
                .saturating_sub(self.fleet.transfer_selection.destroyers),
            FleetDetachClass::FullTransports => fleet
                .ships
                .loaded_transports
                .saturating_sub(self.fleet.transfer_selection.full_transports),
            FleetDetachClass::EmptyTransports => nc_data::troop_transports(&fleet)
                .saturating_sub(fleet.ships.loaded_transports)
                .saturating_sub(self.fleet.transfer_selection.empty_transports),
            FleetDetachClass::Scouts => fleet
                .ships
                .scouts
                .saturating_sub(self.fleet.transfer_selection.scouts),
            FleetDetachClass::Etacs => fleet
                .ships
                .etacs
                .saturating_sub(self.fleet.transfer_selection.etacs),
        };
        let total_limit = self
            .fleet_transfer_remaining_total_after_selection()
            .saturating_sub(1);
        class_available.min(total_limit as u16)
    }

    fn fleet_transfer_donor_after_selection_record(&self) -> Option<Fleet> {
        let fleet_record_index_1_based = self.fleet.transfer_donor_record_index_1_based?;
        let donor = self.game_data.fleets.get(fleet_record_index_1_based - 1)?;
        let resolved = resolve_selection(donor, &self.fleet.transfer_selection).ok()?;
        let mut donor_after = donor.clone();
        donor_after.crippled = resolved.remaining;
        donor_after.ships.battleships = donor_after
            .ships
            .battleships
            .saturating_sub(self.fleet.transfer_selection.battleships);
        donor_after.ships.cruisers = donor_after
            .ships
            .cruisers
            .saturating_sub(self.fleet.transfer_selection.cruisers);
        donor_after.ships.destroyers = donor_after
            .ships
            .destroyers
            .saturating_sub(self.fleet.transfer_selection.destroyers);
        let transports = nc_data::troop_transports(&donor_after).saturating_sub(
            self.fleet.transfer_selection.full_transports
                + self.fleet.transfer_selection.empty_transports,
        );
        nc_data::set_troop_transports(&mut donor_after, transports);
        let armies = donor_after
            .ships
            .loaded_transports
            .saturating_sub(self.fleet.transfer_selection.full_transports);
        nc_data::set_armies_aboard(&mut donor_after, armies);
        donor_after.ships.scouts = donor_after
            .ships
            .scouts
            .saturating_sub(self.fleet.transfer_selection.scouts);
        donor_after.ships.etacs = donor_after
            .ships
            .etacs
            .saturating_sub(self.fleet.transfer_selection.etacs);
        Some(donor_after)
    }

    fn fleet_transfer_host_after_selection_record(&self) -> Option<Fleet> {
        let donor = self
            .game_data
            .fleets
            .get(self.fleet.transfer_donor_record_index_1_based? - 1)?;
        let resolved = resolve_selection(donor, &self.fleet.transfer_selection).ok()?;
        let fleet_record_index_1_based = self.fleet.transfer_host_record_index_1_based?;
        let mut host_after = self
            .game_data
            .fleets
            .get(fleet_record_index_1_based - 1)?
            .clone();
        host_after.crippled.add(&resolved.moved);
        host_after.ships.battleships = host_after
            .ships
            .battleships
            .saturating_add(self.fleet.transfer_selection.battleships);
        host_after.ships.cruisers = host_after
            .ships
            .cruisers
            .saturating_add(self.fleet.transfer_selection.cruisers);
        host_after.ships.destroyers = host_after
            .ships
            .destroyers
            .saturating_add(self.fleet.transfer_selection.destroyers);
        let transports = nc_data::troop_transports(&host_after).saturating_add(
            self.fleet.transfer_selection.full_transports
                + self.fleet.transfer_selection.empty_transports,
        );
        nc_data::set_troop_transports(&mut host_after, transports);
        let armies = host_after
            .ships
            .loaded_transports
            .saturating_add(self.fleet.transfer_selection.full_transports);
        nc_data::set_armies_aboard(&mut host_after, armies);
        host_after.ships.scouts = host_after
            .ships
            .scouts
            .saturating_add(self.fleet.transfer_selection.scouts);
        host_after.ships.etacs = host_after
            .ships
            .etacs
            .saturating_add(self.fleet.transfer_selection.etacs);
        Some(host_after)
    }

    pub(super) fn calculate_fleet_eta_message(
        &self,
        row: &FleetRow,
        destination: [u8; 2],
        include_system: bool,
    ) -> String {
        let mission_code = if include_system {
            nc_data::Order::ScoutSolarSystem.code()
        } else {
            nc_data::Order::MoveOnly.code()
        };
        fleet_target_eta_message(
            &self.game_data,
            row.fleet_record_index_1_based,
            row.fleet_number,
            mission_code,
            destination,
        )
    }
}

/// The longest class code a player types: `ETAC`.
const CLASS_CODE_MAX_LEN: usize = 4;

impl From<FleetDetachClass> for ShipCategory {
    fn from(class: FleetDetachClass) -> Self {
        match class {
            FleetDetachClass::Battleships => ShipCategory::Battleships,
            FleetDetachClass::Cruisers => ShipCategory::Cruisers,
            FleetDetachClass::Destroyers => ShipCategory::Destroyers,
            FleetDetachClass::FullTransports => ShipCategory::FullTransports,
            FleetDetachClass::EmptyTransports => ShipCategory::EmptyTransports,
            FleetDetachClass::Scouts => ShipCategory::Scouts,
            FleetDetachClass::Etacs => ShipCategory::Etacs,
        }
    }
}

impl From<ShipCategory> for FleetDetachClass {
    fn from(category: ShipCategory) -> Self {
        match category {
            ShipCategory::Battleships => FleetDetachClass::Battleships,
            ShipCategory::Cruisers => FleetDetachClass::Cruisers,
            ShipCategory::Destroyers => FleetDetachClass::Destroyers,
            ShipCategory::FullTransports => FleetDetachClass::FullTransports,
            ShipCategory::EmptyTransports => FleetDetachClass::EmptyTransports,
            ShipCategory::Scouts => FleetDetachClass::Scouts,
            ShipCategory::Etacs => FleetDetachClass::Etacs,
        }
    }
}

/// Whether `ch` may follow `typed` in a quantity: digits, then at most one
/// `!` after the digits (`3!` means three crippled ships).
fn quantity_char_fits(typed: &str, ch: char) -> bool {
    if typed.ends_with('!') {
        return false;
    }
    match ch {
        '!' => !typed.is_empty(),
        ch => ch.is_ascii_digit() && typed.len() < 3,
    }
}

/// A typed quantity: digits, with a trailing `!` for crippled ships only.
/// Empty input is the default quantity.
fn parse_quantity_input(typed: &str, default: u16) -> Option<(u16, bool)> {
    let typed = typed.trim();
    if typed.is_empty() {
        return Some((default, false));
    }
    let (digits, crippled) = match typed.strip_suffix('!') {
        Some(digits) => (digits, true),
        None => (typed, false),
    };
    digits.parse::<u16>().ok().map(|value| (value, crippled))
}

/// The class quantity and its current limit.
fn quantity_prompt(class: FleetDetachClass, available: u16) -> String {
    let code = ShipCategory::from(class).code();
    format!("{code} to stage (max {available}) ")
}

/// Why `value` ships of `category` cannot be staged on top of `selection`, if
/// they cannot. A plain quantity has to fit what the class has left; `n!`
/// has to fit the crippled ships the staged ships leave, and is never filled
/// out with healthy ones.
fn staging_error(
    donor: &Fleet,
    selection: &FleetDetachSelection,
    category: ShipCategory,
    value: u16,
    crippled: bool,
    available: u16,
) -> Option<String> {
    if !crippled {
        return (value == 0 || value > available)
            .then(|| format!("Enter a quantity from 1 to {available}."));
    }
    let crippled_left =
        crippled_left_to_stage(donor, selection, category).min(u32::from(available)) as u16;
    if crippled_left == 0 {
        return Some(format!("No crippled {} left to stage.", category.code()));
    }
    (value == 0 || value > crippled_left)
        .then(|| format!("Enter a crippled quantity from 1 to {crippled_left}."))
}

pub(super) fn fleet_eta_label(game_data: &CoreGameData, fleet_idx: usize) -> String {
    engine_fleet_eta_label(game_data, fleet_idx)
}

pub(super) fn fleet_list_eta_label(game_data: &CoreGameData, fleet_idx: usize) -> String {
    engine_fleet_list_eta_label(game_data, fleet_idx)
}

/// Ship-class choices that the current staging action can actually accept.
/// Commission/transfer and cancel are separate actions, not ship classes.
fn class_prompt(codes: impl IntoIterator<Item = &'static str>) -> String {
    let codes = codes.into_iter().collect::<Vec<_>>();
    if codes.is_empty() {
        "No more ships to stage ".to_string()
    } else {
        format!("Class <{}> ", codes.join(","))
    }
}
