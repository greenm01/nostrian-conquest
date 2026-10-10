use crate::app::helpers::sync_scroll_to_cursor;
use crate::app::state::App;
use crate::screen::{
    CommandMenu, PlanetBuildChangeRow, PlanetBuildListRow, PlanetBuildMenuView, PlanetBuildOrder,
    PlanetCommissionDraftRow, PlanetCommissionPickerRow, PlanetCommissionRow, PlanetCommissionView,
    PlanetListSort, ScreenId, build_unit_spec, build_unit_spec_by_kind, format_fleet_number,
    format_sector_coords, format_sector_coords_table,
};
use crossterm::event::KeyCode;
use nc_data::{
    AutoCommissionEntry, AutoCommissionFleetEntry, AutoCommissionReport,
    AutoCommissionStarbaseEntry, CommissionFleetDraft, CommissionResult, GameStateMutationError,
    ProductionItemKind, STARDOCK_SLOT_COUNT,
};
use nc_engine::{
    commission_fleet_draft_from_entries, planet_build_committed_points, planet_build_list_entries,
    planet_build_max_quantity, planet_build_orders, planet_build_unavailable_message,
    planet_build_view, planet_commission_draft_state, planet_commission_slot_entries,
    planet_has_any_buildable_unit, production_item_kind_raw,
};

impl App {
    const PLANET_BUILD_BUDGET_EXHAUSTED_NOTICE: &'static str = "No build budget remains.";

    fn planet_commission_picker_visible_rows(&self) -> usize {
        crate::domains::planet::screens::planet_commission::planet_commission_picker_visible_rows(
            self.screen_geometry,
        )
    }

    fn planet_commission_visible_rows(&self) -> usize {
        crate::domains::planet::screens::planet_commission::planet_commission_visible_rows(
            self.screen_geometry,
        )
    }

    fn planet_build_change_visible_rows(&self) -> usize {
        crate::domains::planet::screens::planet_build::planet_build_change_visible_rows(
            self.screen_geometry,
        )
    }

    fn planet_build_list_visible_rows(&self) -> usize {
        crate::domains::planet::screens::planet_build::planet_build_list_visible_rows(
            self.screen_geometry,
        )
    }

    fn planet_auto_commission_report_page_rows(&self) -> usize {
        crate::domains::planet::screens::planet_commission::planet_auto_commission_report_page_rows(
            self.screen_geometry,
        )
    }

    fn planet_build_return_screen(&self) -> ScreenId {
        if self.planet.build_return_to_list {
            ScreenId::PlanetList(crate::screen::PlanetListMode::Brief, self.planet.list_sort)
        } else {
            ScreenId::PlanetBuildMenu
        }
    }

    pub fn open_planet_commission_menu(&mut self) {
        if matches!(
            self.current_screen,
            ScreenId::PlanetList(crate::screen::PlanetListMode::Brief, _)
        ) {
            self.planet.command_context = crate::domains::planet::state::PlanetCommandContext::List;
            self.clear_command_menu_notice();
            self.clear_planet_list_status();
            self.close_planet_tax_prompt();
            self.close_planet_auto_commission_prompt();
            self.close_planet_info_prompt();
            self.clear_planet_commission_draft_state();
            self.planet.commission_status = None;
            self.planet.commission_result_title = None;
            self.planet.commission_result_return_to_picker = false;
            self.planet.commission_result_dismiss_key = None;
            self.planet.commission_result_notice = None;
            self.planet.commission_cursor = 0;
            self.planet.commission_scroll_offset = 0;
            self.planet.commission_selected_slots.clear();
            let Ok(row) = self.current_planet_list_row() else {
                self.show_planet_context_notice("You do not currently control any planets.");
                return;
            };
            let Some(index) = self.commission_planet_rows().iter().position(|planet| {
                planet.planet_record_index_1_based == row.planet_record_index_1_based
            }) else {
                self.show_planet_context_notice(
                    "That planet has no ships or starbases waiting in stardock.",
                );
                return;
            };
            self.planet.commission_index = index;
            self.load_planet_commission_draft_for_current_planet();
            self.current_screen = ScreenId::PlanetCommissionDraft;
            return;
        }
        self.command_return_menu = CommandMenu::Planet;
        self.close_planet_tax_prompt();
        self.close_planet_auto_commission_prompt();
        self.close_planet_info_prompt();
        self.clear_command_menu_notice();
        self.clear_planet_commission_draft_state();
        self.planet.commission_status = None;
        self.planet.commission_result_title = None;
        self.planet.commission_result_return_to_picker = false;
        self.planet.commission_result_dismiss_key = None;
        self.planet.commission_result_notice = None;
        self.planet.commission_cursor = 0;
        self.planet.commission_scroll_offset = 0;
        self.planet.commission_selected_slots.clear();
        let total = self.commission_planet_rows().len();
        if total == 0 {
            self.planet.commission_index = 0;
            self.planet.commission_picker_scroll_offset = 0;
            self.show_command_menu_notice(
                CommandMenu::Planet,
                "No owned planets have units waiting in stardock.",
            );
            return;
        }
        self.planet.commission_index = self.planet.commission_index.min(total - 1);
        let visible_rows = self.planet_commission_picker_visible_rows();
        sync_scroll_to_cursor(
            &mut self.planet.commission_picker_scroll_offset,
            self.planet.commission_index,
            visible_rows,
        );
        self.current_screen = ScreenId::PlanetCommissionPicker;
    }

    pub fn open_planet_commission_planet(&mut self) {
        if self.current_screen != ScreenId::PlanetCommissionPicker {
            return;
        }
        let total = self.commission_planet_rows().len();
        if total == 0 {
            self.open_planet_commission_menu();
            return;
        }
        self.planet.commission_index = self.planet.commission_index.min(total - 1);
        self.clear_planet_commission_draft_state();
        self.planet.commission_cursor = 0;
        self.planet.commission_scroll_offset = 0;
        self.planet.commission_selected_slots.clear();
        self.planet.commission_status = None;
        self.planet.commission_result_title = None;
        self.planet.commission_result_return_to_picker = false;
        self.planet.commission_result_dismiss_key = None;
        self.planet.commission_result_notice = None;
        self.load_planet_commission_draft_for_current_planet();
        self.current_screen = ScreenId::PlanetCommissionDraft;
    }

    pub fn close_planet_commission_planet(&mut self) {
        if self.current_screen != ScreenId::PlanetCommissionMenu {
            return;
        }
        self.clear_planet_commission_draft_state();
        self.planet.commission_selected_slots.clear();
        self.planet.commission_status = None;
        self.planet.commission_result_dismiss_key = None;
        self.current_screen = ScreenId::PlanetCommissionPicker;
    }

    pub fn dismiss_planet_commission_result(&mut self, key_code: KeyCode) {
        if self.current_screen != ScreenId::PlanetCommissionResult {
            return;
        }
        if self.planet.command_context == crate::domains::planet::state::PlanetCommandContext::List
        {
            self.clear_planet_commission_draft_state();
            self.planet.commission_result_title = None;
            self.planet.commission_result_return_to_picker = false;
            self.planet.commission_result_notice = None;
            self.planet.commission_status = None;
            self.planet.commission_selected_slots.clear();
            self.planet.commission_result_dismiss_key = Some(key_code);
            self.current_screen = self.planet_context_screen();
            return;
        }
        let return_to_picker = self.planet.commission_result_return_to_picker;
        self.clear_planet_commission_draft_state();
        self.planet.commission_result_title = None;
        self.planet.commission_result_return_to_picker = false;
        self.planet.commission_result_notice = None;
        self.planet.commission_status = None;
        self.planet.commission_selected_slots.clear();
        if return_to_picker {
            let total = self.commission_planet_rows().len();
            if total == 0 {
                self.planet.commission_result_dismiss_key = None;
                self.open_planet_menu();
                return;
            }
            self.planet.commission_index = self.planet.commission_index.min(total - 1);
            let visible_rows = self.planet_commission_picker_visible_rows();
            sync_scroll_to_cursor(
                &mut self.planet.commission_picker_scroll_offset,
                self.planet.commission_index,
                visible_rows,
            );
            self.planet.commission_result_dismiss_key = Some(key_code);
            self.current_screen = ScreenId::PlanetCommissionPicker;
            return;
        }
        let current_planet_has_rows = self
            .current_commission_planet_row()
            .ok()
            .map(|row| {
                !self
                    .current_planet_commission_rows_for(row.planet_record_index_1_based)
                    .is_empty()
            })
            .unwrap_or(false);
        if current_planet_has_rows {
            self.load_planet_commission_draft_for_current_planet();
            self.planet.commission_result_dismiss_key = Some(key_code);
            self.current_screen = ScreenId::PlanetCommissionDraft;
            return;
        }
        let total = self.commission_planet_rows().len();
        if total == 0 {
            self.planet.commission_result_dismiss_key = None;
            self.open_planet_menu();
            return;
        }
        self.planet.commission_index = self.planet.commission_index.min(total - 1);
        let visible_rows = self.planet_commission_picker_visible_rows();
        sync_scroll_to_cursor(
            &mut self.planet.commission_picker_scroll_offset,
            self.planet.commission_index,
            visible_rows,
        );
        self.planet.commission_result_dismiss_key = Some(key_code);
        self.current_screen = ScreenId::PlanetCommissionPicker;
    }

