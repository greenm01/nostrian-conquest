use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use std::collections::BTreeSet;

use crate::app::Action;
use crate::domains::planet::PlanetAction;
use crate::screen::layout::{
    LEFT_WINDOW_PAD_COL, ScreenGeometry, command_line_row_for, dismiss_prompt_row,
    draw_bottom_aligned_transcript_rows, draw_dismiss_prompt_padded, draw_plain_prompt_padded,
    draw_title_bar_padded, new_playfield, new_playfield_for, standard_table_visible_rows_for,
};
use crate::screen::table::{
    HorizontalAlign, LayoutRect, TableColumn, TableFooter, TableWidthMode, VerticalAlign,
    draw_table_footer, draw_table_title, layout_standard_table_block,
    resolve_table_columns_for_widget, resolve_table_columns_for_widget_with_footer_floor,
    table_footer_scaffold_width, write_table_window_with_cursor_at,
};
use crate::screen::{
    PlayfieldBuffer, Screen, ScreenFrame, format_sector_coords, format_sector_coords_table,
};
use crate::theme::classic;
use nc_data::ProductionItemKind;

pub struct PlanetCommissionScreen;

pub fn planet_commission_picker_visible_rows(geometry: ScreenGeometry) -> usize {
    standard_table_visible_rows_for(geometry, 1)
}

pub fn planet_commission_visible_rows(geometry: ScreenGeometry) -> usize {
    standard_table_visible_rows_for(geometry, 1)
}

pub fn planet_commission_draft_visible_rows(geometry: ScreenGeometry) -> usize {
    standard_table_visible_rows_for(geometry, 1)
}

pub fn planet_auto_commission_report_last_row(geometry: ScreenGeometry) -> usize {
    command_line_row_for(geometry).saturating_sub(2)
}

pub fn planet_auto_commission_report_page_rows(geometry: ScreenGeometry) -> usize {
    planet_auto_commission_report_last_row(geometry) + 1
}

const COMMISSION_PICKER_COLUMNS: [TableColumn<'static>; 9] = [
    TableColumn::left("(X,Y)", 7),
    TableColumn::left("Planet Name", 18),
    TableColumn::right("SC", 2),
    TableColumn::right("BB", 2),
    TableColumn::right("CA", 2),
    TableColumn::right("DD", 2),
    TableColumn::right("TT", 2),
    TableColumn::right("ET", 2),
    TableColumn::right("SB", 2),
];

const COMMISSION_COLUMNS: [TableColumn<'static>; 4] = [
    TableColumn::right("#", 2),
    TableColumn::center("Sel", 3),
    TableColumn::left("Unit", 24),
    TableColumn::right("Qty", 4),
];

const COMMISSION_DRAFT_COLUMNS: [TableColumn<'static>; 3] = [
    TableColumn::left("Unit", 24),
    TableColumn::right("Remaining", 9),
    TableColumn::right("This Fleet", 11),
];

#[derive(Debug, Clone)]
pub struct PlanetCommissionPickerRow {
    pub coords: [u8; 2],
    pub planet_name: String,
    pub destroyers: u32,
    pub cruisers: u32,
    pub battleships: u32,
    pub scouts: u32,
    pub troop_transports: u32,
    pub etacs: u32,
    pub starbases: u32,
}

#[derive(Debug, Clone)]
pub struct PlanetCommissionRow {
    pub slot_0_based: usize,
    pub kind: ProductionItemKind,
    pub unit_label: String,
    pub qty: u32,
}

#[derive(Debug, Clone)]
pub struct PlanetCommissionDraftRow {
    pub direct_slot_0_based: Option<usize>,
    pub kind: ProductionItemKind,
    pub unit_label: String,
    pub remaining_qty: u16,
    pub fleet_qty: u16,
}

#[derive(Debug, Clone)]
pub struct PlanetCommissionView {
    pub planet_name: String,
    pub coords: [u8; 2],
    pub rows: Vec<PlanetCommissionRow>,
}

