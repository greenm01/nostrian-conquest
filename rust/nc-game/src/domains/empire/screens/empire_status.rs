use crossterm::event::KeyEvent;

use crate::app::Action;
use crate::screen::layout::{
    aligned_label_width, dismiss_prompt_row, draw_aligned_status_line, draw_dismiss_prompt_padded,
    draw_title_bar_padded, new_playfield,
};
use crate::screen::{CommandMenu, PlayfieldBuffer, Screen, ScreenFrame};
pub struct EmpireStatusScreen;

impl EmpireStatusScreen {
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
        let empire_id = player_idx as u8;
        let label_width = aligned_label_width([
            "Empire",
            "Handle",
            "Year",
            "Campaign state",
            "Tax rate",
            "Owned planets",
            "Owned fleets",
            "Starbases",
            "Autopilot",
        ]);

        let mut buffer = new_playfield();
        draw_title_bar_padded(&mut buffer, 0, "STATUS, YOUR EMPIRE: ");
        draw_aligned_status_line(
            &mut buffer,
            2,
            label_width,
            "Empire",
            display_or_unknown(&frame.player.empire_name),
        );
        draw_aligned_status_line(
            &mut buffer,
            3,
            label_width,
            "Handle",
            display_or_unknown(&frame.player.handle),
        );
        draw_aligned_status_line(
            &mut buffer,
            4,
            label_width,
            "Year",
            &frame.game_data.conquest.game_year.to_string(),
        );
        draw_aligned_status_line(
            &mut buffer,
            5,
            label_width,
            "Campaign state",
            campaign_state_label(frame.game_data.empire_campaign_state(empire_id)),
        );
        draw_aligned_status_line(
            &mut buffer,
            7,
            label_width,
            "Tax rate",
            &format!("{}%", player.tax_rate),
        );
        draw_aligned_status_line(
            &mut buffer,
            8,
            label_width,
            "Owned planets",
            &frame
                .game_data
                .player_owned_planet_count_current_known(player_idx)
                .to_string(),
        );
        draw_aligned_status_line(
            &mut buffer,
            9,
            label_width,
            "Owned fleets",
            &frame
                .game_data
                .player_owned_fleet_count_current_known(player_idx)
                .to_string(),
        );
        draw_aligned_status_line(
            &mut buffer,
            10,
            label_width,
            "Starbases",
            &frame
                .game_data
                .player_owned_base_record_count_current_known(player_idx)
                .to_string(),
        );
        draw_aligned_status_line(
            &mut buffer,
            11,
            label_width,
            "Autopilot",
            if u8::from(player.autopilot) != 0 {
                "ON"
            } else {
                "OFF"
            },
        );
        let _ = menu;
        draw_dismiss_prompt_padded(&mut buffer, dismiss_prompt_row(11));
        Ok(buffer)
    }
}

impl Screen for EmpireStatusScreen {
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

fn campaign_state_label(state: Option<nc_data::CampaignState>) -> &'static str {
    match state {
        Some(nc_data::CampaignState::Stable) => "Stable",
        Some(nc_data::CampaignState::MarginalExistence) => "Marginal Existence",
        Some(nc_data::CampaignState::DefectionRisk) => "Defection Risk",
        Some(nc_data::CampaignState::Defeated) => "Defeated",
        Some(nc_data::CampaignState::CivilDisorder) => "In Civil Disorder",
        Some(nc_data::CampaignState::Rogue) => "Rogue",
        None => "Unknown",
    }
}

fn display_or_unknown(value: &str) -> &str {
    if value.is_empty() { "<unknown>" } else { value }
}