    pub fn clear_planet_commission_dismiss_key(&mut self) {
        self.planet.commission_result_dismiss_key = None;
    }

    pub fn open_planet_build_menu(&mut self) {
        if self.planet.build_return_to_list
            && matches!(
                self.current_screen,
                ScreenId::PlanetBuildSpecify
                    | ScreenId::PlanetBuildQuantity
                    | ScreenId::PlanetBuildList
            )
        {
            self.planet.build_status = None;
            self.planet.build_unit_input.clear();
            self.planet.build_unit_status = None;
            self.planet.build_unit_notice = None;
            self.planet.build_quantity_input.clear();
            self.planet.build_quantity_status = None;
            self.planet.build_selected_kind = None;
            self.current_screen = self.planet_build_return_screen();
            return;
        }
        self.command_return_menu = CommandMenu::PlanetBuild;
        self.planet.build_return_to_list = false;
        self.close_planet_build_abort_prompt();
        self.planet.build_status = None;
        self.planet.build_unit_input.clear();
        self.planet.build_unit_status = None;
        self.planet.build_unit_notice = None;
        self.planet.build_quantity_input.clear();
        self.planet.build_quantity_status = None;
        self.planet.build_selected_kind = None;
        self.planet.build_list_scroll_offset = 0;
        self.reset_planet_build_list_delete_state();
        let total = self.build_planet_rows().len();
        if total == 0 {
            self.planet.build_index = 0;
            self.show_command_menu_notice(
                CommandMenu::Planet,
                "No owned planets available for building.",
            );
            return;
        }
        self.clear_command_menu_notice();
        self.planet.build_index = self.planet.build_index.min(total - 1);
        self.current_screen = ScreenId::PlanetBuildMenu;
    }

    pub fn open_current_build_planet_info(&mut self) {
        if self.build_planet_rows().is_empty() {
            self.open_planet_build_menu();
            return;
        }
        let Ok(row) = self.current_build_planet_row() else {
            self.open_planet_build_menu();
            return;
        };
        self.command_return_menu = CommandMenu::PlanetBuild;
        let _ = self.open_planet_info_detail_at_coords(row.coords, Some(ScreenId::PlanetBuildMenu));
    }

    pub fn open_planet_build_list(&mut self) {
        let opened_from_planet_list = matches!(
            self.current_screen,
            ScreenId::PlanetList(crate::screen::PlanetListMode::Brief, _)
        );
        if opened_from_planet_list {
            let Ok(row) = self.current_planet_list_row() else {
                self.show_planet_context_notice("You do not currently control any planets.");
                return;
            };
            let Some(index) = self.build_planet_rows().iter().position(|planet| {
                planet.planet_record_index_1_based == row.planet_record_index_1_based
            }) else {
                self.show_planet_context_notice("No owned planets available for building.");
                return;
            };
            self.planet.command_context = crate::domains::planet::state::PlanetCommandContext::List;
            self.planet.build_return_to_list = true;
            self.planet.build_index = index;
            self.clear_planet_list_status();
        }
        if self.build_planet_rows().is_empty() {
            if opened_from_planet_list {
                self.show_planet_context_notice("No owned planets available for building.");
            } else {
                self.open_planet_build_menu();
            }
            return;
        }
        if self.current_planet_build_orders().is_empty() {
            if opened_from_planet_list {
                self.show_planet_context_notice("No build orders are queued.");
            } else {
                self.planet.build_status = Some("No build orders are queued.".to_string());
                self.current_screen = ScreenId::PlanetBuildMenu;
            }
            return;
        }
        self.planet.build_list_scroll_offset = 0;
        self.planet.build_list_cursor = 0;
        self.reset_planet_build_list_delete_state();
        self.current_screen = ScreenId::PlanetBuildList;
    }

    pub fn open_planet_build_change(&mut self) {
        // Pre-position cursor on the current planet so it's already highlighted.
        self.planet.build_change_cursor = self.planet.build_index;
        self.planet.build_change_scroll_offset = 0;
        let visible_rows = self.planet_build_change_visible_rows();
        sync_scroll_to_cursor(
            &mut self.planet.build_change_scroll_offset,
            self.planet.build_change_cursor,
            visible_rows,
        );
        self.current_screen = ScreenId::PlanetBuildChange;
    }

    pub fn move_planet_build_change_cursor(&mut self, delta: i8) {
        if self.current_screen != ScreenId::PlanetBuildChange {
            return;
        }
        let total = self.build_planet_rows().len();
        if total == 0 {
            return;
        }
        let next = self.planet.build_change_cursor as isize + delta as isize;
        self.planet.build_change_cursor = next.rem_euclid(total as isize) as usize;
        let visible_rows = self.planet_build_change_visible_rows();
        sync_scroll_to_cursor(
            &mut self.planet.build_change_scroll_offset,
            self.planet.build_change_cursor,
            visible_rows,
        );
    }

    pub fn confirm_planet_build_change(&mut self) {
        let total = self.build_planet_rows().len();
        if total == 0 {
            self.current_screen = ScreenId::PlanetBuildMenu;
            return;
        }
        self.planet.build_index = self.planet.build_change_cursor.min(total - 1);
        self.planet.build_status = None;
        self.current_screen = ScreenId::PlanetBuildMenu;
    }

    pub fn open_planet_build_abort_prompt(&mut self) {
        let opened_from_planet_list = matches!(
            self.current_screen,
            ScreenId::PlanetList(crate::screen::PlanetListMode::Brief, _)
        );
        if opened_from_planet_list {
            let Ok(row) = self.current_planet_list_row() else {
                self.show_planet_context_notice("You do not currently control any planets.");
                return;
            };
            let Some(index) = self.build_planet_rows().iter().position(|planet| {
                planet.planet_record_index_1_based == row.planet_record_index_1_based
            }) else {
                self.show_planet_context_notice("No owned planets available for building.");
                return;
            };
            self.planet.command_context = crate::domains::planet::state::PlanetCommandContext::List;
            self.planet.build_return_to_list = true;
            self.planet.build_index = index;
            self.clear_planet_list_status();
        }
        if self.build_planet_rows().is_empty() {
            if opened_from_planet_list {
                self.show_planet_context_notice("No owned planets available for building.");
            } else {
                self.open_planet_build_menu();
            }
            return;
        }
        if self.current_planet_build_orders().is_empty() {
            if opened_from_planet_list {
                self.show_planet_context_notice("No build orders are queued.");
            } else {
                self.planet.build_status = Some("No build orders are queued.".to_string());
                self.current_screen = ScreenId::PlanetBuildMenu;
            }
            return;
        }
        self.close_planet_info_prompt();
        self.planet.build_status = None;
        self.planet.build_abort_prompt_active = true;
        self.current_screen = if opened_from_planet_list {
            self.planet_build_return_screen()
        } else {
            ScreenId::PlanetBuildMenu
        };
    }