impl PlanetCommissionScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn render_picker(
        &mut self,
        geometry: ScreenGeometry,
        rows: &[PlanetCommissionPickerRow],
        scroll_offset: usize,
        cursor: usize,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield_for(geometry);

        let table_rows: Vec<Vec<String>> = rows
            .iter()
            .map(|row| {
                vec![
                    format_sector_coords_table(row.coords),
                    row.planet_name.clone(),
                    format_picker_count(row.scouts, 2),
                    format_picker_count(row.battleships, 2),
                    format_picker_count(row.cruisers, 2),
                    format_picker_count(row.destroyers, 2),
                    format_picker_count(row.troop_transports, 2),
                    format_picker_count(row.etacs, 2),
                    format_picker_count(row.starbases, 2),
                ]
            })
            .collect();

        let visible_rows = planet_commission_picker_visible_rows(geometry);
        let displayed_rows = table_rows
            .len()
            .saturating_sub(scroll_offset)
            .min(visible_rows);
        let scrollable = table_rows.len() > visible_rows;
        let default = rows
            .get(cursor.min(rows.len().saturating_sub(1)))
            .map(|row| format!("{:02},{:02}", row.coords[0], row.coords[1]));
        let footer = TableFooter::CommandBar {
            hotkeys_markup: "? <Q>",
            default: default.as_deref(),
            input: "",
        };
        let footer_scaffold_floor = rows
            .iter()
            .map(|row| {
                let default = format!("{:02},{:02}", row.coords[0], row.coords[1]);
                table_footer_scaffold_width(TableFooter::CommandBar {
                    hotkeys_markup: "? <Q>",
                    default: Some(default.as_str()),
                    input: "",
                })
            })
            .max()
            .unwrap_or(0);
        let columns = resolve_table_columns_for_widget_with_footer_floor(
            &COMMISSION_PICKER_COLUMNS,
            &table_rows,
            buffer.width(),
            scrollable,
            TableWidthMode::Compact,
            Some("COMMISSION SHIPS:"),
            Some(footer),
            footer_scaffold_floor,
        );
        let layout = layout_standard_table_block(
            LayoutRect::new(0, 0, buffer.width(), buffer.height()),
            &columns,
            displayed_rows,
            Some("COMMISSION SHIPS:"),
            Some(footer),
            scrollable,
            HorizontalAlign::Center,
            VerticalAlign::Center,
        );
        draw_table_title(
            &mut buffer,
            layout.table_row,
            layout.table_col,
            "COMMISSION SHIPS:",
        );
        let selected = if rows.is_empty() { None } else { Some(cursor) };
        let metrics = write_table_window_with_cursor_at(
            &mut buffer,
            layout.table_row,
            layout.table_col,
            &columns,
            &table_rows,
            scroll_offset,
            visible_rows,
            classic::status_value_style(),
            classic::status_value_style(),
            selected,
            0,
        );

        draw_table_footer(
            &mut buffer,
            geometry,
            layout.command_col,
            metrics.bottom_row,
            footer,
        );
        Ok(buffer)
    }

    pub fn render_menu(
        &mut self,
        geometry: ScreenGeometry,
        view: &PlanetCommissionView,
        scroll_offset: usize,
        cursor: usize,
        selected_slots: &BTreeSet<usize>,
        status: Option<&str>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield_for(geometry);
        let title = format!(
            "COMMISSION SHIPS: \"{}\" IN SYSTEM {}:",
            view.planet_name,
            format_sector_coords(view.coords)
        );

        let table_rows: Vec<Vec<String>> = view
            .rows
            .iter()
            .map(|row| {
                vec![
                    (row.slot_0_based + 1).to_string(),
                    if selected_slots.contains(&row.slot_0_based) {
                        "X".to_string()
                    } else {
                        "".to_string()
                    },
                    row.unit_label.clone(),
                    row.qty.to_string(),
                ]
            })
            .collect();

        let visible_rows = planet_commission_visible_rows(geometry);
        let displayed_rows = table_rows
            .len()
            .saturating_sub(scroll_offset)
            .min(visible_rows);
        let scrollable = table_rows.len() > visible_rows;
        let footer = TableFooter::CommandBar {
            hotkeys_markup: "? SPACE <Q>",
            default: None,
            input: "",
        };
        let columns = resolve_table_columns_for_widget(
            &COMMISSION_COLUMNS,
            &table_rows,
            buffer.width(),
            scrollable,
            TableWidthMode::Compact,
            Some(&title),
            Some(footer),
        );
        let layout = layout_standard_table_block(
            LayoutRect::new(0, 0, buffer.width(), buffer.height()),
            &columns,
            displayed_rows,
            Some(&title),
            Some(footer),
            scrollable,
            HorizontalAlign::Center,
            VerticalAlign::Center,
        );
        draw_table_title(&mut buffer, layout.table_row, layout.table_col, &title);
        let selected = if view.rows.is_empty() {
            None
        } else {
            Some(cursor)
        };
        let metrics = write_table_window_with_cursor_at(
            &mut buffer,
            layout.table_row,
            layout.table_col,
            &columns,
            &table_rows,
            scroll_offset,
            visible_rows,
            classic::status_value_style(),
            classic::status_value_style(),
            selected,
            0,
        );
        draw_table_footer(
            &mut buffer,
            geometry,
            layout.command_col,
            metrics.bottom_row,
            footer,
        );
        let _ = status;
        Ok(buffer)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render_draft(
        &mut self,
        geometry: ScreenGeometry,
        title: &str,
        rows: &[PlanetCommissionDraftRow],
        cursor: usize,
        input: &str,
        status: Option<&str>,
        notice: Option<&str>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield_for(geometry);
        let table_rows: Vec<Vec<String>> = rows
            .iter()
            .enumerate()
            .map(|(idx, row)| {
                let fleet_qty = if !row.accepts_fleet_qty() {
                    String::new()
                } else if idx == cursor && !input.trim().is_empty() {
                    format_draft_input(input)
                } else {
                    format_draft_qty(row.fleet_qty)
                };
                vec![
                    row.unit_label.clone(),
                    format_draft_qty(row.remaining_qty),
                    fleet_qty,
                ]
            })
            .collect();
        let visible_rows = planet_commission_draft_visible_rows(geometry);
        let displayed_rows = table_rows.len().min(visible_rows);
        let scrollable = table_rows.len() > visible_rows;
        let has_ship_draft = rows
            .iter()
            .any(|row| row.accepts_fleet_qty() && row.fleet_qty > 0);
        let current_row = rows.get(cursor);
        let current_is_ship = current_row
            .map(PlanetCommissionDraftRow::accepts_fleet_qty)
            .unwrap_or(false);
        let prompt_label = current_row
            .map(|row| format!("Qty for {} ", row.unit_label))
            .unwrap_or_else(|| "Qty ".to_string());
        let default_qty = current_row
            .map(|row| format_draft_qty(row.remaining_qty))
            .unwrap_or_else(|| "00".to_string());
        let footer = if current_is_ship {
            TableFooter::CommandInput {
                label: "COMMAND",
                prompt: &prompt_label,
                default: &default_qty,
                input,
            }
        } else {
            TableFooter::CommandPrompt {
                label: "COMMAND",
                prompt: if has_ship_draft {
                    "<ENTER> Commission fleet <Q> -> "
                } else {
                    "<ENTER> Commission starbase <Q> -> "
                },
            }
        };
        let footer_scaffold_floor = rows
            .iter()
            .filter(|row| row.accepts_fleet_qty())
            .map(|row| {
                let prompt_label = format!("Qty for {} ", row.unit_label);
                let default_qty = format_draft_qty(row.remaining_qty);
                table_footer_scaffold_width(TableFooter::CommandInput {
                    label: "COMMAND",
                    prompt: prompt_label.as_str(),
                    default: default_qty.as_str(),
                    input: "",
                })
            })
            .max()
            .unwrap_or_else(|| table_footer_scaffold_width(footer));
        let columns = resolve_table_columns_for_widget_with_footer_floor(
            &COMMISSION_DRAFT_COLUMNS,
            &table_rows,
            buffer.width(),
            scrollable,
            TableWidthMode::Compact,
            Some(title),
            Some(footer),
            footer_scaffold_floor,
        );
        let layout = layout_standard_table_block(
            LayoutRect::new(0, 0, buffer.width(), buffer.height()),
            &columns,
            displayed_rows,
            Some(title),
            Some(footer),
            scrollable,
            HorizontalAlign::Center,
            VerticalAlign::Center,
        );
        draw_table_title(&mut buffer, layout.table_row, layout.table_col, title);
        let metrics = write_table_window_with_cursor_at(
            &mut buffer,
            layout.table_row,
            layout.table_col,
            &columns,
            &table_rows,
            0,
            visible_rows,
            classic::status_value_style(),
            classic::status_value_style(),
            if rows.is_empty() { None } else { Some(cursor) },
            0,
        );
        if let Some(notice_text) = notice {
            // Override the footer with the commission notice + slap-a-key prompt.
            let prompt = format!("(Slap a key) {}", notice_text);
            crate::screen::layout::draw_plain_prompt_at_col(
                &mut buffer,
                crate::screen::layout::table_prompt_row_for(geometry, metrics.bottom_row),
                layout.command_col,
                &prompt,
            );
        } else {
            draw_table_footer(
                &mut buffer,
                geometry,
                layout.command_col,
                metrics.bottom_row,
                footer,
            );
        }
        let _ = status;
        Ok(buffer)
    }

    pub fn render_result(
        &mut self,
        title: &str,
        notice: &str,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        draw_title_bar_padded(&mut buffer, 0, title);
        buffer.write_spans(
            2,
            LEFT_WINDOW_PAD_COL,
            &[
                crate::screen::StyledSpan::new("Notice: ", classic::notice_style()),
                crate::screen::StyledSpan::new(notice, classic::status_value_style()),
            ],
        );
        draw_dismiss_prompt_padded(&mut buffer, dismiss_prompt_row(2));
        Ok(buffer)
    }

    pub fn render_auto_commission_report(
        &mut self,
        geometry: ScreenGeometry,
        rows: &[String],
        revealed_rows: usize,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield_for(geometry);
        let last_row = planet_auto_commission_report_last_row(geometry);
        draw_bottom_aligned_transcript_rows(
            &mut buffer,
            rows,
            revealed_rows,
            0,
            last_row,
            |buffer, row, line| write_auto_commission_report_line(buffer, row, line),
        );
        draw_plain_prompt_padded(&mut buffer, command_line_row_for(geometry), "(slap a key)");
        Ok(buffer)
    }

    pub fn handle_picker_key(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                Action::Planet(PlanetAction::MoveCommissionPlanet(-1))
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                Action::Planet(PlanetAction::MoveCommissionPlanet(1))
            }
            KeyCode::PageUp => Action::Planet(PlanetAction::MoveCommissionPlanet(-8)),
            KeyCode::PageDown => Action::Planet(PlanetAction::MoveCommissionPlanet(8)),
            KeyCode::Enter => Action::Planet(PlanetAction::OpenCommissionPlanet),
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                Action::Planet(PlanetAction::OpenMenu)
            }
            _ => Action::Noop,
        }
    }

    pub fn handle_detail_key(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                Action::Planet(PlanetAction::MoveCommissionRow(-1))
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                Action::Planet(PlanetAction::MoveCommissionRow(1))
            }
            KeyCode::PageUp => Action::Planet(PlanetAction::MoveCommissionRow(-8)),
            KeyCode::PageDown => Action::Planet(PlanetAction::MoveCommissionRow(8)),
            KeyCode::Char(' ') => Action::Planet(PlanetAction::ToggleCommissionSelection),
            KeyCode::Enter => Action::Planet(PlanetAction::CommissionStardockSelection),
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                Action::Planet(PlanetAction::CloseCommissionPlanet)
            }
            _ => Action::Noop,
        }
    }

    pub fn handle_draft_key(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                Action::Planet(PlanetAction::MoveCommissionDraftRow(-1))
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                Action::Planet(PlanetAction::MoveCommissionDraftRow(1))
            }
            KeyCode::PageUp => Action::Planet(PlanetAction::MoveCommissionDraftRow(-8)),
            KeyCode::PageDown => Action::Planet(PlanetAction::MoveCommissionDraftRow(8)),
            KeyCode::Backspace => Action::Planet(PlanetAction::BackspaceCommissionDraftInput),
            KeyCode::Char(ch) if ch.is_ascii_digit() => {
                Action::Planet(PlanetAction::AppendCommissionDraftChar(ch))
            }
            KeyCode::Enter => Action::Planet(PlanetAction::SubmitCommissionDraft),
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                Action::Planet(PlanetAction::CloseCommissionDraft)
            }
            _ => Action::Noop,
        }
    }

    pub fn handle_result_key(&self, key: KeyEvent) -> Action {
        if key.kind == KeyEventKind::Press {
            Action::Planet(PlanetAction::DismissCommissionResult(key.code))
        } else {
            Action::Noop
        }
    }

    pub fn handle_auto_commission_report_key(&self, key: KeyEvent) -> Action {
        if key.kind == KeyEventKind::Press {
            Action::Planet(PlanetAction::AdvanceAutoCommissionReport)
        } else {
            Action::Noop
        }
    }
}

