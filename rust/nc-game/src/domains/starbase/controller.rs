use crate::app::helpers::{
    center_scroll_to_cursor, is_coordinate_input_char, resolve_default_coords_input,
    sync_scroll_to_cursor,
};
use crate::app::state::App;
use crate::domains::starbase::StarbaseAction;
use crate::domains::starbase::state::StarbaseMovePromptMode;
use crate::screen::{
    CommandMenu, ScreenId, StarbaseRow, format_sector_coords_default, format_sector_coords_table,
};
use nc_data::map_size_for_player_count;
use nc_engine::{format_guard_fleet_clause, guard_fleet_numbers_for_starbase};

impl App {
    fn starbase_visible_rows(&self) -> usize {
        crate::domains::starbase::screens::starbase::starbase_visible_rows(self.screen_geometry)
    }

    pub fn open_starbase_menu(&mut self) {
        self.clear_command_menu_notice();
        self.clear_starbase_move_prompt();
        self.current_screen = ScreenId::StarbaseMenu;
    }

    pub fn open_starbase_list(&mut self) {
        let total = self.starbase_rows().len();
        if total == 0 {
            self.show_command_menu_notice(CommandMenu::Starbase, "You have no active starbases.");
            return;
        }
        self.clear_command_menu_notice();
        self.clear_starbase_move_prompt();
        self.starbase.cursor = self.starbase.cursor.min(total - 1);
        let visible_rows = self.starbase_visible_rows();
        center_scroll_to_cursor(
            &mut self.starbase.scroll_offset,
            self.starbase.cursor,
            visible_rows,
            total,
        );
        self.current_screen = ScreenId::StarbaseList;
    }

    pub fn open_starbase_review_select(&mut self) {
        let total = self.starbase_rows().len();
        if total == 0 {
            self.show_command_menu_notice(CommandMenu::Starbase, "You have no active starbases.");
            return;
        }
        self.clear_command_menu_notice();
        self.clear_starbase_move_prompt();
        self.starbase.cursor = self.starbase.cursor.min(total - 1);
        self.starbase.review_input.clear();
        self.starbase.review_status = None;
        let visible_rows = self.starbase_visible_rows();
        center_scroll_to_cursor(
            &mut self.starbase.scroll_offset,
            self.starbase.cursor,
            visible_rows,
            total,
        );
        self.current_screen = ScreenId::StarbaseReviewSelect;
    }

    pub fn open_starbase_review(&mut self) {
        let rows = self.starbase_rows();
        if rows.is_empty() {
            self.show_command_menu_notice(CommandMenu::Starbase, "You have no active starbases.");
            return;
        }
        let Some(_) = rows.get(self.starbase.cursor) else {
            self.current_screen = ScreenId::StarbaseMenu;
            return;
        };
        self.starbase.review_index = self.starbase.cursor;
        self.current_screen = ScreenId::StarbaseReview;
    }

    pub fn open_starbase_move_prompt(&mut self) {
        let Some(default_base_id) = self.default_starbase_move_base_id() else {
            self.show_command_menu_notice(CommandMenu::Starbase, "You have no active starbases.");
            return;
        };
        self.clear_command_menu_notice();
        self.close_planet_info_prompt();
        self.open_starbase_move_prompt_mode(
            StarbaseMovePromptMode::Base,
            default_base_id.to_string(),
        );
        self.current_screen = ScreenId::StarbaseMenu;
    }

    pub fn move_starbase_select(&mut self, delta: i8) {
        if !matches!(
            self.current_screen,
            ScreenId::StarbaseList | ScreenId::StarbaseReviewSelect
        ) {
            return;
        }
        self.starbase.move_select(
            delta,
            &self.game_data,
            self.player.record_index_1_based,
            self.starbase_visible_rows(),
        );
    }

    pub fn append_starbase_char(&mut self, ch: char) {
        if self.current_screen != ScreenId::StarbaseReviewSelect || !ch.is_ascii_digit() {
            return;
        }
        if self.starbase.review_input.len() >= 3 {
            return;
        }
        self.starbase.append_char(
            ch,
            &self.game_data,
            self.player.record_index_1_based,
            self.starbase_visible_rows(),
        );
    }

    pub fn backspace_starbase_input(&mut self) {
        if self.current_screen != ScreenId::StarbaseReviewSelect {
            return;
        }
        self.starbase.backspace_input(
            &self.game_data,
            self.player.record_index_1_based,
            self.starbase_visible_rows(),
        );
    }