    pub fn close_planet_build_abort_prompt(&mut self) {
        self.planet.build_abort_prompt_active = false;
    }

    pub fn open_planet_build_specify(&mut self) {
        let opened_from_planet_list = matches!(
            self.current_screen,
            ScreenId::PlanetList(crate::screen::PlanetListMode::Brief, _)
        );
        if matches!(
            self.current_screen,
            ScreenId::PlanetList(crate::screen::PlanetListMode::Brief, _)
        ) {
            let Ok(row) = self.current_planet_list_row() else {
                self.show_planet_context_notice("You do not currently control any planets.");
                return;
            };
            let Some(index) = self.build_planet_rows().iter().position(|planet| {
                planet.planet_record_index_1_based == row.planet_record_index_1_based
            }) else {
                self.show_planet_context_notice("No owned planets available for building.");
                return;
            };
            self.planet.build_return_to_list = true;
            self.planet.build_index = index;
            self.clear_planet_list_status();
        }
        if self.build_planet_rows().is_empty() {
            self.open_planet_build_menu();
            return;
        }
        self.planet.build_unit_input.clear();
        self.planet.build_unit_status = None;
        self.planet.build_unit_notice = None;
        self.planet.build_quantity_input.clear();
        self.planet.build_quantity_status = None;
        self.planet.build_selected_kind = None;
        if !self.current_planet_can_afford_any_build() {
            self.show_planet_build_budget_exhausted_notice(opened_from_planet_list);
            return;
        }
        self.current_screen = ScreenId::PlanetBuildSpecify;
    }

    pub fn move_planet_build(&mut self, delta: i8) {
        let total = self.build_planet_rows().len();
        if total == 0 {
            self.planet.build_index = 0;
            return;
        }
        // Wrap around so N on the last planet returns to the first.
        let next = self.planet.build_index as isize + delta as isize;
        self.planet.build_index = next.rem_euclid(total as isize) as usize;
        self.planet.build_status = None;
    }

    pub fn move_planet_commission_planet(&mut self, delta: i8) {
        if self.current_screen != ScreenId::PlanetCommissionPicker {
            return;
        }
        let total = self.commission_planet_rows().len();
        if total == 0 {
            self.planet.commission_index = 0;
            return;
        }
        let next = self.planet.commission_index as isize + delta as isize;
        self.planet.commission_index = next.rem_euclid(total as isize) as usize;
        let visible_rows = self.planet_commission_picker_visible_rows();
        sync_scroll_to_cursor(
            &mut self.planet.commission_picker_scroll_offset,
            self.planet.commission_index,
            visible_rows,
        );
    }

    pub fn move_planet_commission_row(&mut self, delta: i8) {
        if self.current_screen != ScreenId::PlanetCommissionMenu {
            return;
        }
        let total = self.current_planet_commission_rows().len();
        if total == 0 {
            self.planet.commission_cursor = 0;
            return;
        }
        let next = self.planet.commission_cursor as isize + delta as isize;
        self.planet.commission_cursor = next.rem_euclid(total as isize) as usize;
        if self.planet.commission_cursor < self.planet.commission_scroll_offset {
            self.planet.commission_scroll_offset = self.planet.commission_cursor;
        } else {
            let visible_rows = self.planet_commission_visible_rows();
            if self.planet.commission_cursor >= self.planet.commission_scroll_offset + visible_rows
            {
                self.planet.commission_scroll_offset =
                    self.planet.commission_cursor + 1 - visible_rows;
            }
        }
        self.planet.commission_status = None;
    }

    pub fn move_planet_commission_draft_row(
        &mut self,
        delta: i8,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if self.current_screen != ScreenId::PlanetCommissionDraft {
            return Ok(());
        }
        if !self.commit_current_planet_commission_draft_input()? {
            return Ok(());
        }
        let total = self.planet.commission_draft_rows.len();
        if total == 0 {
            self.planet.commission_draft_cursor = 0;
            return Ok(());
        }
        let next = self.planet.commission_draft_cursor as isize + delta as isize;
        self.planet.commission_draft_cursor = next.rem_euclid(total as isize) as usize;
        self.planet.commission_draft_input.clear();
        self.planet.commission_draft_status = None;
        Ok(())
    }

    pub fn append_planet_commission_draft_char(&mut self, ch: char) {
        if self.current_screen != ScreenId::PlanetCommissionDraft
            || !ch.is_ascii_digit()
            || self.planet.commission_draft_input.len() >= 3
        {
            return;
        }
        let Some(row) = self
            .planet
            .commission_draft_rows
            .get(self.planet.commission_draft_cursor)
        else {
            return;
        };
        if !row.accepts_fleet_qty() {
            return;
        }
        self.planet.commission_draft_input.push(ch);
        self.planet.commission_draft_status = None;
        self.planet.commission_draft_notice = None;
    }

    pub fn backspace_planet_commission_draft_input(&mut self) {
        if self.current_screen != ScreenId::PlanetCommissionDraft {
            return;
        }
        let Some(row) = self
            .planet
            .commission_draft_rows
            .get(self.planet.commission_draft_cursor)
        else {
            return;
        };
        if !row.accepts_fleet_qty() {
            return;
        }
        self.planet.commission_draft_input.pop();
        self.planet.commission_draft_status = None;
        self.planet.commission_draft_notice = None;
    }

    pub fn close_planet_commission_draft(&mut self) {
        if self.current_screen != ScreenId::PlanetCommissionDraft {
            return;
        }
        self.clear_planet_commission_draft_state();
        self.planet.commission_status = None;
        self.planet.commission_result_dismiss_key = None;
        if self.planet.command_context == crate::domains::planet::state::PlanetCommandContext::List
        {
            self.current_screen = self.planet_context_screen();
            return;
        }
        if self.commission_planet_rows().is_empty() {
            self.open_planet_menu();
            return;
        }
        self.current_screen = ScreenId::PlanetCommissionPicker;
    }

