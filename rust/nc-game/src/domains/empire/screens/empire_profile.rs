use crossterm::event::KeyEvent;

use crate::app::Action;
use crate::screen::layout::{
    DetailField, LEFT_WINDOW_PAD_COL, aligned_label_width, dismiss_prompt_row,
    draw_aligned_detail_line_at, draw_aligned_detail_pair_at, draw_centered_text,
    draw_dismiss_prompt_padded, draw_title_bar_padded, new_playfield,
};
use crate::screen::{CommandMenu, PlayfieldBuffer, Screen, ScreenFrame};
use crate::theme::classic;

pub struct EmpireProfileScreen;

const EMPIRE_PROFILE_LEFT_COL: usize = LEFT_WINDOW_PAD_COL;
const EMPIRE_PROFILE_UNITS_RIGHT_COL: usize = 38 + LEFT_WINDOW_PAD_COL;
const EMPIRE_PROFILE_STATS_RIGHT_COL: usize = 40 + LEFT_WINDOW_PAD_COL;

impl EmpireProfileScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn render_with_menu(
        &mut self,
        frame: &ScreenFrame<'_>,
        menu: CommandMenu,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let player_idx = frame.player.record_index_1_based;
        let player = &frame.game_data.players[player_idx - 1];
        let economy = frame.game_data.empire_economy_summary(player_idx);
        let active = frame.game_data.empire_active_duty_summary(player_idx);
        let stardock = frame.game_data.empire_stardock_summary(player_idx);
        let left_stat_width = aligned_label_width([
            "Number of Planets",
            "Production",
            "Potential Production",
            "Empire Revenue",
            "Efficiency of Empire",
        ]);
        let right_stat_width = aligned_label_width([
            "Rank by Number of Planets",
            "Rank by Production",
            "Tax Rate of Empire",
            "Maximum # of Fleets & Bases",
            "Current # of Fleets & Bases",
        ]);
        let unit_label_width = aligned_label_width([
            "Destroyers",
            "Cruisers",
            "Battleships",
            "Scouts",
            "Transports",
            "ETACs",
            "StarBases",
            "Armies",
            "Ground Batteries",
        ]);

        let mut buffer = new_playfield();
        draw_title_bar_padded(
            &mut buffer,
            0,
            &format!(
                "Empire Name: {}",
                display_or_unknown(&frame.player.empire_name)
            ),
        );

        write_stat_pair(
            &mut buffer,
            2,
            left_stat_width,
            "Number of Planets",
            &economy.owned_planets.to_string(),
            right_stat_width,
            "Rank by Number of Planets",
            &ordinal_rank(economy.rank_by_planets),
        );
        write_stat_pair(
            &mut buffer,
            3,
            left_stat_width,
            "Production",
            &economy.present_production.to_string(),
            right_stat_width,
            "Rank by Production",
            &ordinal_rank(economy.rank_by_present_production),
        );
        write_stat_pair(
            &mut buffer,
            4,
            left_stat_width,
            "Potential Production",
            &economy.potential_production.to_string(),
            right_stat_width,
            "Tax Rate of Empire",
            &format!("{:.1}%", economy.tax_rate as f64),
        );
        write_stat_pair(
            &mut buffer,
            5,
            left_stat_width,
            "Empire Revenue",
            &economy.total_available_points.to_string(),
            right_stat_width,
            "Maximum # of Fleets & Bases",
            &economy.max_fleets_and_bases.to_string(),
        );
        write_stat_pair(
            &mut buffer,
            6,
            left_stat_width,
            "Efficiency of Empire",
            &format!("{:.3}%", economy.efficiency_percent),
            right_stat_width,
            "Current # of Fleets & Bases",
            &economy.current_fleets_and_bases.to_string(),
        );

        buffer.write_text(
            8,
            LEFT_WINDOW_PAD_COL,
            "SHIPS & OTHER UNITS ON ACTIVE DUTY",
            classic::menu_hotkey_style(),
        );
        buffer.write_text(
            8,
            EMPIRE_PROFILE_UNITS_RIGHT_COL,
            "SHIPS & OTHER UNITS IN STARDOCK",
            classic::menu_hotkey_style(),
        );

        write_unit_line(
            &mut buffer,
            9,
            unit_label_width,
            "Destroyers",
            active.destroyers,
            stardock.destroyers,
        );
        write_unit_line(
            &mut buffer,
            10,
            unit_label_width,
            "Cruisers",
            active.cruisers,
            stardock.cruisers,
        );
        write_unit_line(
            &mut buffer,
            11,
            unit_label_width,
            "Battleships",
            active.battleships,
            stardock.battleships,
        );
        write_unit_line(
            &mut buffer,
            12,
            unit_label_width,
            "Scouts",
            active.scouts,
            stardock.scouts,
        );
        write_unit_line(
            &mut buffer,
            13,
            unit_label_width,
            "Transports",
            active.transports,
            stardock.transports,
        );
        write_unit_line(
            &mut buffer,
            14,
            unit_label_width,
            "ETACs",
            active.etacs,
            stardock.etacs,
        );
        write_unit_line(
            &mut buffer,
            15,
            unit_label_width,
            "StarBases",
            active.starbases,
            stardock.starbases,
        );
        draw_aligned_detail_line_at(
            &mut buffer,
            16,
            EMPIRE_PROFILE_LEFT_COL,
            unit_label_width,
            "Armies",
            " : ",
            &active.armies.to_string(),
        );
        draw_aligned_detail_line_at(
            &mut buffer,
            17,
            EMPIRE_PROFILE_LEFT_COL,
            unit_label_width,
            "Ground Batteries",
            " : ",
            &active.ground_batteries.to_string(),
        );

        draw_centered_text(
            &mut buffer,
            18,
            &format!(
                "Autopilot is {}.",
                if u8::from(player.autopilot) != 0 {
                    "ON"
                } else {
                    "OFF"
                }
            ),
            classic::status_value_style(),
        );
        let _ = menu;
        draw_dismiss_prompt_padded(&mut buffer, dismiss_prompt_row(18));
        Ok(buffer)
    }
}