    pub fn submit_starbase_review_select(&mut self) {
        if self.current_screen != ScreenId::StarbaseReviewSelect {
            return;
        }
        let rows = self.starbase_rows();
        let Some(_) = rows.get(self.starbase.cursor) else {
            self.current_screen = ScreenId::StarbaseMenu;
            return;
        };
        if !self.starbase.review_input.trim().is_empty() {
            let target_base_id = match self.starbase.review_input.trim().parse::<u16>() {
                Ok(value) => value,
                Err(_) => {
                    self.starbase.review_status =
                        Some("Enter a starbase number from the table.".to_string());
                    return;
                }
            };
            let Some(index) = rows.iter().position(|row| row.base_id == target_base_id) else {
                self.starbase.review_status =
                    Some(format!("Starbase #{target_base_id} is not in your list."));
                return;
            };
            self.starbase.cursor = index;
            let visible_rows = self.starbase_visible_rows();
            sync_scroll_to_cursor(
                &mut self.starbase.scroll_offset,
                self.starbase.cursor,
                visible_rows,
            );
        }
        self.starbase.review_input.clear();
        self.starbase.review_status = None;
        self.open_starbase_review();
    }

    pub(crate) fn starbase_rows(&self) -> Vec<StarbaseRow> {
        self.starbase
            .starbase_rows(&self.game_data, self.player.record_index_1_based)
    }

    pub(crate) fn inline_starbase_move_prompt_active_on_current_screen(&self) -> bool {
        self.current_screen == ScreenId::StarbaseMenu && self.starbase.move_prompt_mode.is_some()
    }