    pub fn commission_selected_stardock_row(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.current_screen != ScreenId::PlanetCommissionMenu {
            return Ok(());
        }
        let rows = self.current_planet_commission_rows();
        let Some(current_row) = rows.get(self.planet.commission_cursor) else {
            self.planet.commission_status = Some("No stardock units are available.".to_string());
            return Ok(());
        };
        let selected_slots: Vec<usize> = if self.planet.commission_selected_slots.is_empty() {
            vec![current_row.slot_0_based]
        } else {
            rows.iter()
                .filter(|row| {
                    self.planet
                        .commission_selected_slots
                        .contains(&row.slot_0_based)
                })
                .map(|row| row.slot_0_based)
                .collect()
        };
        let selected_rows: Vec<PlanetCommissionRow> = rows
            .iter()
            .filter(|row| selected_slots.contains(&row.slot_0_based))
            .cloned()
            .collect();
        let starbase_count = selected_rows
            .iter()
            .filter(|row| row.kind == ProductionItemKind::Starbase)
            .count();
        let ship_count = selected_rows
            .iter()
            .filter(|row| {
                matches!(
                    row.kind,
                    ProductionItemKind::Destroyer
                        | ProductionItemKind::Cruiser
                        | ProductionItemKind::Battleship
                        | ProductionItemKind::Scout
                        | ProductionItemKind::Transport
                        | ProductionItemKind::Etac
                )
            })
            .count();
        if starbase_count > 1 || (starbase_count == 1 && ship_count > 0) {
            self.planet.commission_status =
                Some("Select either ships for one fleet or one starbase by itself.".to_string());
            return Ok(());
        }
        if ship_count > 0 {
            self.load_planet_commission_draft_for_current_planet();
            self.current_screen = ScreenId::PlanetCommissionDraft;
            return Ok(());
        }

        let commission_title = self.current_planet_commission_title()?;
        let planet_record = self
            .current_commission_planet_row()?
            .planet_record_index_1_based;
        let result = match self.game_data.commission_planet_stardock_slots(
            self.player.record_index_1_based,
            planet_record,
            &selected_slots,
        ) {
            Ok(result) => result,
            Err(GameStateMutationError::InvalidCommissionSelection) => {
                self.planet.commission_status = Some(
                    "Select either ships for one fleet or one starbase by itself.".to_string(),
                );
                return Ok(());
            }
            Err(err) => return Err(err.into()),
        };
        self.save_game_data()?;
        let result_notice = match result {
            CommissionResult::Fleet {
                fleet_record_index_1_based,
            } => {
                self.remember_newly_commissioned_fleet_record(fleet_record_index_1_based);
                let fleet = self
                    .game_data
                    .fleets
                    .records
                    .get(fleet_record_index_1_based - 1)
                    .ok_or("commissioned fleet record missing")?;
                let max_fleet_number = self
                    .game_data
                    .fleets
                    .records
                    .iter()
                    .filter(|fleet| {
                        fleet.owner_empire_raw() as usize == self.player.record_index_1_based
                    })
                    .map(|fleet| fleet.local_slot_word_raw())
                    .max()
                    .unwrap_or(fleet.local_slot_word_raw());
                format!(
                    "Fleet {} Commissioned",
                    format_fleet_number(fleet.local_slot_word_raw(), max_fleet_number)
                )
            }
            CommissionResult::Starbase {
                base_record_index_1_based,
            } => {
                let base = self
                    .game_data
                    .bases
                    .records
                    .get(base_record_index_1_based - 1)
                    .ok_or("commissioned starbase record missing")?;
                format!("Commissioned Starbase {:02}.", base.base_id_raw())
            }
        };
        self.planet.commission_status = None;
        self.clear_planet_commission_draft_state();
        self.planet.commission_result_title = Some(commission_title);
        self.planet.commission_result_return_to_picker = false;
        self.planet.commission_result_notice = Some(result_notice);

        let planet_rows = self.commission_planet_rows();
        if planet_rows.is_empty() {
            self.planet.commission_index = 0;
            self.planet.commission_cursor = 0;
            self.planet.commission_scroll_offset = 0;
            self.planet.commission_picker_scroll_offset = 0;
        } else {
            self.planet.commission_index = self.planet.commission_index.min(planet_rows.len() - 1);
            let visible_rows = self.planet_commission_picker_visible_rows();
            sync_scroll_to_cursor(
                &mut self.planet.commission_picker_scroll_offset,
                self.planet.commission_index,
                visible_rows,
            );
        }
        self.planet.commission_selected_slots.clear();
        self.current_screen = ScreenId::PlanetCommissionResult;
        Ok(())
    }

    fn commission_current_planet_direct_stardock_slot(
        &mut self,
        slot_0_based: usize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let commission_title = self.current_planet_commission_title()?;
        let planet_record = self
            .current_commission_planet_row()?
            .planet_record_index_1_based;
        let result = self.game_data.commission_planet_stardock_slots(
            self.player.record_index_1_based,
            planet_record,
            &[slot_0_based],
        )?;
        self.save_game_data()?;

        let result_notice = match result {
            CommissionResult::Fleet {
                fleet_record_index_1_based,
            } => {
                self.remember_newly_commissioned_fleet_record(fleet_record_index_1_based);
                self.commission_fleet_notice(fleet_record_index_1_based)?
            }
            CommissionResult::Starbase {
                base_record_index_1_based,
            } => {
                let base = self
                    .game_data
                    .bases
                    .records
                    .get(base_record_index_1_based - 1)
                    .ok_or("commissioned starbase record missing")?;
                format!("Commissioned Starbase {:02}.", base.base_id_raw())
            }
        };

        self.clear_planet_commission_draft_state();
        self.planet.commission_status = None;
        self.planet.commission_result_title = Some(commission_title);
        self.planet.commission_result_return_to_picker = false;
        self.planet.commission_result_notice = Some(result_notice);
        self.current_screen = ScreenId::PlanetCommissionResult;
        Ok(())
    }

    pub fn submit_planet_commission_draft(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.current_screen != ScreenId::PlanetCommissionDraft {
            return Ok(());
        }
        if !self.commit_current_planet_commission_draft_input()? {
            return Ok(());
        }

        let draft = self.current_planet_commission_draft()?;
        let current_row = self
            .planet
            .commission_draft_rows
            .get(self.planet.commission_draft_cursor)
            .cloned();
        if draft.total_ships() == 0 {
            if let Some(slot_0_based) = current_row.and_then(|row| row.direct_slot_0_based) {
                return self.commission_current_planet_direct_stardock_slot(slot_0_based);
            }
        }
        if draft.total_ships() == 0 {
            self.planet.commission_draft_status =
                Some("Select at least one ship for this fleet.".to_string());
            return Ok(());
        }

        let draft_title = self.current_planet_commission_draft_title()?;
        let planet_record = self
            .current_commission_planet_row()?
            .planet_record_index_1_based;
        let result = self.game_data.commission_planet_stardock_slots_with_draft(
            self.player.record_index_1_based,
            planet_record,
            &self.planet.commission_draft_slots,
            draft,
        )?;
        self.save_game_data()?;

        let fleet_record_index_1_based = match result {
            CommissionResult::Fleet {
                fleet_record_index_1_based,
            } => fleet_record_index_1_based,
            CommissionResult::Starbase { .. } => {
                return Err("ship draft unexpectedly commissioned a starbase".into());
            }
        };
        self.remember_newly_commissioned_fleet_record(fleet_record_index_1_based);
        let notice = self.commission_fleet_notice(fleet_record_index_1_based)?;
        let remaining_entries = self
            .current_planet_commission_rows_for(planet_record)
            .into_iter()
            .map(|row| nc_engine::PlanetCommissionSlotEntry {
                slot_0_based: row.slot_0_based,
                kind: row.kind,
                qty: row.qty,
            })
            .collect::<Vec<_>>();
        let draft_state = planet_commission_draft_state(&remaining_entries);
        self.planet.commission_selected_slots.clear();
        self.planet.commission_draft_input.clear();
        self.planet.commission_draft_status = None;

        let has_remaining_ships = draft_state.rows.iter().any(|row| {
            matches!(
                row.kind,
                ProductionItemKind::Destroyer
                    | ProductionItemKind::Cruiser
                    | ProductionItemKind::Battleship
                    | ProductionItemKind::Scout
                    | ProductionItemKind::Transport
                    | ProductionItemKind::Etac
            ) && row.direct_slot_0_based.is_none()
        });
        if !has_remaining_ships {
            self.clear_planet_commission_draft_state();
            self.planet.commission_result_title = Some(draft_title);
            self.planet.commission_result_return_to_picker = true;
            self.planet.commission_result_notice = Some(notice);
            self.current_screen = ScreenId::PlanetCommissionResult;
            return Ok(());
        }

        self.planet.commission_draft_slots = draft_state.draft_slots;
        self.planet.commission_draft_rows = draft_state
            .rows
            .into_iter()
            .map(|row| {
                let unit_label = build_unit_spec_by_kind(row.kind)
                    .map(|spec| spec.label.to_string())
                    .unwrap_or_else(|| {
                        format!("Unknown (kind {})", production_item_kind_raw(row.kind))
                    });
                PlanetCommissionDraftRow {
                    direct_slot_0_based: row.direct_slot_0_based,
                    kind: row.kind,
                    unit_label,
                    remaining_qty: row.remaining_qty,
                    fleet_qty: row.fleet_qty,
                }
            })
            .collect();
        self.planet.commission_draft_cursor = self
            .planet
            .commission_draft_cursor
            .min(self.planet.commission_draft_rows.len() - 1);
        self.planet.commission_draft_notice = Some(notice);
        Ok(())
    }

    pub fn confirm_planet_auto_commission(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if !self.inline_planet_auto_commission_active_on_current_screen() {
            return Ok(());
        }
        let report = self
            .game_data
            .auto_commission_all_stardock_units(self.player.record_index_1_based)?;
        self.save_game_data()?;
        self.remember_auto_commissioned_fleets(&report);
        self.close_planet_auto_commission_prompt();
        let rows = self.build_auto_commission_report_rows(&report);
        if rows.is_empty() {
            self.show_planet_context_notice("No ships or starbases are waiting in stardock.");
            return Ok(());
        }
        self.planet.auto_commission_report_revealed_rows = rows
            .len()
            .min(self.planet_auto_commission_report_page_rows());
        self.planet.auto_commission_report_rows = rows;
        self.current_screen = ScreenId::PlanetAutoCommissionReport;
        Ok(())
    }

