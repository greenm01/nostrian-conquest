use crate::app::state::App;
use crate::domains::startup::state::FirstTimeOnboardingMode;
use crate::screen::layout::{PLAYFIELD_HEIGHT, draw_centered_text, new_playfield};
use crate::screen::{
    CellStyle, GameColor, PlayfieldBuffer, ScreenFrame, ScreenId,
    render_first_time_homeworld_confirm, render_first_time_homeworld_name,
    render_first_time_join_name, render_first_time_join_name_confirm,
    render_first_time_join_no_pending, render_first_time_join_summary,
};

pub fn render(app: &mut App) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
    let frame = ScreenFrame {
        game_dir: &app.game_dir,
        game_data: &app.game_data,
        player: &app.player,
        campaign_seed: app.campaign_seed,
        player_activity_states: &app.player_activity_states,
        player_lifecycle_states: &app.player_lifecycle_states,
        winner_state: app.winner_state,
        planet_intel_snapshots: &app.planet_intel_snapshots,
        owned_planet_years: &app.owned_planet_years,
        geometry: app.screen_geometry,
    };
    match app.current_screen {
        ScreenId::TerminalNotice => Ok(render_terminal_notice(&app.terminal_notice_lines)),
        ScreenId::Startup(phase) => app.startup.render_phase(
            &frame,
            phase,
            app.startup_state.startup_status.as_deref(),
            app.startup_state.splash_page,
            app.startup_state.intro_page,
            app.startup_state.results_block,
            app.startup_state.results_page,
            app.startup_state.results_mode,
            app.startup_state.results_nonstop,
            &app.startup_state.results_review_history_rows,
            app.startup_state.messages_block,
            app.startup_state.messages_page,
            app.startup_state.messages_mode,
            app.startup_state.messages_nonstop,
            &app.startup_state.messages_review_history_rows,
            app.startup_state.results_deleted_any,
            app.startup_state.messages_deleted_any,
            app.game_data.conquest.game_year,
        ),
        ScreenId::FirstTimeMenu => app.first_time_menu.render(
            app.startup_state.first_time_status.as_deref(),
            app.door_mode,
        ),
        ScreenId::FirstTimeEmpires => app
            .first_time_empires
            .render_rows(frame.geometry, &app.first_time_empire_rows()),
        ScreenId::FirstTimeIntro => app
            .first_time_intro
            .render_page(frame.geometry, app.startup_state.first_time_intro_page),
        ScreenId::FirstTimeReservedPrompt => crate::screen::render_first_time_reserved_prompt(
            app.startup_state.reserved_seat_alias.as_deref(),
        ),
        ScreenId::ThemePicker => app.theme_picker.render(
            frame.geometry,
            &app.startup_state.theme_picker_rows,
            app.startup_state.theme_picker_scroll_offset,
            app.startup_state.theme_picker_cursor,
            crate::theme::current_theme_key().as_deref(),
            &app.startup_state.theme_picker_input,
            app.startup_state.theme_picker_status.as_deref(),
        ),
        ScreenId::FirstTimePreloadedRenamePrompt => {
            crate::screen::render_preloaded_first_login_rename_prompt(&app.player.empire_name)
        }
        ScreenId::FirstTimeJoinEmpireName => render_first_time_join_name(
            app.startup_state.first_time_rename_preloaded_empire,
            app.startup_state.first_time_onboarding_mode == FirstTimeOnboardingMode::BbsReserved,
            app.startup_state.fixed_player_launch,
            app.startup_state.reserved_seat_alias.as_deref(),
            &app.player.empire_name,
            &app.startup_state.first_time_input,
            app.startup_state.first_time_status.as_deref(),
            app.door_mode,
        ),
        ScreenId::FirstTimeJoinEmpireConfirm => render_first_time_join_name_confirm(
            app.startup_state.first_time_rename_preloaded_empire,
            app.startup_state.first_time_onboarding_mode == FirstTimeOnboardingMode::BbsReserved,
            app.startup_state.fixed_player_launch,
            &app.startup_state.first_time_empire_name,
            app.door_mode,
        ),
        ScreenId::FirstTimeJoinSummary => render_first_time_join_summary(
            &app.startup_state.first_time_empire_name,
            app.player.record_index_1_based,
            app.game_data.conquest.game_year,
        ),
        ScreenId::FirstTimeJoinNoPending => render_first_time_join_no_pending(),
        ScreenId::FirstTimeHomeworldName => {
            let (coords, present, potential) = app.first_time_homeworld_summary()?;
            render_first_time_homeworld_name(
                coords,
                present,
                potential,
                app.player.classic_login_state
                    == crate::model::ClassicLoginState::MatchedPreloadedFirstLogin,
                &app.startup_state.first_time_input,
                app.startup_state.first_time_status.as_deref(),
            )
        }
        ScreenId::FirstTimeHomeworldConfirm => {
            let (coords, present, potential) = app.first_time_homeworld_summary()?;
            render_first_time_homeworld_confirm(
                coords,
                present,
                potential,
                app.player.classic_login_state
                    == crate::model::ClassicLoginState::MatchedPreloadedFirstLogin,
                &app.startup_state.first_time_homeworld_name,
            )
        }
        ScreenId::ColonyWorldName => {
            let (coords, present, potential) = app.colony_world_summary()?;
            crate::screen::render_colony_world_name(
                coords,
                present,
                potential,
                &app.startup_state.first_time_input,
                app.startup_state.first_time_status.as_deref(),
            )
        }
        ScreenId::ColonyWorldConfirm => {
            let (coords, _, _) = app.colony_world_summary()?;
            crate::screen::render_colony_world_confirm(coords, &app.startup_state.colony_world_name)
        }
        ScreenId::MainMenu => app.main_menu.render_with_notice(
            app.command_menu_notice.as_deref(),
            app.door_mode,
            app.expert_mode,
            app.planet.info_prompt_active
                && app.command_return_menu == crate::screen::CommandMenu::Main,
            app.default_planet_prompt_coords(),
            &app.planet.info_input,
            app.planet.info_error.as_deref(),
        ),
        ScreenId::GeneralMenu => app.general_menu.render_with_notice(
            &frame,
            app.command_menu_notice.as_deref(),
            app.expert_mode,
            app.messaging.delete_reviewables_prompt_active,
            app.planet.info_prompt_active
                && app.command_return_menu == crate::screen::CommandMenu::General,
            app.default_planet_prompt_coords(),
            &app.planet.info_input,
            app.planet.info_error.as_deref(),
        ),
        ScreenId::Reports => app.reports.render_inbox(
            frame.geometry,
            app.command_return_menu,
            &app.filtered_inbox_display_items(),
            app.messaging.inbox_type_filter,
            app.messaging.inbox_year_filter,
            app.messaging.inbox_cursor,
            app.messaging.inbox_scroll_offset,
            app.messaging.inbox_preview_scroll,
            app.messaging.inbox_focus,
            &app.messaging.inbox_id_input,
            &app.messaging.inbox_year_input,
            app.messaging.inbox_prompt_mode,
            app.messaging.inbox_feedback.as_ref(),
            app.game_data.conquest.game_year,
        ),
        _ => unreachable!("startup views called for non-startup screen"),
    }
}

fn render_terminal_notice(lines: &[String]) -> PlayfieldBuffer {
    let mut buffer = new_playfield();
    let style = CellStyle::new(GameColor::BrightWhite, GameColor::Black, false);
    let start_row = PLAYFIELD_HEIGHT.saturating_sub(lines.len()) / 2;
    for (offset, line) in lines.iter().enumerate() {
        draw_centered_text(&mut buffer, start_row + offset, line, style);
    }
    buffer
}