    pub(crate) fn handle_starbase_review_select_key(
        &self,
        key: crossterm::event::KeyEvent,
    ) -> crate::app::Action {
        use crossterm::event::KeyCode;

        match key.code {
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                crate::app::Action::Starbase(StarbaseAction::MoveSelect(-1))
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                crate::app::Action::Starbase(StarbaseAction::MoveSelect(1))
            }
            KeyCode::PageUp => crate::app::Action::Starbase(StarbaseAction::MoveSelect(-8)),
            KeyCode::PageDown => crate::app::Action::Starbase(StarbaseAction::MoveSelect(8)),
            KeyCode::Enter => crate::app::Action::Starbase(StarbaseAction::SubmitReviewSelect),
            KeyCode::Backspace => crate::app::Action::Starbase(StarbaseAction::BackspaceInput),
            KeyCode::Char(ch) if ch.is_ascii_digit() => {
                crate::app::Action::Starbase(StarbaseAction::AppendChar(ch))
            }
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                crate::app::Action::Starbase(StarbaseAction::OpenMenu)
            }
            _ => crate::app::Action::Noop,
        }
    }

    pub(crate) fn handle_starbase_move_prompt_key(
        &self,
        key: crossterm::event::KeyEvent,
    ) -> crate::app::Action {
        use crossterm::event::KeyCode;

        let mode = self.starbase.move_prompt_mode;
        match key.code {
            KeyCode::Enter => crate::app::Action::Starbase(StarbaseAction::SubmitMovePrompt),
            KeyCode::Backspace => {
                crate::app::Action::Starbase(StarbaseAction::BackspaceMovePromptInput)
            }
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                crate::app::Action::Starbase(StarbaseAction::CancelMovePrompt)
            }
            KeyCode::Char('n') | KeyCode::Char('N')
                if mode == Some(StarbaseMovePromptMode::HaltConfirm) =>
            {
                crate::app::Action::Starbase(StarbaseAction::CancelMovePrompt)
            }
            KeyCode::Char('y') | KeyCode::Char('Y')
                if mode == Some(StarbaseMovePromptMode::HaltConfirm) =>
            {
                crate::app::Action::Starbase(StarbaseAction::SubmitMovePrompt)
            }
            KeyCode::Char(ch)
                if match mode {
                    Some(StarbaseMovePromptMode::Base) => ch.is_ascii_digit(),
                    Some(StarbaseMovePromptMode::Decision) => ch.is_ascii_alphabetic(),
                    Some(StarbaseMovePromptMode::Destination) => is_coordinate_input_char(ch),
                    Some(StarbaseMovePromptMode::HaltConfirm) | None => false,
                } =>
            {
                crate::app::Action::Starbase(StarbaseAction::AppendMovePromptChar(ch))
            }
            _ => crate::app::Action::Noop,
        }
    }

    pub(crate) fn starbase_move_prompt_mode(&self) -> Option<StarbaseMovePromptMode> {
        self.starbase.move_prompt_mode
    }

    pub(crate) fn starbase_move_prompt_label(&self) -> Option<&'static str> {
        match self.starbase.move_prompt_mode? {
            StarbaseMovePromptMode::Base => Some("Starbase # "),
            StarbaseMovePromptMode::Decision => Some("Choose <H>alt or <M>ove "),
            StarbaseMovePromptMode::Destination => Some("Destination "),
            StarbaseMovePromptMode::HaltConfirm => None,
        }
    }

    pub(crate) fn append_starbase_move_prompt_char(&mut self, ch: char) {
        if !self.inline_starbase_move_prompt_active_on_current_screen() {
            return;
        }
        let max_len = match self.starbase.move_prompt_mode {
            Some(StarbaseMovePromptMode::Base) => 3,
            Some(StarbaseMovePromptMode::Decision) => 1,
            Some(StarbaseMovePromptMode::Destination) => 16,
            Some(StarbaseMovePromptMode::HaltConfirm) | None => 0,
        };
        if self.starbase.move_prompt_input.len() < max_len {
            self.starbase.move_prompt_input.push(ch);
            self.starbase.move_prompt_status = None;
        }
    }

    pub(crate) fn backspace_starbase_move_prompt_input(&mut self) {
        if !self.inline_starbase_move_prompt_active_on_current_screen() {
            return;
        }
        self.starbase.move_prompt_input.pop();
        self.starbase.move_prompt_status = None;
    }

    pub(crate) fn cancel_starbase_move_prompt(&mut self) {
        let Some(mode) = self.starbase.move_prompt_mode else {
            return;
        };
        match mode {
            StarbaseMovePromptMode::Base => self.clear_starbase_move_prompt(),
            StarbaseMovePromptMode::Decision => self.clear_starbase_move_prompt(),
            StarbaseMovePromptMode::Destination | StarbaseMovePromptMode::HaltConfirm => {
                self.open_starbase_move_prompt_mode(
                    StarbaseMovePromptMode::Decision,
                    "M".to_string(),
                );
            }
        }
        self.current_screen = ScreenId::StarbaseMenu;
    }

    pub(crate) fn submit_starbase_move_prompt(&mut self) -> Result<(), String> {
        let Some(mode) = self.starbase.move_prompt_mode else {
            return Ok(());
        };
        match mode {
            StarbaseMovePromptMode::Base => {
                let raw = self.starbase_move_prompt_value();
                let base_id = raw
                    .parse::<u16>()
                    .map_err(|_| "Enter one of your starbase numbers.".to_string())?;
                let row = self.resolve_starbase_move_base_row(base_id)?;
                self.starbase.move_prompt_base_record_index_1_based =
                    Some(row.base_record_index_1_based);
                self.open_starbase_move_prompt_mode(
                    StarbaseMovePromptMode::Decision,
                    "M".to_string(),
                );
            }
            StarbaseMovePromptMode::Decision => {
                let raw = self.starbase_move_prompt_value();
                let Some(choice) = raw.chars().next().map(|ch| ch.to_ascii_uppercase()) else {
                    return Err("Choose H or M.".to_string());
                };
                match choice {
                    'H' => self.open_starbase_move_prompt_mode(
                        StarbaseMovePromptMode::HaltConfirm,
                        String::new(),
                    ),
                    'M' => {
                        let row = self
                            .selected_starbase_move_row()
                            .ok_or_else(|| "Choose one of your starbases first.".to_string())?;
                        self.open_starbase_move_prompt_mode(
                            StarbaseMovePromptMode::Destination,
                            format_sector_coords_default(row.destination_coords),
                        );
                    }
                    _ => return Err("Choose H or M.".to_string()),
                }
            }
            StarbaseMovePromptMode::Destination => {
                let row = self
                    .selected_starbase_move_row()
                    .ok_or_else(|| "Choose one of your starbases first.".to_string())?;
                let Some(destination) = resolve_default_coords_input(
                    &self.starbase.move_prompt_input,
                    row.destination_coords,
                ) else {
                    return Err("Enter coordinates like 10,13".to_string());
                };
                let map_size = map_size_for_player_count(self.game_data.conquest.player_count);
                if destination[0] == 0
                    || destination[1] == 0
                    || destination[0] > map_size
                    || destination[1] > map_size
                {
                    return Err(format!("Enter coordinates within 1..{map_size}"));
                }
                self.finalize_starbase_destination(row, destination)?;
            }
            StarbaseMovePromptMode::HaltConfirm => {
                let row = self
                    .selected_starbase_move_row()
                    .ok_or_else(|| "Choose one of your starbases first.".to_string())?;
                let destination = row.coords;
                self.finalize_starbase_destination(row, destination)?;
            }
        }
        Ok(())
    }

    fn open_starbase_move_prompt_mode(
        &mut self,
        mode: StarbaseMovePromptMode,
        default_value: String,
    ) {
        self.starbase.move_prompt_mode = Some(mode);
        self.starbase.move_prompt_input.clear();
        self.starbase.move_prompt_status = None;
        self.starbase.move_prompt_default_value = default_value;
    }

    pub(crate) fn clear_starbase_move_prompt(&mut self) {
        self.starbase.move_prompt_mode = None;
        self.starbase.move_prompt_input.clear();
        self.starbase.move_prompt_status = None;
        self.starbase.move_prompt_default_value.clear();
        self.starbase.move_prompt_base_record_index_1_based = None;
    }

    fn default_starbase_move_base_id(&self) -> Option<u16> {
        let rows = self.starbase_rows();
        if rows.is_empty() {
            return None;
        }
        rows.get(self.starbase.cursor.min(rows.len() - 1))
            .or_else(|| rows.first())
            .map(|row| row.base_id)
    }

    fn starbase_move_prompt_value(&self) -> String {
        if self.starbase.move_prompt_input.trim().is_empty() {
            self.starbase.move_prompt_default_value.trim().to_string()
        } else {
            self.starbase.move_prompt_input.trim().to_string()
        }
    }

    fn resolve_starbase_move_base_row(&mut self, base_id: u16) -> Result<StarbaseRow, String> {
        let rows = self.starbase_rows();
        let Some(index) = rows.iter().position(|row| row.base_id == base_id) else {
            return Err(format!("Starbase #{base_id} is not in your list."));
        };
        self.starbase.cursor = index;
        self.starbase
            .sync_scroll(rows.len(), self.starbase_visible_rows());
        Ok(rows[index].clone())
    }

    fn selected_starbase_move_row(&self) -> Option<StarbaseRow> {
        let record_index = self.starbase.move_prompt_base_record_index_1_based?;
        self.starbase_rows()
            .into_iter()
            .find(|row| row.base_record_index_1_based == record_index)
    }

    fn finalize_starbase_destination(
        &mut self,
        row: StarbaseRow,
        destination: [u8; 2],
    ) -> Result<(), String> {
        let player_record = self.player.record_index_1_based;
        let base_record = row.base_record_index_1_based;
        let report_text = self.starbase_move_report_text(&row, destination);
        self.change(|state| {
            if destination == row.coords {
                state.game_data.halt_starbase(player_record, base_record)?;
            } else {
                state.game_data.set_starbase_destination(
                    player_record,
                    base_record,
                    destination,
                )?;
            }
            crate::app::persistence::append_report_block(
                &mut state.report_block_rows,
                player_record as u8,
                report_text,
            );
            Ok(())
        })
        .map_err(|err| err.to_string())?;
        self.clear_starbase_move_prompt();
        let notice = if destination == row.coords {
            format!(
                "Starbase #{} halted at {}.",
                row.base_id,
                format_sector_coords_table(destination)
            )
        } else {
            format!(
                "Starbase #{} ordered to move to {}.",
                row.base_id,
                format_sector_coords_table(destination)
            )
        };
        self.show_command_menu_notice(CommandMenu::Starbase, notice);
        Ok(())
    }

    fn starbase_move_report_text(&self, row: &StarbaseRow, destination: [u8; 2]) -> String {
        if destination == row.coords {
            return format!(
                "Starbase {} halted at {}.",
                row.base_id,
                format_sector_coords_table(destination)
            );
        }
        let mut text = format!(
            "Starbase {} is moving to {}.",
            row.base_id,
            format_sector_coords_table(destination)
        );
        let guard_fleets = guard_fleet_numbers_for_starbase(
            &self.game_data,
            self.player.record_index_1_based,
            row.base_id,
        );
        if let Some(clause) = format_guard_fleet_clause(&guard_fleets) {
            text.push(' ');
            text.push_str(&clause);
        }
        text
    }
}
