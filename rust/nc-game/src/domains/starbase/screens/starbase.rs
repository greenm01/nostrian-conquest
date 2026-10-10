use crossterm::event::{KeyCode, KeyEvent};

use crate::app::Action;
use crate::domains::fleet::FleetAction;
use crate::domains::planet::PlanetAction;
use crate::domains::starbase::StarbaseAction;
use crate::domains::starbase::state::StarbaseMovePromptMode;
use crate::domains::starmap::StarmapAction;
use crate::screen::layout::{
    EXPERT_MENU_PROMPT_ROW, MenuEntry, dismiss_prompt_row, draw_command_line_default_input_padded,
    draw_command_line_prompt_text_padded, draw_command_prompt_padded, draw_dismiss_prompt_padded,
    draw_expert_menu_padded, draw_inline_planet_info_prompt_padded, draw_menu_entry,
    draw_menu_notice_padded, draw_prompt_error_after_padded, draw_status_line, draw_title_bar,
    menu_prompt_row, new_playfield, standard_table_visible_rows, standard_table_visible_rows_for,
};
use crate::screen::table::{
    TABLE_TEXT_INSET, TableColumn, TableFooter, draw_table_footer, draw_table_title,
    fit_table_columns_for_widget, fit_table_columns_for_widget_with_footer_floor,
    table_footer_scaffold_width, write_table_window_with_cursor,
};
use crate::screen::{
    PlayfieldBuffer, Screen, ScreenFrame, ScreenGeometry, format_sector_coords_padded,
    format_sector_coords_table,
};
use crate::theme::classic;

pub const STARBASE_VISIBLE_ROWS: usize = standard_table_visible_rows(1);

pub fn starbase_visible_rows(geometry: ScreenGeometry) -> usize {
    standard_table_visible_rows_for(geometry, 1)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StarbaseRow {
    pub base_record_index_1_based: usize,
    pub base_id: u16,
    pub escort_list_label: String,
    pub escort_review_label: String,
    pub coords: [u8; 2],
    pub destination_coords: [u8; 2],
    pub eta_label: String,
    pub operation_label: String,
}

pub struct StarbaseMenuScreen;
pub struct StarbaseListScreen;
pub struct StarbaseReviewScreen;

const TOP_ROW: [MenuEntry<'static>; 2] = [
    MenuEntry::new(29, "X", "pert mode ON/OFF"),
    MenuEntry::new(52, "V", "iew Partial Star Map"),
];

const ROW_1: [MenuEntry<'static>; 3] = [
    MenuEntry::new(2, "H", "elp with commands"),
    MenuEntry::new(29, "S", "tarbases-List"),
    MenuEntry::new(52, "I", "nfo about a Planet"),
];

const ROW_2: [MenuEntry<'static>; 3] = [
    MenuEntry::new(2, "Q", "uit to Fleet Command"),
    MenuEntry::new(29, "R", "eview a Starbase"),
    MenuEntry::new(52, "M", "ove/Halt Starbase"),
];

const STARBASE_COLUMNS: [TableColumn<'static>; 6] = [
    TableColumn::right("ID", 2),
    TableColumn::left("Escort/Guard", 16),
    TableColumn::left("Location", 14),
    TableColumn::left("Destination", 14),
    TableColumn::right("ETA", 4),
    TableColumn::left("Present Operation", 23),
];

impl StarbaseMenuScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn render_with_notice(
        &mut self,
        notice: Option<&str>,
        expert_mode: bool,
        inline_planet_info: bool,
        info_default_coords: [u8; 2],
        info_input: &str,
        info_notice: Option<&str>,
        move_prompt_mode: Option<StarbaseMovePromptMode>,
        move_prompt_label: Option<&str>,
        move_prompt_default: &str,
        move_prompt_input: &str,
        move_prompt_status: Option<&str>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        if expert_mode {
            if let Some(mode) = move_prompt_mode {
                draw_starbase_move_prompt(
                    &mut buffer,
                    EXPERT_MENU_PROMPT_ROW,
                    mode,
                    move_prompt_label,
                    move_prompt_default,
                    move_prompt_input,
                    move_prompt_status,
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
                    "STARBASE COMMAND",
                    "? X S R V I M <Q>",
                    notice,
                );
            }
            return Ok(buffer);
        }
        draw_title_bar(&mut buffer, 0, "STARBASE CONTROL:");
        for entry in TOP_ROW {
            draw_menu_entry(&mut buffer, 0, entry.col, entry.hotkey, entry.label);
        }
        for (row_idx, row) in [ROW_1.as_slice(), ROW_2.as_slice()].into_iter().enumerate() {
            buffer.fill_row(row_idx + 1, classic::menu_style());
            for entry in row {
                draw_menu_entry(
                    &mut buffer,
                    row_idx + 1,
                    entry.col,
                    entry.hotkey,
                    entry.label,
                );
            }
        }
        let command_row = menu_prompt_row(2);
        if let Some(mode) = move_prompt_mode {
            draw_starbase_move_prompt(
                &mut buffer,
                command_row,
                mode,
                move_prompt_label,
                move_prompt_default,
                move_prompt_input,
                move_prompt_status,
            );
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
        if !inline_planet_info && move_prompt_mode.is_none() {
            draw_command_prompt_padded(
                &mut buffer,
                command_row,
                "STARBASE COMMAND",
                "? X S R V I M <Q>",
            );
        }
        Ok(buffer)
    }
}

impl Screen for StarbaseMenuScreen {
    fn render(
        &mut self,
        _frame: &ScreenFrame<'_>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        self.render_with_notice(
            None,
            false,
            false,
            [0, 0],
            "",
            None,
            None,
            None,
            "",
            "",
            None,
        )
    }

    fn handle_key(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Char('h') | KeyCode::Char('H') => Action::OpenPopupHelp,
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                Action::Fleet(FleetAction::OpenMenu)
            }
            KeyCode::Char('x') | KeyCode::Char('X') => Action::ToggleExpertMode,
            KeyCode::Char('s') | KeyCode::Char('S') => Action::Starbase(StarbaseAction::OpenList),
            KeyCode::Char('r') | KeyCode::Char('R') => {
                Action::Starbase(StarbaseAction::OpenReviewSelect)
            }
            KeyCode::Char('m') | KeyCode::Char('M') => {
                Action::Starbase(StarbaseAction::OpenMovePrompt)
            }
            KeyCode::Char('v') | KeyCode::Char('V') => Action::Starmap(
                StarmapAction::OpenPartialView(crate::screen::CommandMenu::Starbase),
            ),
            KeyCode::Char('i') | KeyCode::Char('I') => Action::Planet(
                PlanetAction::OpenInfoPrompt(crate::screen::CommandMenu::Starbase),
            ),
            _ => Action::Noop,
        }
    }
}