    pub fn advance_planet_auto_commission_report(&mut self) {
        if self.current_screen != ScreenId::PlanetAutoCommissionReport {
            return;
        }
        let total_rows = self.planet.auto_commission_report_rows.len();
        if total_rows == 0 {
            self.clear_planet_auto_commission_report();
            self.current_screen = self.planet_context_screen();
            return;
        }
        if self.planet.auto_commission_report_revealed_rows >= total_rows {
            self.clear_planet_auto_commission_report();
            self.current_screen = self.planet_context_screen();
            return;
        }
        self.planet.auto_commission_report_revealed_rows = usize::min(
            self.planet.auto_commission_report_revealed_rows
                + self.planet_auto_commission_report_page_rows(),
            total_rows,
        );
    }

    fn build_auto_commission_report_rows(&self, report: &AutoCommissionReport) -> Vec<String> {
        let max_fleet_number = self
            .game_data
            .fleets
            .records
            .iter()
            .filter(|fleet| fleet.owner_empire_raw() as usize == self.player.record_index_1_based)
            .map(|fleet| fleet.local_slot_word_raw())
            .max()
            .unwrap_or(1);

        let mut rows = Vec::new();
        for (idx, entry) in report.entries.iter().enumerate() {
            if idx > 0 {
                rows.push(String::new());
            }
            let line = match entry {
                AutoCommissionEntry::Fleet(entry) => {
                    format_auto_commission_fleet_entry(entry, max_fleet_number)
                }
                AutoCommissionEntry::Starbase(entry) => {
                    format_auto_commission_starbase_entry(entry)
                }
            };
            rows.extend(crate::screen::layout::wrap_text(
                &line,
                crate::screen::layout::PLAYFIELD_WIDTH,
                crate::screen::layout::PLAYFIELD_WIDTH,
            ));
        }
        rows
    }

    pub fn toggle_planet_commission_selection(&mut self) {
        if self.current_screen != ScreenId::PlanetCommissionMenu {
            return;
        }
        let rows = self.current_planet_commission_rows();
        let Some(row) = rows.get(self.planet.commission_cursor) else {
            return;
        };
        if self
            .planet
            .commission_selected_slots
            .contains(&row.slot_0_based)
        {
            self.planet
                .commission_selected_slots
                .remove(&row.slot_0_based);
        } else {
            self.planet
                .commission_selected_slots
                .insert(row.slot_0_based);
        }
        self.planet.commission_status = None;
    }

    pub fn scroll_planet_build_list(&mut self, delta: i8) {
        if self.current_screen != ScreenId::PlanetBuildList {
            return;
        }
        let total = self.planet_build_list_rows().len();
        let max_offset = total.saturating_sub(self.planet_build_list_visible_rows());
        self.planet.build_list_scroll_offset = self
            .planet
            .build_list_scroll_offset
            .saturating_add_signed(delta as isize)
            .min(max_offset);
    }

    pub fn move_planet_build_list_cursor(&mut self, delta: i8) {
        if self.current_screen != ScreenId::PlanetBuildList {
            return;
        }
        let total = self.planet_build_list_rows().len();
        if total == 0 {
            self.planet.build_list_cursor = 0;
            return;
        }
        let next = self.planet.build_list_cursor as isize + delta as isize;
        self.planet.build_list_cursor = next.rem_euclid(total as isize) as usize;
        // Keep scroll window in sync: ensure cursor is visible.
        if self.planet.build_list_cursor < self.planet.build_list_scroll_offset {
            self.planet.build_list_scroll_offset = self.planet.build_list_cursor;
        } else {
            let visible_rows = self.planet_build_list_visible_rows();
            if self.planet.build_list_cursor >= self.planet.build_list_scroll_offset + visible_rows
            {
                self.planet.build_list_scroll_offset =
                    self.planet.build_list_cursor + 1 - visible_rows;
            }
        }
    }

    pub fn delete_planet_build_slot_request(&mut self) {
        if self.current_screen != ScreenId::PlanetBuildList {
            return;
        }
        let rows = self.planet_build_list_rows();
        let Some(row) = rows.get(self.planet.build_list_cursor) else {
            return;
        };
        if row.queue_qty == 0 {
            return;
        }
        self.reset_planet_build_list_delete_state();
        self.planet.build_list_delete_qty_prompt_active = true;
    }

    pub fn append_delete_build_qty_char(&mut self, ch: char) {
        if !self.planet.build_list_delete_qty_prompt_active || !ch.is_ascii_digit() {
            return;
        }
        let rows = self.planet_build_list_rows();
        let max_digits = rows
            .get(self.planet.build_list_cursor)
            .map(|row| row.queue_qty.to_string().len())
            .unwrap_or(1);
        if self.planet.build_list_delete_qty_input.len() >= max_digits {
            return;
        }
        self.planet.build_list_delete_qty_input.push(ch);
        self.planet.build_list_delete_qty_status = None;
    }

    pub fn backspace_delete_build_qty_input(&mut self) {
        if !self.planet.build_list_delete_qty_prompt_active {
            return;
        }
        self.planet.build_list_delete_qty_input.pop();
        self.planet.build_list_delete_qty_status = None;
    }

    pub fn submit_delete_build_qty(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if !self.planet.build_list_delete_qty_prompt_active {
            return Ok(());
        }
        let rows = self.planet_build_list_rows();
        let Some(row) = rows.get(self.planet.build_list_cursor) else {
            self.reset_planet_build_list_delete_state();
            return Ok(());
        };
        let max_qty = row.queue_qty;
        if max_qty == 0 {
            self.reset_planet_build_list_delete_state();
            return Ok(());
        }
        let input = self.planet.build_list_delete_qty_input.trim();
        let quantity = if input.is_empty() {
            max_qty
        } else if let Ok(value) = input.parse::<u32>() {
            value
        } else {
            self.planet.build_list_delete_qty_status =
                Some(format!("Enter 1-{max_qty}, or press Enter for All."));
            return Ok(());
        };
        if quantity == 0 || quantity > max_qty {
            self.planet.build_list_delete_qty_status =
                Some(format!("Enter 1-{max_qty}, or press Enter for All."));
            return Ok(());
        }
        self.planet.build_list_delete_qty_pending = Some(quantity);
        self.planet.build_list_delete_qty_prompt_active = false;
        self.planet.build_list_delete_qty_status = None;
        self.planet.build_list_confirming = true;
        Ok(())
    }

    pub fn confirm_delete_planet_build_slot(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if !self.planet.build_list_confirming {
            return Ok(());
        }
        let rows = self.planet_build_list_rows();
        let Some(row) = rows.get(self.planet.build_list_cursor) else {
            self.reset_planet_build_list_delete_state();
            return Ok(());
        };
        let planet_record = self.current_build_planet_row()?.planet_record_index_1_based;
        let quantity = self
            .planet
            .build_list_delete_qty_pending
            .unwrap_or(row.queue_qty);
        if let Some(unit) = build_unit_spec_by_kind(row.kind) {
            self.game_data.remove_planet_build_points_by_kind(
                planet_record,
                row.kind,
                quantity.saturating_mul(unit.cost),
            )?;
        } else {
            self.game_data
                .clear_planet_build_orders_by_kind(planet_record, row.kind)?;
        }
        self.save_game_data()?;
        self.reset_planet_build_list_delete_state();
        // Clamp cursor after deletion.
        let new_total = self.planet_build_list_rows().len();
        if new_total == 0 {
            self.planet.build_list_cursor = 0;
        } else {
            self.planet.build_list_cursor = self.planet.build_list_cursor.min(new_total - 1);
        }
        Ok(())
    }

    pub fn cancel_delete_planet_build_slot(&mut self) {
        self.reset_planet_build_list_delete_state();
    }

