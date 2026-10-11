use crossterm::event::{KeyCode, KeyEvent};

use crate::app::Action;
use crate::domains::empire::EmpireAction;
use crate::domains::messaging::MessagingAction;
use crate::domains::planet::PlanetAction;
use crate::domains::starmap::StarmapAction;
use crate::domains::startup::StartupAction;
use crate::screen::layout::{
    CMD_COL_1, CMD_COL_2, EXPERT_MENU_PROMPT_ROW, MenuEntry, PRIMARY_MENU_ROW,
    draw_command_center_padded, draw_expert_menu_padded,
    draw_inline_delete_reviewables_prompt_padded, draw_inline_planet_info_prompt_padded,
    draw_menu_entry_with_toggle, draw_menu_notice_padded, menu_prompt_row, new_playfield,
};
use crate::screen::{CommandMenu, PlayfieldBuffer, Screen, ScreenFrame};

pub struct GeneralMenuScreen;

const TOP_ROW: [MenuEntry<'static>; 2] = [
    MenuEntry::new(CMD_COL_2, "I", "nfo about a Planet"),
    MenuEntry::new(51, "C", "ommunicate (send message)"),
];

const ROW_1: [MenuEntry<'static>; 2] = [
    MenuEntry::new(CMD_COL_1, "H", "elp with commands"),
    MenuEntry::new(51, "R", "eview Inbox"),
];

const ROW_2: [MenuEntry<'static>; 3] = [
    MenuEntry::new(CMD_COL_1, "Q", "uit to main menu"),
    MenuEntry::new(CMD_COL_2, "S", "tatus, your"),
    MenuEntry::new(51, "D", "elete ALL reports"),
];

const ROW_3: [MenuEntry<'static>; 3] = [
    MenuEntry::new(CMD_COL_1, "X", "pert mode ON/OFF"),
    MenuEntry::new(CMD_COL_2, "P", "rofile of your empire"),
    MenuEntry::new(51, "O", "ther empires (rankings)"),
];

const ROW_4: [MenuEntry<'static>; 3] = [
    MenuEntry::new(CMD_COL_1, "V", "iew Partial Starmap"),
    MenuEntry::new(CMD_COL_2, "M", "ap of the galaxy"),
    MenuEntry::new(51, "E", "nemies, declare or list"),
];

impl GeneralMenuScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn render_with_notice(
        &mut self,
        frame: &ScreenFrame<'_>,
        notice: Option<&str>,
        expert_mode: bool,
        inline_delete_reviewables: bool,
        inline_planet_info: bool,
        info_default_coords: [u8; 2],
        info_input: &str,
        info_notice: Option<&str>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        if expert_mode {
            if inline_delete_reviewables {
                draw_inline_delete_reviewables_prompt_padded(
                    &mut buffer,
                    EXPERT_MENU_PROMPT_ROW,
                    notice,
                );
            } else if inline_planet_info {
                draw_inline_planet_info_prompt_padded(
                    &mut buffer,
                    EXPERT_MENU_PROMPT_ROW,
                    info_default_coords,
                    info_input,
                    info_notice,
                    notice,
                );
            } else {
                draw_expert_menu_padded(
                    &mut buffer,
                    "GENERAL COMMAND",
                    "? X V I A S P M C R D O E <Q>",
                    notice,
                );
            }
            return Ok(buffer);
        }
        draw_command_center_padded(
            &mut buffer,
            "GENERAL COMMAND CENTER:",
            &TOP_ROW,
            &[&ROW_1, &ROW_2, &ROW_3, &ROW_4],
            "GENERAL COMMAND",
            "? X V I A S P M C R D O E <Q>",
        );
        let autopilot_on =
            u8::from(frame.game_data.players[frame.player.record_index_1_based - 1].autopilot) != 0;
        draw_menu_entry_with_toggle(
            &mut buffer,
            PRIMARY_MENU_ROW + 1,
            CMD_COL_2,
            "A",
            "utopilot ",
            autopilot_on,
        );
        let command_row = menu_prompt_row(PRIMARY_MENU_ROW + 4);
        if inline_delete_reviewables {
            draw_inline_delete_reviewables_prompt_padded(&mut buffer, command_row, notice);
        } else if inline_planet_info {
            draw_inline_planet_info_prompt_padded(
                &mut buffer,
                command_row,
                info_default_coords,
                info_input,
                info_notice,
                notice,
            );
        } else if let Some(notice) = notice {
            draw_menu_notice_padded(&mut buffer, command_row, notice);
        }
        Ok(buffer)
    }
}

impl Screen for GeneralMenuScreen {
    fn render(
        &mut self,
        frame: &ScreenFrame<'_>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        self.render_with_notice(frame, None, false, false, false, [0, 0], "", None)
    }

    fn handle_key(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Char('h') | KeyCode::Char('H') => Action::OpenPopupHelp,
            KeyCode::Char('i') | KeyCode::Char('I') => {
                Action::Planet(PlanetAction::OpenInfoPrompt(CommandMenu::General))
            }
            KeyCode::Char('a') | KeyCode::Char('A') => Action::ToggleAutopilot,
            KeyCode::Char('c') | KeyCode::Char('C') => {
                Action::Messaging(MessagingAction::OpenComposeRecipient)
            }
            KeyCode::Char('e') | KeyCode::Char('E') => Action::Empire(EmpireAction::OpenEnemies),
            KeyCode::Char('m') | KeyCode::Char('M') => Action::Starmap(StarmapAction::OpenFull),
            KeyCode::Char('v') | KeyCode::Char('V') => {
                Action::Starmap(StarmapAction::OpenPartialView(CommandMenu::General))
            }
            KeyCode::Char('s') | KeyCode::Char('S') => Action::Empire(EmpireAction::OpenStatus),
            KeyCode::Char('p') | KeyCode::Char('P') => Action::Empire(EmpireAction::OpenProfile),
            KeyCode::Char('d') | KeyCode::Char('D') => {
                Action::Messaging(MessagingAction::OpenDeleteReviewables)
            }
            KeyCode::Char('o') | KeyCode::Char('O') => Action::Empire(
                EmpireAction::OpenRankingsTable(nc_data::EmpireProductionRankingSort::Production),
            ),
            KeyCode::Char('r') | KeyCode::Char('R') => Action::Startup(StartupAction::OpenReports),
            KeyCode::Char('x') | KeyCode::Char('X') => Action::ToggleExpertMode,
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => Action::OpenMainMenu,
            _ => Action::Noop,
        }
    }
}