impl Screen for EmpireProfileScreen {
    fn render(
        &mut self,
        frame: &ScreenFrame<'_>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        self.render_with_menu(frame, CommandMenu::General)
    }

    fn handle_key(&self, _key: KeyEvent) -> Action {
        Action::ReturnToCommandMenu
    }
}

fn write_stat_pair(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    left_label_width: usize,
    left_label: &str,
    left_value: &str,
    right_label_width: usize,
    right_label: &str,
    right_value: &str,
) {
    draw_aligned_detail_pair_at(
        buffer,
        row,
        EMPIRE_PROFILE_LEFT_COL,
        DetailField {
            label_width: left_label_width,
            label: left_label,
            separator: " : ",
            value: left_value,
        },
        EMPIRE_PROFILE_STATS_RIGHT_COL,
        DetailField {
            label_width: right_label_width,
            label: right_label,
            separator: " : ",
            value: right_value,
        },
    );
}

fn write_unit_line(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    label_width: usize,
    label: &str,
    active_value: u32,
    stardock_value: u32,
) {
    let active_text = active_value.to_string();
    let stardock_text = stardock_value.to_string();
    draw_aligned_detail_pair_at(
        buffer,
        row,
        EMPIRE_PROFILE_LEFT_COL,
        DetailField {
            label_width,
            label,
            separator: " : ",
            value: &active_text,
        },
        EMPIRE_PROFILE_UNITS_RIGHT_COL,
        DetailField {
            label_width,
            label,
            separator: " : ",
            value: &stardock_text,
        },
    );
}

fn ordinal_rank(rank: usize) -> String {
    let suffix = match rank % 100 {
        11..=13 => "th",
        _ => match rank % 10 {
            1 => "st",
            2 => "nd",
            3 => "rd",
            _ => "th",
        },
    };
    format!("{rank}{suffix}")
}

fn display_or_unknown(value: &str) -> &str {
    if value.is_empty() { "<unnamed>" } else { value }
}