fn draw_starbase_move_prompt(
    buffer: &mut PlayfieldBuffer,
    command_row: usize,
    mode: StarbaseMovePromptMode,
    prompt_label: Option<&str>,
    prompt_default: &str,
    prompt_input: &str,
    prompt_status: Option<&str>,
) {
    match mode {
        StarbaseMovePromptMode::HaltConfirm => {
            draw_command_line_prompt_text_padded(
                buffer,
                command_row,
                "STARBASE COMMAND",
                "Halt this starbase? [Y]/N -> ",
            );
        }
        StarbaseMovePromptMode::Decision => {
            draw_command_line_prompt_text_padded(
                buffer,
                command_row,
                "STARBASE COMMAND",
                "<H>alt or [M]ove <Q> -> ",
            );
        }
        StarbaseMovePromptMode::Base | StarbaseMovePromptMode::Destination => {
            draw_command_line_default_input_padded(
                buffer,
                command_row,
                "STARBASE COMMAND",
                prompt_label.unwrap_or("Command "),
                prompt_default,
                prompt_input,
            );
        }
    }
    if let Some(status) = prompt_status {
        draw_prompt_error_after_padded(buffer, command_row, status);
    }
}

impl StarbaseListScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn render(
        &mut self,
        geometry: ScreenGeometry,
        rows: &[StarbaseRow],
        scroll_offset: usize,
        cursor: usize,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = crate::screen::layout::new_playfield_for(geometry);
        draw_table_title(&mut buffer, 1, 0, "STARBASE LIST:");
        let table_rows = rows
            .iter()
            .map(|row| {
                vec![
                    row.base_id.to_string(),
                    row.escort_list_label.clone(),
                    format!("System{}", format_sector_coords_table(row.coords)),
                    format!(
                        "System{}",
                        format_sector_coords_table(row.destination_coords)
                    ),
                    row.eta_label.clone(),
                    starbase_list_operation_label(&row.operation_label),
                ]
            })
            .collect::<Vec<_>>();
        let footer = TableFooter::CommandBar {
            hotkeys_markup: "? <Q>",
            default: None,
            input: "",
        };
        let columns = fit_table_columns_for_widget(
            &STARBASE_COLUMNS,
            &table_rows,
            Some("STARBASE LIST:"),
            Some(footer),
        );
        let metrics = write_table_window_with_cursor(
            &mut buffer,
            1,
            &columns,
            &table_rows,
            scroll_offset,
            starbase_visible_rows(geometry),
            classic::status_value_style(),
            classic::status_value_style(),
            if table_rows.is_empty() {
                None
            } else {
                Some(cursor)
            },
            0,
        );
        draw_table_footer(
            &mut buffer,
            geometry,
            TABLE_TEXT_INSET,
            metrics.bottom_row,
            footer,
        );
        Ok(buffer)
    }

    pub fn handle_key(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                Action::Starbase(StarbaseAction::MoveSelect(-1))
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                Action::Starbase(StarbaseAction::MoveSelect(1))
            }
            KeyCode::PageUp => Action::Starbase(StarbaseAction::MoveSelect(-8)),
            KeyCode::PageDown => Action::Starbase(StarbaseAction::MoveSelect(8)),
            KeyCode::Enter => Action::Starbase(StarbaseAction::OpenReview),
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                Action::Starbase(StarbaseAction::OpenMenu)
            }
            _ => Action::Noop,
        }
    }
}