    pub fn append_planet_build_unit_char(&mut self, ch: char) {
        if self.current_screen == ScreenId::PlanetBuildSpecify
            && self.planet.build_unit_input.len() < 2
        {
            self.planet.build_unit_input.push(ch);
            self.planet.build_unit_status = None;
            self.planet.build_unit_notice = None;
        }
    }

    pub fn backspace_planet_build_unit_input(&mut self) {
        if self.current_screen == ScreenId::PlanetBuildSpecify {
            self.planet.build_unit_input.pop();
            self.planet.build_unit_status = None;
            self.planet.build_unit_notice = None;
        }
    }

    pub fn submit_planet_build_unit(&mut self) {
        let raw = self.planet.build_unit_input.trim();
        let number = if raw.is_empty() {
            0
        } else if let Ok(value) = raw.parse::<u8>() {
            value
        } else {
            self.planet.build_unit_status = Some("Enter a valid unit number.".to_string());
            return;
        };

        if number == 0 {
            self.planet.build_unit_notice = None;
            self.current_screen = self.planet_build_return_screen();
            return;
        }

        let Some(unit) = build_unit_spec(number) else {
            self.planet.build_unit_status = Some("That unit is not available.".to_string());
            return;
        };

        let Ok(max_qty) = self.current_planet_build_max_quantity_for(unit.kind) else {
            self.planet.build_unit_status = Some("No points are available to spend.".to_string());
            return;
        };
        if max_qty == 0 {
            self.planet.build_unit_status = Some(
                self.planet_build_unavailable_message(unit.kind)
                    .unwrap_or_else(|_| "No points are available to spend.".to_string()),
            );
            return;
        }

        self.planet.build_selected_kind = Some(unit.kind);
        self.planet.build_quantity_input.clear();
        self.planet.build_quantity_status = None;
        self.planet.build_unit_notice = None;
        self.current_screen = ScreenId::PlanetBuildQuantity;
    }

    pub fn append_planet_build_quantity_char(&mut self, ch: char) {
        if self.current_screen == ScreenId::PlanetBuildQuantity
            && self.planet.build_quantity_input.len() < 3
        {
            self.planet.build_quantity_input.push(ch);
            self.planet.build_quantity_status = None;
        }
    }

    pub fn backspace_planet_build_quantity_input(&mut self) {
        if self.current_screen == ScreenId::PlanetBuildQuantity {
            self.planet.build_quantity_input.pop();
            self.planet.build_quantity_status = None;
        }
    }

    pub fn submit_planet_build_quantity(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let Some(kind) = self.planet.build_selected_kind else {
            self.current_screen = if self.planet.build_return_to_list {
                self.planet_build_return_screen()
            } else {
                ScreenId::PlanetBuildSpecify
            };
            return Ok(());
        };
        let Some(unit) = build_unit_spec_by_kind(kind) else {
            self.current_screen = if self.planet.build_return_to_list {
                self.planet_build_return_screen()
            } else {
                ScreenId::PlanetBuildSpecify
            };
            return Ok(());
        };
        let max_qty = self.current_planet_build_max_quantity_for(kind)?;
        if max_qty == 0 {
            self.planet.build_quantity_status = Some(self.planet_build_unavailable_message(kind)?);
            return Ok(());
        }

        let qty = if self.planet.build_quantity_input.trim().is_empty() {
            max_qty
        } else {
            match self.planet.build_quantity_input.trim().parse::<u32>() {
                Ok(value) => value,
                Err(_) => {
                    self.planet.build_quantity_status = Some("Enter a valid quantity.".to_string());
                    return Ok(());
                }
            }
        };

        if qty == 0 {
            self.current_screen = if self.planet.build_return_to_list {
                self.planet_build_return_screen()
            } else {
                ScreenId::PlanetBuildSpecify
            };
            self.planet.build_quantity_input.clear();
            return Ok(());
        }
        if qty > max_qty {
            self.planet.build_quantity_status =
                Some(format!("Enter a quantity from 0 to {}.", max_qty));
            return Ok(());
        }

        let planet_record = self.current_build_planet_row()?.planet_record_index_1_based;

        // Armies and batteries do not use stardock. For stardock-requiring kinds,
        // the quantity cap above already accounts for queue and stardock capacity.
        let needs_stardock = !matches!(
            kind,
            ProductionItemKind::Army | ProductionItemKind::GroundBattery
        );
        if needs_stardock {
            let capacity = self
                .game_data
                .planet_additional_build_points_capacity(planet_record, kind)?;
            if capacity == 0 {
                self.planet.build_quantity_status =
                    Some("Stardock is full -- commission ships first to free space.".to_string());
                return Ok(());
            }
        }

        let points = qty.saturating_mul(unit.cost);
        match self.game_data.append_planet_build_order(
            planet_record,
            points,
            production_item_kind_raw(kind),
        ) {
            Ok(()) => {}
            Err(GameStateMutationError::PlanetBuildQueueFull { .. }) => {
                self.planet.build_quantity_status =
                    Some("Build queue is full for this planet.".to_string());
                return Ok(());
            }
            Err(e) => return Err(e.into()),
        }
        self.save_game_data()?;
        self.planet.build_unit_input.clear();
        self.planet.build_unit_status = None;
        self.planet.build_unit_notice = Some(format!("Queued {} {}.", qty, unit.label));
        self.planet.build_quantity_input.clear();
        self.planet.build_quantity_status = None;
        self.planet.build_selected_kind = None;
        if self.current_planet_can_afford_any_build() {
            self.current_screen = ScreenId::PlanetBuildSpecify;
        } else {
            self.show_planet_build_budget_exhausted_notice(self.planet.build_return_to_list);
        }
        Ok(())
    }

    pub fn abort_current_planet_builds(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if !self.inline_planet_build_abort_active_on_current_screen() {
            return Ok(());
        }
        let row = self.current_build_planet_row()?;
        self.game_data
            .clear_planet_build_queue(row.planet_record_index_1_based)?;
        self.save_game_data()?;
        self.close_planet_build_abort_prompt();
        if self.planet.build_return_to_list {
            self.clear_command_menu_notice();
            self.planet.list_prompt_status = Some("Build orders aborted.".to_string());
            self.current_screen = self.planet_build_return_screen();
        } else {
            self.planet.build_status = Some("Build orders aborted.".to_string());
            self.current_screen = ScreenId::PlanetBuildMenu;
        }
        Ok(())
    }

    pub(super) fn build_planet_rows(&self) -> Vec<nc_data::EmpirePlanetEconomyRow> {
        self.sorted_planet_rows(PlanetListSort::CurrentProduction)
    }

    pub(super) fn commission_planet_rows(&self) -> Vec<nc_data::EmpirePlanetEconomyRow> {
        self.build_planet_rows()
            .into_iter()
            .filter(|row| {
                self.game_data
                    .planets
                    .records
                    .get(row.planet_record_index_1_based - 1)
                    .map(|record| {
                        (0..STARDOCK_SLOT_COUNT).any(|slot| {
                            let count = record.stardock_count_raw(slot);
                            let kind = ProductionItemKind::from_raw(record.stardock_kind_raw(slot));
                            count > 0 && kind.requires_stardock()
                        })
                    })
                    .unwrap_or(false)
            })
            .collect()
    }

    pub(crate) fn planet_commission_picker_rows(&self) -> Vec<PlanetCommissionPickerRow> {
        self.commission_planet_rows()
            .into_iter()
            .filter_map(|row| {
                let record = self
                    .game_data
                    .planets
                    .records
                    .get(row.planet_record_index_1_based - 1)?;
                let mut destroyers = 0u32;
                let mut cruisers = 0u32;
                let mut battleships = 0u32;
                let mut scouts = 0u32;
                let mut troop_transports = 0u32;
                let mut etacs = 0u32;
                let mut starbases = 0u32;
                for slot in 0..STARDOCK_SLOT_COUNT {
                    let count = u32::from(record.stardock_count_raw(slot));
                    if count == 0 {
                        continue;
                    }
                    match ProductionItemKind::from_raw(record.stardock_kind_raw(slot)) {
                        ProductionItemKind::Destroyer => {
                            destroyers = destroyers.saturating_add(count)
                        }
                        ProductionItemKind::Cruiser => cruisers = cruisers.saturating_add(count),
                        ProductionItemKind::Battleship => {
                            battleships = battleships.saturating_add(count)
                        }
                        ProductionItemKind::Scout => scouts = scouts.saturating_add(count),
                        ProductionItemKind::Transport => {
                            troop_transports = troop_transports.saturating_add(count)
                        }
                        ProductionItemKind::Etac => etacs = etacs.saturating_add(count),
                        ProductionItemKind::Starbase => starbases = starbases.saturating_add(count),
                        ProductionItemKind::Army
                        | ProductionItemKind::GroundBattery
                        | ProductionItemKind::Unknown(_) => {}
                    }
                }
                Some(PlanetCommissionPickerRow {
                    coords: row.coords,
                    planet_name: row.planet_name,
                    destroyers,
                    cruisers,
                    battleships,
                    scouts,
                    troop_transports,
                    etacs,
                    starbases,
                })
            })
            .collect()
    }