impl Screen for PlanetCommissionScreen {
    fn render(
        &mut self,
        _frame: &ScreenFrame<'_>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        Ok(new_playfield())
    }

    fn handle_key(&self, key: KeyEvent) -> Action {
        self.handle_detail_key(key)
    }
}

impl PlanetCommissionDraftRow {
    pub fn accepts_fleet_qty(&self) -> bool {
        self.direct_slot_0_based.is_none() && is_commission_ship_kind(self.kind)
    }
}

fn format_picker_count(value: u32, width: usize) -> String {
    if value == 0 {
        String::new()
    } else {
        format!("{value:0width$}")
    }
}

fn write_auto_commission_report_line(buffer: &mut PlayfieldBuffer, row: usize, line: &str) {
    let mut col = 0;
    if let Some(rest) = line.strip_prefix("Fleet ") {
        col += buffer.write_text(row, col, "Fleet ", classic::body_style());
        let digits = rest
            .as_bytes()
            .iter()
            .take_while(|byte| byte.is_ascii_digit())
            .count();
        if digits > 0 {
            col += buffer.write_text(row, col, &rest[..digits], classic::status_value_style());
            write_auto_commission_body_with_coords(buffer, row, col, &rest[digits..]);
            return;
        }
    }
    if let Some(rest) = line.strip_prefix("Starbase ") {
        col += buffer.write_text(row, col, "Starbase ", classic::body_style());
        let digits = rest
            .as_bytes()
            .iter()
            .take_while(|byte| byte.is_ascii_digit())
            .count();
        if digits > 0 {
            col += buffer.write_text(row, col, &rest[..digits], classic::status_value_style());
            write_auto_commission_body_with_coords(buffer, row, col, &rest[digits..]);
            return;
        }
    }
    write_auto_commission_body_with_coords(buffer, row, col, line);
}