impl StarbaseReviewScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn render_select(
        &mut self,
        geometry: ScreenGeometry,
        rows: &[StarbaseRow],
        scroll_offset: usize,
        cursor: usize,
        input: &str,
        _status: Option<&str>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = crate::screen::layout::new_playfield_for(geometry);
        draw_table_title(&mut buffer, 1, 0, "REVIEW A STARBASE:");
        let table_rows = rows
            .iter()
            .map(|row| {
                vec![
                    row.base_id.to_string(),
                    row.escort_list_label.clone(),
                    format!("System{}", format_sector_coords_table(row.coords)),
                    format!(
                        "System{}",
                        format_sector_coords_table(row.destination_coords)
                    ),
                    row.eta_label.clone(),
                    starbase_list_operation_label(&row.operation_label),
                ]
            })
            .collect::<Vec<_>>();
        let default_base = rows
            .get(cursor)
            .map(|row| row.base_id.to_string())
            .unwrap_or_else(|| "1".to_string());
        let footer = if rows.is_empty() {
            TableFooter::CommandBar {
                hotkeys_markup: "? <Q>",
                default: None,
                input: "",
            }
        } else {
            TableFooter::CommandBar {
                hotkeys_markup: "? <Q>",
                default: Some(&default_base),
                input,
            }
        };
        let footer_scaffold_floor = rows
            .iter()
            .map(|row| {
                table_footer_scaffold_width(TableFooter::CommandBar {
                    hotkeys_markup: "? <Q>",
                    default: Some(&row.base_id.to_string()),
                    input: "",
                })
            })
            .max()
            .unwrap_or(0);
        let columns = fit_table_columns_for_widget_with_footer_floor(
            &STARBASE_COLUMNS,
            &table_rows,
            Some("REVIEW A STARBASE:"),
            Some(footer),
            footer_scaffold_floor,
        );
        let metrics = write_table_window_with_cursor(
            &mut buffer,
            1,
            &columns,
            &table_rows,
            scroll_offset,
            starbase_visible_rows(geometry),
            classic::status_value_style(),
            classic::status_value_style(),
            if table_rows.is_empty() {
                None
            } else {
                Some(cursor)
            },
            0,
        );
        draw_table_footer(
            &mut buffer,
            geometry,
            TABLE_TEXT_INSET,
            metrics.bottom_row,
            footer,
        );
        Ok(buffer)
    }

    pub fn render_detail(
        &mut self,
        row: &StarbaseRow,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        draw_title_bar(&mut buffer, 0, &format!("REVIEW STARBASE {}:", row.base_id));
        draw_status_line(
            &mut buffer,
            3,
            "Starbase ID: ",
            &format!("Starbase {}", row.base_id),
        );
        draw_status_line(
            &mut buffer,
            4,
            "Location:    ",
            &format!(
                "World in Solar System {}",
                format_sector_coords_padded(row.coords)
            ),
        );
        draw_status_line(
            &mut buffer,
            5,
            "Destination: ",
            &format!(
                "World in Solar System {}",
                format_sector_coords_padded(row.destination_coords)
            ),
        );
        draw_status_line(&mut buffer, 6, "Operation:   ", &row.operation_label);
        let eta_text = if row.coords == row.destination_coords {
            format!(
                "Starbase {} has already arrived and is in operation.",
                row.base_id
            )
        } else {
            format!(
                "Starbase {} is in transit with ETA {} years.",
                row.base_id, row.eta_label
            )
        };
        draw_status_line(&mut buffer, 7, "ETA:         ", &eta_text);
        draw_status_line(&mut buffer, 8, "Escort:      ", &row.escort_review_label);
        buffer.write_text(10, 0, &"-".repeat(79), classic::help_panel_style());
        draw_dismiss_prompt_padded(&mut buffer, dismiss_prompt_row(10));
        Ok(buffer)
    }
}

fn starbase_list_operation_label(operation: &str) -> String {
    match operation {
        "Protection & Enhancement" => "Protect & Enhance".to_string(),
        other => other.to_string(),
    }
}

impl Screen for StarbaseReviewScreen {
    fn render(
        &mut self,
        _frame: &ScreenFrame<'_>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        Err("use starbase review render helpers".into())
    }

    fn handle_key(&self, _key: KeyEvent) -> Action {
        Action::Noop
    }
}