    pub(crate) fn current_planet_commission_view(
        &self,
    ) -> Result<PlanetCommissionView, Box<dyn std::error::Error>> {
        let row = match self.current_commission_planet_row() {
            Ok(row) => row,
            Err(_) => {
                return Ok(PlanetCommissionView {
                    planet_name: "No commissionable planets".to_string(),
                    coords: self.default_planet_prompt_coords(),
                    rows: vec![],
                });
            }
        };
        Ok(PlanetCommissionView {
            planet_name: row.planet_name,
            coords: row.coords,
            rows: self.current_planet_commission_rows_for(row.planet_record_index_1_based),
        })
    }

    fn current_planet_commission_rows(&self) -> Vec<PlanetCommissionRow> {
        let Ok(row) = self.current_commission_planet_row() else {
            return vec![];
        };
        self.current_planet_commission_rows_for(row.planet_record_index_1_based)
    }

    fn current_planet_commission_rows_for(
        &self,
        planet_record_index_1_based: usize,
    ) -> Vec<PlanetCommissionRow> {
        let Some(record) = self
            .game_data
            .planets
            .records
            .get(planet_record_index_1_based - 1)
        else {
            return vec![];
        };
        planet_commission_slot_entries(record)
            .into_iter()
            .map(|entry| {
                let unit_label = build_unit_spec_by_kind(entry.kind)
                    .map(|spec| spec.label.to_string())
                    .unwrap_or_else(|| {
                        format!("Unknown (kind {})", production_item_kind_raw(entry.kind))
                    });
                PlanetCommissionRow {
                    slot_0_based: entry.slot_0_based,
                    kind: entry.kind,
                    unit_label,
                    qty: entry.qty,
                }
            })
            .collect()
    }

    fn clear_planet_commission_draft_state(&mut self) {
        self.planet.commission_draft_slots.clear();
        self.planet.commission_draft_rows.clear();
        self.planet.commission_draft_cursor = 0;
        self.planet.commission_draft_input.clear();
        self.planet.commission_draft_status = None;
        self.planet.commission_draft_notice = None;
    }

    fn load_planet_commission_draft_for_current_planet(&mut self) {
        let rows = self.current_planet_commission_rows();
        let entries = rows
            .iter()
            .map(|row| nc_engine::PlanetCommissionSlotEntry {
                slot_0_based: row.slot_0_based,
                kind: row.kind,
                qty: row.qty,
            })
            .collect::<Vec<_>>();
        let draft_state = planet_commission_draft_state(&entries);
        self.planet.commission_draft_slots = draft_state.draft_slots;
        self.planet.commission_draft_rows = draft_state
            .rows
            .into_iter()
            .map(|row| {
                let unit_label = build_unit_spec_by_kind(row.kind)
                    .map(|spec| spec.label.to_string())
                    .unwrap_or_else(|| {
                        format!("Unknown (kind {})", production_item_kind_raw(row.kind))
                    });
                PlanetCommissionDraftRow {
                    direct_slot_0_based: row.direct_slot_0_based,
                    kind: row.kind,
                    unit_label,
                    remaining_qty: row.remaining_qty,
                    fleet_qty: row.fleet_qty,
                }
            })
            .collect();
        self.planet.commission_draft_cursor = 0;
        self.planet.commission_draft_input.clear();
        self.planet.commission_draft_status = None;
        self.planet.commission_draft_notice = None;
        self.planet.commission_status = None;
    }

    fn commit_current_planet_commission_draft_input(
        &mut self,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        if self.current_screen != ScreenId::PlanetCommissionDraft {
            return Ok(false);
        }
        let Some(row) = self
            .planet
            .commission_draft_rows
            .get_mut(self.planet.commission_draft_cursor)
        else {
            self.planet.commission_draft_status =
                Some("No ships remain in this commission draft.".to_string());
            return Ok(false);
        };
        if !row.accepts_fleet_qty() {
            self.planet.commission_draft_input.clear();
            self.planet.commission_draft_status = None;
            return Ok(true);
        }
        let raw = self.planet.commission_draft_input.trim();
        if raw.is_empty() {
            row.fleet_qty = row.remaining_qty;
            self.planet.commission_draft_input.clear();
            self.planet.commission_draft_status = None;
            return Ok(true);
        }
        let qty = match raw.parse::<u16>() {
            Ok(value) => value,
            Err(_) => {
                self.planet.commission_draft_status = Some("Enter a valid quantity.".to_string());
                return Ok(false);
            }
        };
        if qty > row.remaining_qty {
            self.planet.commission_draft_status =
                Some(format!("Enter a quantity from 0 to {}.", row.remaining_qty));
            return Ok(false);
        }
        row.fleet_qty = qty;
        self.planet.commission_draft_input.clear();
        self.planet.commission_draft_status = None;
        Ok(true)
    }

    fn current_planet_commission_draft(
        &self,
    ) -> Result<CommissionFleetDraft, Box<dyn std::error::Error>> {
        let entries = self
            .planet
            .commission_draft_rows
            .iter()
            .map(|row| nc_engine::PlanetCommissionDraftEntry {
                direct_slot_0_based: row.direct_slot_0_based,
                kind: row.kind,
                remaining_qty: row.remaining_qty,
                fleet_qty: row.fleet_qty,
            })
            .collect::<Vec<_>>();
        commission_fleet_draft_from_entries(&entries).map_err(Into::into)
    }

    fn current_planet_commission_title(&self) -> Result<String, Box<dyn std::error::Error>> {
        let row = self.current_commission_planet_row()?;
        Ok(format!(
            "COMMISSION SHIPS: \"{}\" IN SYSTEM {}:",
            row.planet_name,
            format_sector_coords(row.coords)
        ))
    }

    pub(crate) fn current_planet_commission_draft_title(
        &self,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let row = self.current_commission_planet_row()?;
        Ok(format!(
            "DRAFT COMMISSION FLEET: \"{}\" IN SYSTEM {}:",
            row.planet_name,
            format_sector_coords(row.coords)
        ))
    }

    fn commission_fleet_notice(
        &self,
        fleet_record_index_1_based: usize,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let fleet = self
            .game_data
            .fleets
            .records
            .get(fleet_record_index_1_based - 1)
            .ok_or("commissioned fleet record missing")?;
        let max_fleet_number = self
            .game_data
            .fleets
            .records
            .iter()
            .filter(|fleet| fleet.owner_empire_raw() as usize == self.player.record_index_1_based)
            .map(|fleet| fleet.local_slot_word_raw())
            .max()
            .unwrap_or(fleet.local_slot_word_raw());
        Ok(format!(
            "Fleet {} Commissioned",
            format_fleet_number(fleet.local_slot_word_raw(), max_fleet_number)
        ))
    }

    fn owned_fleet_record_index_by_local_slot(&self, fleet_number: u16) -> Option<usize> {
        self.game_data
            .fleets
            .records
            .iter()
            .enumerate()
            .find(|(_, fleet)| {
                fleet.owner_empire_raw() as usize == self.player.record_index_1_based
                    && fleet.local_slot_word_raw() == fleet_number
            })
            .map(|(idx, _)| idx + 1)
    }

