use crate::app::helpers::{is_coordinate_input_char, resolve_default_coords_input};
use crate::app::state::App;
use crate::domains::planet::PlanetAction;
use crate::domains::planet::screens::planet_list::{
    PLANET_LIST_SCORCH_CONFIRM_PROMPT, PLANET_LIST_SCORCH_LAST_CONFIRM_PROMPT,
    PLANET_LIST_SCORCH_REALLY_CONFIRM_PROMPT, PLANET_LIST_SCORCH_RESCIND_PROMPT,
};
use crate::domains::planet::state::PlanetScorchPromptMode;
use crate::screen::{CommandMenu, ScreenId, format_sector_coords_default};
use crossterm::event::KeyCode;

impl App {
    pub fn open_planet_scorch_prompt(&mut self) {
        if matches!(
            self.current_screen,
            ScreenId::PlanetList(crate::screen::PlanetListMode::Brief, _)
        ) {
            self.planet.command_context = crate::domains::planet::state::PlanetCommandContext::List;
            let Ok(row) = self.current_planet_list_row() else {
                self.show_planet_context_notice("No owned planets available.");
                return;
            };
            self.clear_command_menu_notice();
            self.clear_planet_list_status();
            self.close_planet_auto_commission_prompt();
            self.close_planet_tax_prompt();
            self.close_planet_info_prompt();
            self.clear_planet_transport_prompt();
            self.open_planet_scorch_prompt_mode(
                PlanetScorchPromptMode::Confirm1,
                None,
                Some(row.planet_record_index_1_based),
            );
            self.current_screen = self.planet_context_screen();
            return;
        }
        let Some(default_coords) = self.default_planet_scorch_coords() else {
            self.show_command_menu_notice(CommandMenu::Planet, "No owned planets available.");
            return;
        };
        self.clear_command_menu_notice();
        self.close_planet_auto_commission_prompt();
        self.close_planet_tax_prompt();
        self.close_planet_info_prompt();
        self.clear_planet_transport_prompt();
        self.open_planet_scorch_prompt_mode(
            PlanetScorchPromptMode::Planet,
            Some(default_coords),
            None,
        );
        self.current_screen = ScreenId::PlanetMenu;
    }

    pub(crate) fn inline_planet_scorch_prompt_active_on_current_screen(&self) -> bool {
        matches!(
            self.current_screen,
            ScreenId::PlanetMenu | ScreenId::PlanetList(crate::screen::PlanetListMode::Brief, _)
        ) && self.planet.scorch_prompt_mode.is_some()
    }

