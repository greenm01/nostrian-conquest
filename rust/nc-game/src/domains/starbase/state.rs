use crate::domains::starbase::screens::starbase::StarbaseRow;
use nc_data::CoreGameData;
use nc_engine::{
    format_starbase_list_guard_label, format_starbase_review_guard_label,
    guard_fleet_numbers_for_starbase, starbase_eta_label, starbase_operation_label,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StarbaseMovePromptMode {
    Base,
    Decision,
    Destination,
    HaltConfirm,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StarbaseState {
    pub scroll_offset: usize,
    pub cursor: usize,
    pub review_index: usize,
    pub review_input: String,
    pub review_status: Option<String>,
    pub move_prompt_mode: Option<StarbaseMovePromptMode>,
    pub move_prompt_input: String,
    pub move_prompt_status: Option<String>,
    pub move_prompt_default_value: String,
    pub move_prompt_base_record_index_1_based: Option<usize>,
}

impl StarbaseState {
    pub fn starbase_rows(
        &self,
        game_data: &CoreGameData,
        player_record_index_1_based: usize,
    ) -> Vec<StarbaseRow> {
        let mut rows = game_data
            .bases
            .records
            .iter()
            .enumerate()
            .filter(|(_, base)| base.owner_empire_raw() as usize == player_record_index_1_based)
            .map(|(idx, base)| {
                let guard_fleet_numbers = guard_fleet_numbers_for_starbase(
                    game_data,
                    player_record_index_1_based,
                    base.base_id_raw(),
                );
                let destination_coords = base.target_coords_raw();
                let operation_label =
                    starbase_operation_label(base.coords_raw(), destination_coords);
                let eta_label = starbase_eta_label(base.coords_raw(), destination_coords);
                StarbaseRow {
                    base_record_index_1_based: idx + 1,
                    base_id: base.base_id_raw(),
                    escort_list_label: format_starbase_list_guard_label(&guard_fleet_numbers),
                    escort_review_label: format_starbase_review_guard_label(&guard_fleet_numbers),
                    coords: base.coords_raw(),
                    destination_coords,
                    eta_label,
                    operation_label,
                }
            })
            .collect::<Vec<_>>();
        rows.sort_by_key(|row| row.base_id);
        rows
    }

    pub fn move_select(
        &mut self,
        delta: i8,
        game_data: &CoreGameData,
        player_idx: usize,
        visible_rows: usize,
    ) {
        let total = self.starbase_rows(game_data, player_idx).len();
        if total == 0 {
            return;
        }
        self.cursor = crate::app::helpers::move_wrapped_cursor(self.cursor, delta as isize, total);
        self.sync_scroll(total, visible_rows);
    }

    pub fn append_char(
        &mut self,
        ch: char,
        game_data: &CoreGameData,
        player_idx: usize,
        visible_rows: usize,
    ) {
        self.review_input.push(ch);
        self.sync_cursor_to_input(game_data, player_idx, visible_rows);
    }

    pub fn backspace_input(
        &mut self,
        game_data: &CoreGameData,
        player_idx: usize,
        visible_rows: usize,
    ) {
        self.review_input.pop();
        self.sync_cursor_to_input(game_data, player_idx, visible_rows);
    }

    fn sync_cursor_to_input(
        &mut self,
        game_data: &CoreGameData,
        player_idx: usize,
        visible_rows: usize,
    ) {
        let rows = self.starbase_rows(game_data, player_idx);
        let match_rows = rows
            .iter()
            .map(|row| vec![row.base_id.to_string()])
            .collect::<Vec<_>>();
        let Some(index) = crate::screen::table_selection::find_typed_jump_index(
            &match_rows,
            0,
            &self.review_input,
        ) else {
            return;
        };
        self.cursor = index;
        self.sync_scroll(rows.len(), visible_rows);
    }

    pub fn sync_scroll(&mut self, total: usize, visible_rows: usize) {
        if total <= visible_rows {
            self.scroll_offset = 0;
            return;
        }
        let half = visible_rows / 2;
        let max_offset = total - visible_rows;
        self.scroll_offset = self.cursor.saturating_sub(half).min(max_offset);
    }
}