    fn remember_auto_commissioned_fleets(&mut self, report: &AutoCommissionReport) {
        for fleet_number in report.entries.iter().filter_map(|entry| match entry {
            AutoCommissionEntry::Fleet(entry) => Some(entry.fleet_number),
            AutoCommissionEntry::Starbase(_) => None,
        }) {
            if let Some(fleet_record_index_1_based) =
                self.owned_fleet_record_index_by_local_slot(fleet_number)
            {
                self.remember_newly_commissioned_fleet_record(fleet_record_index_1_based);
            }
        }
    }

    pub(crate) fn build_change_rows(&self) -> Vec<PlanetBuildChangeRow> {
        self.build_planet_rows()
            .into_iter()
            .map(|row| {
                let available_points = u32::from(row.build_capacity)
                    .min(row.stored_production_points.min(u32::from(u16::MAX)));
                let committed_points = self
                    .current_build_committed_points(row.planet_record_index_1_based)
                    .unwrap_or(0);
                PlanetBuildChangeRow {
                    planet_name: row.planet_name,
                    coords: row.coords,
                    present_production: row.present_production,
                    potential_production: row.potential_production,
                    budget: available_points,
                    committed_points,
                }
            })
            .collect()
    }

    fn current_build_planet_row(
        &self,
    ) -> Result<nc_data::EmpirePlanetEconomyRow, Box<dyn std::error::Error>> {
        self.build_planet_rows()
            .get(self.planet.build_index)
            .cloned()
            .ok_or_else(|| "current build planet missing".into())
    }

    pub(crate) fn current_planet_build_orders(&self) -> Vec<PlanetBuildOrder> {
        let Ok(row) = self.current_build_planet_row() else {
            return vec![];
        };
        let Some(record) = self
            .game_data
            .planets
            .records
            .get(row.planet_record_index_1_based - 1)
        else {
            return vec![];
        };
        planet_build_orders(record)
            .into_iter()
            .map(|order| PlanetBuildOrder {
                kind: order.kind,
                points_remaining: order.points_remaining,
            })
            .collect()
    }

    pub(crate) fn current_planet_build_view(
        &self,
    ) -> Result<PlanetBuildMenuView, Box<dyn std::error::Error>> {
        let row = match self.current_build_planet_row() {
            Ok(row) => row,
            Err(_) => {
                return Ok(PlanetBuildMenuView {
                    row: nc_data::EmpirePlanetEconomyRow {
                        planet_record_index_1_based: 0,
                        coords: self.default_planet_prompt_coords(),
                        planet_name: "No owned planets".to_string(),
                        present_production: 0,
                        potential_production: 0,
                        stored_production_points: 0,
                        yearly_tax_revenue: 0,
                        yearly_growth_delta: 0,
                        build_capacity: 0,
                        has_friendly_starbase: false,
                        armies: 0,
                        ground_batteries: 0,
                        is_homeworld_seed: false,
                    },
                    committed_points: 0,
                    budget: 0,
                    points_left: 0,
                    building_count: 0,
                    docked_count: 0,
                });
            }
        };
        let view_stats = planet_build_view(&self.game_data, &row)?;
        Ok(PlanetBuildMenuView {
            row,
            committed_points: view_stats.committed_points,
            budget: view_stats.budget,
            points_left: view_stats.points_left,
            building_count: view_stats.building_count,
            docked_count: view_stats.docked_count,
        })
    }

    fn current_build_committed_points(
        &self,
        planet_record_index_1_based: usize,
    ) -> Result<u32, Box<dyn std::error::Error>> {
        let record = self
            .game_data
            .planets
            .records
            .get(planet_record_index_1_based - 1)
            .ok_or("planet record missing")?;
        Ok(planet_build_committed_points(record))
    }

    pub(crate) fn current_planet_build_max_quantity(
        &self,
    ) -> Result<u32, Box<dyn std::error::Error>> {
        let kind = self
            .planet
            .build_selected_kind
            .ok_or("planet build kind missing")?;
        self.current_planet_build_max_quantity_for(kind)
    }

    fn current_planet_build_max_quantity_for(
        &self,
        kind: ProductionItemKind,
    ) -> Result<u32, Box<dyn std::error::Error>> {
        let view = self.current_planet_build_view()?;
        planet_build_max_quantity(&self.game_data, &view.row, kind).map_err(Into::into)
    }

    fn current_planet_can_afford_any_build(&self) -> bool {
        let Ok(view) = self.current_planet_build_view() else {
            return false;
        };
        if view.row.planet_record_index_1_based == 0 {
            return false;
        }
        planet_has_any_buildable_unit(&self.game_data, &view.row).unwrap_or(false)
    }

    fn show_planet_build_budget_exhausted_notice(&mut self, return_to_list: bool) {
        self.planet.build_status = None;
        self.planet.build_unit_input.clear();
        self.planet.build_unit_status = None;
        self.planet.build_unit_notice = None;
        self.planet.build_quantity_input.clear();
        self.planet.build_quantity_status = None;
        self.planet.build_selected_kind = None;
        if return_to_list {
            self.show_planet_context_notice(Self::PLANET_BUILD_BUDGET_EXHAUSTED_NOTICE);
        } else {
            self.planet.build_status = Some(Self::PLANET_BUILD_BUDGET_EXHAUSTED_NOTICE.to_string());
            self.current_screen = ScreenId::PlanetBuildMenu;
        }
    }

    fn planet_build_unavailable_message(
        &self,
        kind: ProductionItemKind,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let view = self.current_planet_build_view()?;
        Ok(planet_build_unavailable_message(view.points_left, kind).to_string())
    }

    pub(crate) fn planet_build_list_rows(&self) -> Vec<PlanetBuildListRow> {
        let Ok(row) = self.current_build_planet_row() else {
            return vec![];
        };
        let Some(record) = self
            .game_data
            .planets
            .records
            .get(row.planet_record_index_1_based - 1)
        else {
            return vec![];
        };
        planet_build_list_entries(record)
            .into_iter()
            .map(|entry| {
                let unit_label = build_unit_spec_by_kind(entry.kind)
                    .map(|u| u.label.to_string())
                    .unwrap_or_else(|| {
                        format!("Unknown (kind {})", production_item_kind_raw(entry.kind))
                    });
                PlanetBuildListRow {
                    kind: entry.kind,
                    unit_label,
                    points: entry.points,
                    queue_qty: entry.queue_qty,
                }
            })
            .collect()
    }

    fn reset_planet_build_list_delete_state(&mut self) {
        self.planet.build_list_confirming = false;
        self.planet.build_list_delete_qty_prompt_active = false;
        self.planet.build_list_delete_qty_input.clear();
        self.planet.build_list_delete_qty_status = None;
        self.planet.build_list_delete_qty_pending = None;
    }
}

fn format_auto_commission_fleet_entry(
    entry: &AutoCommissionFleetEntry,
    max_fleet_number: u16,
) -> String {
    let mut composition = Vec::new();
    push_auto_commission_ship_code(&mut composition, "DD", entry.destroyers);
    push_auto_commission_ship_code(&mut composition, "CA", entry.cruisers);
    push_auto_commission_ship_code(&mut composition, "BB", entry.battleships);
    push_auto_commission_ship_code(&mut composition, "SC", entry.scouts);
    push_auto_commission_ship_code(&mut composition, "TT", entry.transports);
    push_auto_commission_ship_code(&mut composition, "ET", entry.etacs);
    format!(
        "Fleet {} commissioned from \"{}\" in sector {} with {}.",
        format_fleet_number(entry.fleet_number, max_fleet_number),
        entry.planet_name,
        format_sector_coords_table(entry.coords),
        composition.join(", "),
    )
}

fn format_auto_commission_starbase_entry(entry: &AutoCommissionStarbaseEntry) -> String {
    format!(
        "Starbase {:02} commissioned to \"{}\" in sector {}.",
        entry.starbase_number,
        entry.planet_name,
        format_sector_coords_table(entry.coords),
    )
}

fn push_auto_commission_ship_code(parts: &mut Vec<String>, code: &str, qty: u32) {
    if qty == 0 {
        return;
    }
    parts.push(format!("{code} {qty:02}"));
}