fn write_auto_commission_body_with_coords(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    mut col: usize,
    text: &str,
) {
    let mut remaining = text;
    while let Some(start) = find_coord_start(remaining) {
        col += buffer.write_text(row, col, &remaining[..start + 1], classic::body_style());
        col += buffer.write_text(
            row,
            col,
            &remaining[start + 1..start + 3],
            classic::status_value_style(),
        );
        col += buffer.write_text(row, col, ",", classic::body_style());
        col += buffer.write_text(
            row,
            col,
            &remaining[start + 4..start + 6],
            classic::status_value_style(),
        );
        col += buffer.write_text(row, col, ")", classic::body_style());
        remaining = &remaining[start + 7..];
    }
    buffer.write_text(row, col, remaining, classic::body_style());
}

fn find_coord_start(text: &str) -> Option<usize> {
    let bytes = text.as_bytes();
    for idx in 0..bytes.len().saturating_sub(6) {
        if bytes[idx] == b'('
            && bytes[idx + 1].is_ascii_digit()
            && bytes[idx + 2].is_ascii_digit()
            && bytes[idx + 3] == b','
            && bytes[idx + 4].is_ascii_digit()
            && bytes[idx + 5].is_ascii_digit()
            && bytes[idx + 6] == b')'
        {
            return Some(idx);
        }
    }
    None
}

fn format_draft_qty(value: u16) -> String {
    format!("{value:02}")
}

fn format_draft_input(input: &str) -> String {
    input
        .trim()
        .parse::<u16>()
        .map(format_draft_qty)
        .unwrap_or_else(|_| input.trim().to_string())
}

fn is_commission_ship_kind(kind: ProductionItemKind) -> bool {
    matches!(
        kind,
        ProductionItemKind::Destroyer
            | ProductionItemKind::Cruiser
            | ProductionItemKind::Battleship
            | ProductionItemKind::Scout
            | ProductionItemKind::Transport
            | ProductionItemKind::Etac
    )
}