    pub(crate) fn handle_planet_scorch_prompt_key(
        &self,
        key: crossterm::event::KeyEvent,
    ) -> crate::app::Action {
        let mode = self.planet.scorch_prompt_mode;
        match key.code {
            KeyCode::Enter
                if matches!(
                    mode,
                    Some(PlanetScorchPromptMode::Confirm1)
                        | Some(PlanetScorchPromptMode::Confirm2)
                        | Some(PlanetScorchPromptMode::Confirm3)
                ) =>
            {
                crate::app::Action::Planet(PlanetAction::CancelScorchPrompt)
            }
            KeyCode::Enter => crate::app::Action::Planet(PlanetAction::SubmitScorchPrompt),
            KeyCode::Backspace => {
                crate::app::Action::Planet(PlanetAction::BackspaceScorchPromptInput)
            }
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                crate::app::Action::Planet(PlanetAction::CancelScorchPrompt)
            }
            KeyCode::Char('n') | KeyCode::Char('N')
                if matches!(
                    mode,
                    Some(PlanetScorchPromptMode::Confirm1)
                        | Some(PlanetScorchPromptMode::Confirm2)
                        | Some(PlanetScorchPromptMode::Confirm3)
                ) =>
            {
                crate::app::Action::Planet(PlanetAction::CancelScorchPrompt)
            }
            KeyCode::Char('y') | KeyCode::Char('Y')
                if matches!(
                    mode,
                    Some(PlanetScorchPromptMode::Confirm1)
                        | Some(PlanetScorchPromptMode::Confirm2)
                        | Some(PlanetScorchPromptMode::Confirm3)
                ) =>
            {
                crate::app::Action::Planet(PlanetAction::SubmitScorchPrompt)
            }
            KeyCode::Char(ch)
                if match mode {
                    Some(PlanetScorchPromptMode::Planet) => is_coordinate_input_char(ch),
                    Some(PlanetScorchPromptMode::Confirm1)
                    | Some(PlanetScorchPromptMode::Confirm2)
                    | Some(PlanetScorchPromptMode::Confirm3)
                    | None => false,
                } =>
            {
                crate::app::Action::Planet(PlanetAction::AppendScorchPromptChar(ch))
            }
            _ => crate::app::Action::Noop,
        }
    }

    pub(crate) fn append_planet_scorch_prompt_char(&mut self, ch: char) {
        if !self.inline_planet_scorch_prompt_active_on_current_screen() {
            return;
        }
        if self.planet.scorch_prompt_mode != Some(PlanetScorchPromptMode::Planet) {
            return;
        }
        if self.planet.scorch_prompt_input.len() >= 16 {
            return;
        }
        self.planet.scorch_prompt_input.push(ch);
        self.planet.scorch_prompt_status = None;
    }

    pub(crate) fn backspace_planet_scorch_prompt_input(&mut self) {
        if !self.inline_planet_scorch_prompt_active_on_current_screen() {
            return;
        }
        self.planet.scorch_prompt_input.pop();
        self.planet.scorch_prompt_status = None;
    }

    pub(crate) fn cancel_planet_scorch_prompt(&mut self) {
        self.clear_planet_scorch_prompt();
        self.current_screen = self.planet_context_screen();
    }

    pub(crate) fn submit_planet_scorch_prompt(&mut self) -> Result<(), String> {
        let Some(mode) = self.planet.scorch_prompt_mode else {
            return Ok(());
        };
        match mode {
            PlanetScorchPromptMode::Planet => {
                let default_coords = self
                    .planet
                    .scorch_prompt_default_coords
                    .ok_or_else(|| "No owned planets available.".to_string())?;
                let Some(coords) =
                    resolve_default_coords_input(&self.planet.scorch_prompt_input, default_coords)
                else {
                    return Err("Enter coordinates like 5,2".to_string());
                };
                let planet_record_index = self.resolve_planet_scorch_record(coords)?;
                self.planet.scorch_selected_planet_record = Some(planet_record_index);
                self.open_planet_scorch_prompt_mode(
                    PlanetScorchPromptMode::Confirm1,
                    None,
                    Some(planet_record_index),
                );
            }
            PlanetScorchPromptMode::Confirm1 if self.selected_planet_scorched() => {
                // One confirmation rescinds the order the same command set.
                return self.finish_planet_scorch(false);
            }
            PlanetScorchPromptMode::Confirm1 => {
                self.open_planet_scorch_prompt_mode(
                    PlanetScorchPromptMode::Confirm2,
                    None,
                    self.planet.scorch_selected_planet_record,
                );
            }
            PlanetScorchPromptMode::Confirm2 => {
                self.open_planet_scorch_prompt_mode(
                    PlanetScorchPromptMode::Confirm3,
                    None,
                    self.planet.scorch_selected_planet_record,
                );
            }
            PlanetScorchPromptMode::Confirm3 => return self.finish_planet_scorch(true),
        }
        self.current_screen = self.planet_context_screen();
        Ok(())
    }

    /// Set or rescind the selected planet's Scorch-Earth order and say so.
    fn finish_planet_scorch(&mut self, scorched: bool) -> Result<(), String> {
        let planet_record_index = self
            .planet
            .scorch_selected_planet_record
            .ok_or_else(|| "Choose one of your planets first.".to_string())?;
        let planet_name = self.game_data.planets[planet_record_index - 1]
            .name
            .trim()
            .to_string();
        self.change(|state| {
            state
                .game_data
                .set_planet_scorch_earth(planet_record_index, scorched)?;
            Ok(())
        })
        .map_err(|err| err.to_string())?;
        self.clear_planet_scorch_prompt();
        self.show_planet_context_notice(if scorched {
            format!("Planet \"{planet_name}\" will be scorched at maintenance.")
        } else {
            format!("Scorch-Earth rescinded on planet \"{planet_name}\".")
        });
        Ok(())
    }

    pub(crate) fn planet_scorch_warning_lines(&self) -> Vec<String> {
        let planet_line = self
            .selected_planet_scorch_name_and_coords()
            .map(|(name, coords)| {
                format!(
                    "Planet \"{}\" at {}.",
                    name,
                    crate::screen::format_sector_coords(coords)
                )
            })
            .unwrap_or_default();
        let text: &[&str] = if self.selected_planet_scorched() {
            &[
                "Scorch-Earth is in force here. Rescinding it ends the yearly decay at",
                "the next maintenance. Production and stored points already lost are not",
                "restored, but the planet grows toward its potential again and earns tax.",
            ]
        } else {
            &[
                "Scorch-Earth is a drastic policy in which a planet destroys anything and",
                "everything potentially usable by an invading force. From the next",
                "maintenance, and every year until you rescind it, the planet's production",
                "falls to about a third and its stored points to a tenth or less. It does",
                "not grow or earn tax while the order stands. Potential production is not",
                "lost, so the planet recovers once the order is rescinded.",
            ]
        };
        let mut lines = vec![planet_line, String::new()];
        lines.extend(text.iter().map(|line| line.to_string()));
        lines
    }

    pub(crate) fn planet_scorch_confirm_prompt(&self) -> Option<&'static str> {
        match self.planet.scorch_prompt_mode {
            Some(PlanetScorchPromptMode::Confirm1) if self.selected_planet_scorched() => {
                Some("Rescind Scorch-Earth? Y/[N] -> ")
            }
            Some(PlanetScorchPromptMode::Confirm1) => Some("Are you sure? Y/[N] -> "),
            Some(PlanetScorchPromptMode::Confirm2) => Some("Are you really sure? Y/[N] -> "),
            Some(PlanetScorchPromptMode::Confirm3) => {
                Some("Are you sure-sure? Last chance to bail! Y/[N] -> ")
            }
            _ => None,
        }
    }

    pub(crate) fn planet_list_scorch_confirm_prompt(&self) -> Option<&'static str> {
        match self.planet.scorch_prompt_mode {
            Some(PlanetScorchPromptMode::Confirm1) if self.selected_planet_scorched() => {
                Some(PLANET_LIST_SCORCH_RESCIND_PROMPT)
            }
            Some(PlanetScorchPromptMode::Confirm1) => Some(PLANET_LIST_SCORCH_CONFIRM_PROMPT),
            Some(PlanetScorchPromptMode::Confirm2) => {
                Some(PLANET_LIST_SCORCH_REALLY_CONFIRM_PROMPT)
            }
            Some(PlanetScorchPromptMode::Confirm3) => Some(PLANET_LIST_SCORCH_LAST_CONFIRM_PROMPT),
            _ => None,
        }
    }

    pub(crate) fn default_planet_scorch_coords(&self) -> Option<[u8; 2]> {
        let mut rows = self
            .game_data
            .empire_planet_economy_rows(self.player.record_index_1_based);
        rows.sort_by(|left, right| {
            left.present_production
                .cmp(&right.present_production)
                .then_with(|| left.coords.cmp(&right.coords))
        });
        rows.first().map(|row| row.coords)
    }

    pub(crate) fn clear_planet_scorch_prompt(&mut self) {
        self.planet.scorch_prompt_mode = None;
        self.planet.scorch_prompt_input.clear();
        self.planet.scorch_prompt_default_value.clear();
        self.planet.scorch_prompt_default_coords = None;
        self.planet.scorch_prompt_status = None;
        self.planet.scorch_selected_planet_record = None;
    }

    fn open_planet_scorch_prompt_mode(
        &mut self,
        mode: PlanetScorchPromptMode,
        default_coords: Option<[u8; 2]>,
        selected_planet_record: Option<usize>,
    ) {
        self.planet.scorch_prompt_mode = Some(mode);
        self.planet.scorch_prompt_input.clear();
        self.planet.scorch_prompt_status = None;
        self.planet.scorch_prompt_default_coords = default_coords;
        self.planet.scorch_prompt_default_value = default_coords
            .map(format_sector_coords_default)
            .unwrap_or_default();
        if let Some(record_index) = selected_planet_record {
            self.planet.scorch_selected_planet_record = Some(record_index);
        }
    }

    fn resolve_planet_scorch_record(&self, coords: [u8; 2]) -> Result<usize, String> {
        let Some(record_index) = self.game_data.planets.iter().position(|planet| {
            planet.sector.to_array() == coords
                && planet.owner.0 as usize == self.player.record_index_1_based
        }) else {
            return Err(format!(
                "Planet [{},{}] is not one of your worlds.",
                coords[0], coords[1]
            ));
        };
        Ok(record_index + 1)
    }

    pub(crate) fn selected_planet_scorched(&self) -> bool {
        self.planet
            .scorch_selected_planet_record
            .and_then(|record| self.game_data.planets.get(record - 1))
            .is_some_and(|planet| planet.scorch_earth)
    }

    pub(crate) fn selected_planet_scorch_name_and_coords(&self) -> Option<(String, [u8; 2])> {
        let record_index = self.planet.scorch_selected_planet_record?;
        let planet = self.game_data.planets.get(record_index - 1)?;
        Some((planet.name.trim().to_string(), planet.sector.to_array()))
    }
}
