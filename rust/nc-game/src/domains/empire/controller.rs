use crate::app::helpers::sync_scroll_to_cursor;
use crate::app::state::App;
use crate::screen::ScreenId;

impl App {
    fn enemies_visible_rows(&self) -> usize {
        crate::domains::empire::screens::enemies::enemies_visible_rows(self.screen_geometry)
    }

    pub fn open_enemies(&mut self) {
        self.empire.enemies_input.clear();
        self.empire.enemies_status = None;
        self.empire.enemies_scroll_offset = 0;
        self.empire.enemies_cursor = 0;
        self.current_screen = ScreenId::Enemies;
    }

    pub fn open_empire_status(&mut self) {
        self.command_return_menu = self.origin_command_menu();
        self.current_screen = ScreenId::EmpireStatus;
    }

    pub fn open_empire_profile(&mut self) {
        self.command_return_menu = self.origin_command_menu();
        self.current_screen = ScreenId::EmpireProfile;
    }

    pub fn open_rankings_table(&mut self, sort: nc_data::EmpireProductionRankingSort) {
        self.command_return_menu = self.origin_command_menu();
        self.current_screen = ScreenId::Rankings(sort);
    }

    pub fn open_reports(&mut self) {
        if self.current_screen == ScreenId::GeneralMenu
            && self
                .inbox_items_for_filters(
                    crate::domains::messaging::state::InboxTypeFilter::All,
                    None,
                )
                .is_empty()
        {
            self.show_command_menu_notice(self.origin_command_menu(), "Inbox is empty.");
            return;
        }
        self.open_reports_inbox();
    }

    pub fn scroll_enemies(&mut self, delta: i8) {
        if self.current_screen != ScreenId::Enemies {
            return;
        }
        let total = self.game_data.players.len().saturating_sub(1);
        let max_offset = total.saturating_sub(self.enemies_visible_rows());
        self.empire.enemies_scroll_offset = self
            .empire
            .enemies_scroll_offset
            .saturating_add_signed(delta as isize)
            .min(max_offset);
    }

    pub fn move_enemies_cursor(&mut self, delta: i8) {
        if self.current_screen != ScreenId::Enemies {
            return;
        }
        // Total rows = all empires minus self.
        let total = self.game_data.players.len().saturating_sub(1);
        if total == 0 {
            return;
        }
        let next = self.empire.enemies_cursor as isize + delta as isize;
        self.empire.enemies_cursor = next.rem_euclid(total as isize) as usize;
        let visible_rows = self.enemies_visible_rows();
        sync_scroll_to_cursor(
            &mut self.empire.enemies_scroll_offset,
            self.empire.enemies_cursor,
            visible_rows,
        );
    }

    pub fn toggle_autopilot(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let player_record = self.player.record_index_1_based;
        self.change(|state| {
            let player = &mut state.game_data.players[player_record - 1];
            player.autopilot = !player.autopilot;
            nc_data::clear_inactivity_autopilot_pending(
                player_record,
                &mut state.player_activity_states,
            );
            Ok(())
        })
    }

    pub fn append_enemies_char(&mut self, ch: char) {
        if self.current_screen == ScreenId::Enemies && self.empire.enemies_input.len() < 2 {
            self.empire.enemies_input.push(ch);
            self.sync_enemies_cursor_to_input();
            self.empire.enemies_status = None;
        }
    }

    pub fn backspace_enemies_input(&mut self) {
        if self.current_screen == ScreenId::Enemies {
            self.empire.enemies_input.pop();
            self.sync_enemies_cursor_to_input();
            self.empire.enemies_status = None;
        }
    }

    pub fn submit_enemies_input(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // If the input box is empty, derive the empire id from the cursor row.
        let empire_id = if self.empire.enemies_input.trim().is_empty() {
            let mut ids: Vec<u8> = self
                .game_data
                .players
                .iter()
                .enumerate()
                .filter(|(idx, _)| *idx + 1 != self.player.record_index_1_based)
                .map(|(idx, _)| (idx + 1) as u8)
                .collect();
            ids.sort_unstable();
            match ids.get(self.empire.enemies_cursor) {
                Some(&id) => id,
                None => {
                    self.empire.enemies_status = Some("No empire selected.".to_string());
                    return Ok(());
                }
            }
        } else {
            match self.empire.enemies_input.parse::<u8>() {
                Ok(id) => id,
                Err(_) => {
                    self.empire.enemies_status = Some("Enter an empire number.".to_string());
                    return Ok(());
                }
            }
        };
        let max_empire = self.game_data.conquest.player_count;
        if !(1..=max_empire).contains(&empire_id) {
            self.empire.enemies_status =
                Some(format!("Enter an empire number in 1..={max_empire}."));
            return Ok(());
        }
        if empire_id as usize == self.player.record_index_1_based {
            self.empire.enemies_status = Some("You cannot target your own empire.".to_string());
            return Ok(());
        }
        let current = self
            .game_data
            .stored_diplomatic_relation(self.player.record_index_1_based as u8, empire_id)
            .unwrap_or(nc_data::DiplomaticRelation::Neutral);
        let next = match current {
            nc_data::DiplomaticRelation::Neutral | nc_data::DiplomaticRelation::Unknown(_) => {
                nc_data::DiplomaticRelation::Enemy
            }
            nc_data::DiplomaticRelation::Enemy => nc_data::DiplomaticRelation::Neutral,
        };
        let player_record = self.player.record_index_1_based as u8;
        self.change(|state| {
            state
                .game_data
                .set_stored_diplomatic_relation(player_record, empire_id, next)?;
            Ok(())
        })?;
        self.empire.enemies_status = None;
        self.empire.enemies_input.clear();
        Ok(())
    }

    pub fn current_autopilot_flag(&self) -> u8 {
        u8::from(self.game_data.players[self.player.record_index_1_based - 1].autopilot)
    }

    pub fn current_relation_to(&self, empire_id: u8) -> Option<nc_data::DiplomaticRelation> {
        self.game_data
            .stored_diplomatic_relation(self.player.record_index_1_based as u8, empire_id)
    }

    pub fn enemies_scroll_offset(&self) -> usize {
        self.empire.enemies_scroll_offset
    }

    fn sync_enemies_cursor_to_input(&mut self) {
        let ids = self
            .game_data
            .players
            .iter()
            .enumerate()
            .filter(|(idx, _)| *idx + 1 != self.player.record_index_1_based)
            .map(|(idx, _)| (idx + 1) as u8)
            .collect::<Vec<_>>();
        let rows = ids
            .iter()
            .map(|id| vec![format!("{id:02}")])
            .collect::<Vec<_>>();
        let Some(index) = crate::screen::table_selection::find_typed_jump_index(
            &rows,
            0,
            &self.empire.enemies_input,
        ) else {
            return;
        };
        self.empire.enemies_cursor = index;
        let visible_rows = self.enemies_visible_rows();
        sync_scroll_to_cursor(
            &mut self.empire.enemies_scroll_offset,
            self.empire.enemies_cursor,
            visible_rows,
        );
    }
}
