use crate::app::state::App;
use crate::domains::fleet::FleetAction;
use crate::domains::fleet::state::{FleetCommandContext, FleetMenuPromptMode};
use crate::screen::layout::PromptFeedback;
use crate::screen::{
    CommandMenu, FleetDetachClass, FleetDetachMode, FleetRow, FleetTransferMode, ScreenId,
};
use nc_data::{CoreGameData, Fleet, FleetDetachSelection, NEW_FLEET_DEFAULT_ROE};
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
                if self.fleet.transfer_input.len() >= 3 {
                    return;
                }
                self.fleet.transfer_input.push(ch.to_ascii_uppercase());
            }
            FleetTransferMode::EnteringQuantity(_) => {
                if !ch.is_ascii_digit() || self.fleet.transfer_input.len() >= 3 {
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
                if self.fleet.detach_input.len() >= 3 {
                    return;
                }
                self.fleet.detach_input.push(ch.to_ascii_uppercase());
            }
            FleetDetachMode::EnteringQuantity(_) | FleetDetachMode::AdjustingDonorSpeed => {
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
            FleetTransferMode::ChoosingClass => {
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
            FleetDetachMode::ChoosingClass => {
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
                if raw.is_empty() {
                    self.fleet.transfer_status = Some("Enter a ship code, C, X, or Q.".to_string());
                    return Ok(());
                }
                match raw.as_str() {
                    "C" => {
                        if self.fleet.transfer_selection.total_ships() == 0 {
                            self.fleet.transfer_status =
                                Some("Stage at least one ship before committing.".to_string());
                            return Ok(());
                        }
                        if self.fleet_transfer_remaining_total_after_selection() == 0 {
                            self.fleet.transfer_status = Some(
                                "At least one ship must remain in the source fleet.".to_string(),
                            );
                            return Ok(());
                        }
                        self.finish_fleet_transfer()?;
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
                                Some("Use BB, CA, DD, TT*, TT, SC, ET, C, X, or Q.".to_string());
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
                let default_qty = 1u16.min(self.fleet_transfer_available_for_class(class));
                let value = self.resolve_fleet_transfer_numeric_input(default_qty)?;
                let available = self.fleet_transfer_available_for_class(class);
                if available == 0 {
                    self.fleet.transfer_status =
                        Some("That class is not available for transfer.".to_string());
                    self.fleet.transfer_mode = FleetTransferMode::ChoosingClass;
                    self.fleet.transfer_input.clear();
                    return Ok(());
                }
                if value == 0 || value > available {
                    self.fleet.transfer_status =
                        Some(format!("Enter a quantity from 1 to {available}."));
                    return Ok(());
                }
                match class {
                    FleetDetachClass::Battleships => {
                        self.fleet.transfer_selection.battleships += value
                    }
                    FleetDetachClass::Cruisers => self.fleet.transfer_selection.cruisers += value,
                    FleetDetachClass::Destroyers => {
                        self.fleet.transfer_selection.destroyers += value
                    }
                    FleetDetachClass::FullTransports => {
                        self.fleet.transfer_selection.full_transports += value
                    }
                    FleetDetachClass::EmptyTransports => {
                        self.fleet.transfer_selection.empty_transports += value
                    }
                    FleetDetachClass::Scouts => {
                        self.fleet.transfer_selection.scouts =
                            self.fleet.transfer_selection.scouts.saturating_add(value);
                    }
                    FleetDetachClass::Etacs => self.fleet.transfer_selection.etacs += value,
                }
                self.fleet.transfer_input.clear();
                self.fleet.transfer_status = None;
                self.fleet.transfer_mode = FleetTransferMode::ChoosingClass;
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
                if raw.is_empty() {
                    self.fleet.detach_status = Some("Enter a ship code, C, X, or Q.".to_string());
                    return Ok(());
                }
                match raw.as_str() {
                    "C" => {
                        if self.fleet.detach_selection.total_ships() == 0 {
                            self.fleet.detach_status =
                                Some("Stage at least one ship before commissioning.".to_string());
                            return Ok(());
                        }
                        if self.fleet_detach_remaining_total_after_selection() == 0 {
                            self.fleet.detach_status = Some(
                                "At least one ship must remain in the donor fleet.".to_string(),
                            );
                            return Ok(());
                        }
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
                    "X" => {
                        self.clear_fleet_detach_selection();
                    }
                    "Q" => {
                        self.cancel_fleet_detach();
                    }
                    _ => {
                        let Some(class) = self.parse_fleet_detach_class_code(&raw) else {
                            self.fleet.detach_status =
                                Some("Use BB, CA, DD, TT*, TT, SC, ET, C, X, or Q.".to_string());
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
                let default_qty = 1u16.min(self.fleet_detach_available_for_class(class));
                let value = self.resolve_fleet_detach_numeric_input(default_qty)?;
                let available = self.fleet_detach_available_for_class(class);
                if available == 0 {
                    self.fleet.detach_status =
                        Some("That class is not available for detach.".to_string());
                    self.fleet.detach_mode = FleetDetachMode::ChoosingClass;
                    self.fleet.detach_input.clear();
                    return Ok(());
                }
                if value == 0 || value > available {
                    self.fleet.detach_status =
                        Some(format!("Enter a quantity from 1 to {available}."));
                    return Ok(());
                }
                match class {
                    FleetDetachClass::Battleships => {
                        self.fleet.detach_selection.battleships += value;
                    }
                    FleetDetachClass::Cruisers => {
                        self.fleet.detach_selection.cruisers += value;
                    }
                    FleetDetachClass::Destroyers => {
                        self.fleet.detach_selection.destroyers += value;
                    }
                    FleetDetachClass::FullTransports => {
                        self.fleet.detach_selection.full_transports += value;
                    }
                    FleetDetachClass::EmptyTransports => {
                        self.fleet.detach_selection.empty_transports += value;
                    }
                    FleetDetachClass::Scouts => {
                        self.fleet.detach_selection.scouts =
                            self.fleet.detach_selection.scouts.saturating_add(value);
                    }
                    FleetDetachClass::Etacs => {
                        self.fleet.detach_selection.etacs += value;
                    }
                }
                self.fleet.detach_input.clear();
                self.fleet.detach_status = None;
                self.fleet.detach_mode = FleetDetachMode::ChoosingClass;
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
                    FleetDetachMode::EnteringQuantity(_) | FleetDetachMode::AdjustingDonorSpeed => {
                        ch.is_ascii_digit()
                    }
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
                    FleetTransferMode::EnteringQuantity(_) => ch.is_ascii_digit(),
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
                "Class <BB,CA,DD,TT*,TT,SC,ET,C,X> ".to_string(),
                String::new(),
            ),
            FleetDetachMode::EnteringQuantity(class) => (
                format!(
                    "{} to stage (max {}) ",
                    self.fleet_detach_class_label(class),
                    self.fleet_detach_available_for_class(class)
                ),
                "1".to_string(),
            ),
            FleetDetachMode::AdjustingDonorSpeed => (
                format!("Fleet #{} new speed ", fleet_number),
                self.fleet_detach_donor_default_speed().to_string(),
            ),
        }
    }

    pub(crate) fn fleet_detach_staged_summary(&self) -> String {
        self.format_fleet_detach_summary_from_selection(self.fleet.detach_selection)
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
                u32::from(fleet.battleship_count())
                    + u32::from(fleet.cruiser_count())
                    + u32::from(fleet.destroyer_count())
                    + u32::from(fleet.troop_transport_count())
                    + u32::from(fleet.scout_count())
                    + u32::from(fleet.etac_count())
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
                .battleship_count()
                .saturating_sub(self.fleet.detach_selection.battleships),
            FleetDetachClass::Cruisers => fleet
                .cruiser_count()
                .saturating_sub(self.fleet.detach_selection.cruisers),
            FleetDetachClass::Destroyers => fleet
                .destroyer_count()
                .saturating_sub(self.fleet.detach_selection.destroyers),
            FleetDetachClass::FullTransports => fleet
                .army_count()
                .saturating_sub(self.fleet.detach_selection.full_transports),
            FleetDetachClass::EmptyTransports => fleet
                .troop_transport_count()
                .saturating_sub(fleet.army_count())
                .saturating_sub(self.fleet.detach_selection.empty_transports),
            FleetDetachClass::Scouts => fleet
                .scout_count()
                .saturating_sub(self.fleet.detach_selection.scouts),
            FleetDetachClass::Etacs => fleet
                .etac_count()
                .saturating_sub(self.fleet.detach_selection.etacs),
        };
        let total_limit = self
            .fleet_detach_remaining_total_after_selection()
            .saturating_sub(1);
        class_available.min(total_limit as u16)
    }

    fn parse_fleet_detach_class_code(&self, raw: &str) -> Option<FleetDetachClass> {
        match raw {
            "BB" => Some(FleetDetachClass::Battleships),
            "CA" => Some(FleetDetachClass::Cruisers),
            "DD" => Some(FleetDetachClass::Destroyers),
            "TT*" => Some(FleetDetachClass::FullTransports),
            "TT" => Some(FleetDetachClass::EmptyTransports),
            "SC" => Some(FleetDetachClass::Scouts),
            "ET" => Some(FleetDetachClass::Etacs),
            _ => None,
        }
    }

    fn fleet_detach_class_label(&self, class: FleetDetachClass) -> &'static str {
        match class {
            FleetDetachClass::Battleships => "BB",
            FleetDetachClass::Cruisers => "CA",
            FleetDetachClass::Destroyers => "DD",
            FleetDetachClass::FullTransports => "TT*",
            FleetDetachClass::EmptyTransports => "TT",
            FleetDetachClass::Scouts => "SC",
            FleetDetachClass::Etacs => "ET",
        }
    }

    fn format_fleet_detach_summary_from_selection(
        &self,
        selection: FleetDetachSelection,
    ) -> String {
        let mut parts = Vec::new();
        for (label, count) in [
            ("SC", selection.scouts),
            ("BB", selection.battleships),
            ("CA", selection.cruisers),
            ("DD", selection.destroyers),
            ("TT*", selection.full_transports),
            ("TT", selection.empty_transports),
            ("ET", selection.etacs),
        ] {
            if count > 0 {
                parts.push(format!("{label}={count}"));
            }
        }
        if parts.is_empty() {
            "none".to_string()
        } else {
            parts.join(" ")
        }
    }

    fn format_fleet_detach_summary_from_record(&self, record: &Fleet) -> String {
        let empty_transports = record
            .troop_transport_count()
            .saturating_sub(record.army_count());
        self.format_fleet_detach_summary_from_selection(FleetDetachSelection {
            battleships: record.battleship_count(),
            cruisers: record.cruiser_count(),
            destroyers: record.destroyer_count(),
            full_transports: record.army_count(),
            empty_transports,
            scouts: record.scout_count(),
            etacs: record.etac_count(),
        })
    }

    fn fleet_detach_donor_after_selection_record(&self) -> Option<Fleet> {
        let fleet_record_index_1_based = self.fleet.detach_donor_record_index_1_based?;
        let mut donor_after = self
            .game_data
            .fleets
            .get(fleet_record_index_1_based - 1)?
            .clone();
        donor_after.set_battleship_count(
            donor_after
                .battleship_count()
                .saturating_sub(self.fleet.detach_selection.battleships),
        );
        donor_after.set_cruiser_count(
            donor_after
                .cruiser_count()
                .saturating_sub(self.fleet.detach_selection.cruisers),
        );
        donor_after.set_destroyer_count(
            donor_after
                .destroyer_count()
                .saturating_sub(self.fleet.detach_selection.destroyers),
        );
        donor_after.set_troop_transport_count(donor_after.troop_transport_count().saturating_sub(
            self.fleet.detach_selection.full_transports
                + self.fleet.detach_selection.empty_transports,
        ));
        donor_after.set_army_count(
            donor_after
                .army_count()
                .saturating_sub(self.fleet.detach_selection.full_transports),
        );
        donor_after.set_scout_count(
            donor_after
                .scout_count()
                .saturating_sub(self.fleet.detach_selection.scouts),
        );
        donor_after.set_etac_count(
            donor_after
                .etac_count()
                .saturating_sub(self.fleet.detach_selection.etacs),
        );
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
        donor_after.recompute_max_speed_from_composition();
        if !fleet_record_supports_mission_code(&donor_after, donor_after.standing_order_code_raw())
        {
            return false;
        }
        donor_after.max_speed() > 0 && fleet.current_speed() > donor_after.max_speed()
    }

    fn fleet_detach_donor_default_speed(&self) -> u8 {
        let Some(mut donor_after) = self.fleet_detach_donor_after_selection_record() else {
            return 1;
        };
        donor_after.recompute_max_speed_from_composition();
        donor_after.max_speed().max(1)
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
            .map(|fleet| fleet.local_slot_word_raw())
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
        self.format_fleet_detach_summary_from_selection(self.fleet.transfer_selection)
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
                "Class <BB,CA,DD,TT*,TT,SC,ET,C,X> ".to_string(),
                String::new(),
            ),
            FleetTransferMode::EnteringQuantity(class) => (
                format!(
                    "{} to stage (max {}) ",
                    self.fleet_detach_class_label(class),
                    self.fleet_transfer_available_for_class(class)
                ),
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

    fn resolve_fleet_transfer_numeric_input(
        &mut self,
        default: u16,
    ) -> Result<u16, Box<dyn std::error::Error>> {
        let raw = self.fleet.transfer_input.trim();
        if raw.is_empty() {
            return Ok(default);
        }
        match raw.parse::<u16>() {
            Ok(value) => Ok(value),
            Err(_) => {
                self.fleet.transfer_status = Some("Enter an integer value.".to_string());
                Err("invalid transfer numeric input".into())
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
                .battleship_count()
                .saturating_sub(self.fleet.transfer_selection.battleships),
            FleetDetachClass::Cruisers => fleet
                .cruiser_count()
                .saturating_sub(self.fleet.transfer_selection.cruisers),
            FleetDetachClass::Destroyers => fleet
                .destroyer_count()
                .saturating_sub(self.fleet.transfer_selection.destroyers),
            FleetDetachClass::FullTransports => fleet
                .army_count()
                .saturating_sub(self.fleet.transfer_selection.full_transports),
            FleetDetachClass::EmptyTransports => fleet
                .troop_transport_count()
                .saturating_sub(fleet.army_count())
                .saturating_sub(self.fleet.transfer_selection.empty_transports),
            FleetDetachClass::Scouts => fleet
                .scout_count()
                .saturating_sub(self.fleet.transfer_selection.scouts),
            FleetDetachClass::Etacs => fleet
                .etac_count()
                .saturating_sub(self.fleet.transfer_selection.etacs),
        };
        let total_limit = self
            .fleet_transfer_remaining_total_after_selection()
            .saturating_sub(1);
        class_available.min(total_limit as u16)
    }

    fn fleet_transfer_donor_after_selection_record(&self) -> Option<Fleet> {
        let fleet_record_index_1_based = self.fleet.transfer_donor_record_index_1_based?;
        let mut donor_after = self
            .game_data
            .fleets
            .get(fleet_record_index_1_based - 1)?
            .clone();
        donor_after.set_battleship_count(
            donor_after
                .battleship_count()
                .saturating_sub(self.fleet.transfer_selection.battleships),
        );
        donor_after.set_cruiser_count(
            donor_after
                .cruiser_count()
                .saturating_sub(self.fleet.transfer_selection.cruisers),
        );
        donor_after.set_destroyer_count(
            donor_after
                .destroyer_count()
                .saturating_sub(self.fleet.transfer_selection.destroyers),
        );
        donor_after.set_troop_transport_count(donor_after.troop_transport_count().saturating_sub(
            self.fleet.transfer_selection.full_transports
                + self.fleet.transfer_selection.empty_transports,
        ));
        donor_after.set_army_count(
            donor_after
                .army_count()
                .saturating_sub(self.fleet.transfer_selection.full_transports),
        );
        donor_after.set_scout_count(
            donor_after
                .scout_count()
                .saturating_sub(self.fleet.transfer_selection.scouts),
        );
        donor_after.set_etac_count(
            donor_after
                .etac_count()
                .saturating_sub(self.fleet.transfer_selection.etacs),
        );
        Some(donor_after)
    }

    fn fleet_transfer_host_after_selection_record(&self) -> Option<Fleet> {
        let fleet_record_index_1_based = self.fleet.transfer_host_record_index_1_based?;
        let mut host_after = self
            .game_data
            .fleets
            .get(fleet_record_index_1_based - 1)?
            .clone();
        host_after.set_battleship_count(
            host_after
                .battleship_count()
                .saturating_add(self.fleet.transfer_selection.battleships),
        );
        host_after.set_cruiser_count(
            host_after
                .cruiser_count()
                .saturating_add(self.fleet.transfer_selection.cruisers),
        );
        host_after.set_destroyer_count(
            host_after
                .destroyer_count()
                .saturating_add(self.fleet.transfer_selection.destroyers),
        );
        host_after.set_troop_transport_count(host_after.troop_transport_count().saturating_add(
            self.fleet.transfer_selection.full_transports
                + self.fleet.transfer_selection.empty_transports,
        ));
        host_after.set_army_count(
            host_after
                .army_count()
                .saturating_add(self.fleet.transfer_selection.full_transports),
        );
        host_after.set_scout_count(
            host_after
                .scout_count()
                .saturating_add(self.fleet.transfer_selection.scouts),
        );
        host_after.set_etac_count(
            host_after
                .etac_count()
                .saturating_add(self.fleet.transfer_selection.etacs),
        );
        Some(host_after)
    }

    pub(super) fn calculate_fleet_eta_message(
        &self,
        row: &FleetRow,
        destination: [u8; 2],
        include_system: bool,
    ) -> String {
        let mission_code = if include_system {
            nc_data::Order::ScoutSolarSystem.to_raw()
        } else {
            nc_data::Order::MoveOnly.to_raw()
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

pub(super) fn fleet_eta_label(game_data: &CoreGameData, fleet_idx: usize) -> String {
    engine_fleet_eta_label(game_data, fleet_idx)
}

pub(super) fn fleet_list_eta_label(game_data: &CoreGameData, fleet_idx: usize) -> String {
    engine_fleet_list_eta_label(game_data, fleet_idx)
}
