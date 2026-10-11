use crossterm::event::{KeyCode, KeyEvent};
use nc_ui::table_filter::{TableFilterClause, is_filter_column_char};
use std::collections::BTreeSet;

use crate::app::Action;
use crate::domains::fleet::FleetAction;
use crate::domains::fleet::missions::FLEET_MISSION_OPTIONS;
use crate::domains::fleet::state::FleetMenuPromptMode;
use crate::domains::planet::PlanetAction;
use crate::domains::starbase::StarbaseAction;
use crate::domains::starmap::StarmapAction;
use crate::screen::layout::{
    EXPERT_MENU_PROMPT_ROW, LEFT_WINDOW_PAD_COL, MenuEntry, PRIMARY_MENU_ROW,
    PRIMARY_MENU_TITLE_COL, PromptFeedback, dismiss_prompt_row, draw_alert_line_at_col,
    draw_command_line_default_input_padded, draw_command_prompt_padded, draw_dismiss_prompt_padded,
    draw_expert_menu_padded, draw_inline_planet_info_prompt_padded, draw_menu_entry_item,
    draw_menu_notice_padded, draw_notice_line_at_col, draw_prompt_error_after_padded,
    draw_prompt_feedback_after_padded, draw_status_line, draw_title_bar_at_col,
    draw_title_bar_padded, draw_wrapped_message, last_body_row, menu_prompt_row, new_playfield,
    standard_table_visible_rows, standard_table_visible_rows_for, wrap_text,
};
use crate::screen::table::{
    HorizontalAlign, LayoutRect, TABLE_TEXT_INSET, TableColumn, TableFooter, TableRowState,
    TableWidthMode, VerticalAlign, draw_table_footer, draw_table_title, fit_table_columns,
    fleet_id_column_width, format_fleet_number, layout_standard_table_block,
    resolve_table_columns_for_widget, resolve_table_columns_for_widget_with_footer_floor,
    table_footer_scaffold_width, write_table_window_with_cursor_at,
    write_table_window_with_states_at,
};
use crate::screen::{
    COMMAND_LABEL, PlanetTransportMode, PlayfieldBuffer, Screen, ScreenFrame, ScreenGeometry,
    SortDirection, StyledSpan, format_sector_coords, format_sector_coords_table,
};
use crate::theme::classic;

pub const FLEET_VISIBLE_ROWS: usize = standard_table_visible_rows(3);
pub const FLEET_LIST_VISIBLE_ROWS: usize = standard_table_visible_rows(1);

pub fn fleet_visible_rows(geometry: ScreenGeometry) -> usize {
    standard_table_visible_rows_for(geometry, 3)
}

pub fn fleet_list_visible_rows(geometry: ScreenGeometry) -> usize {
    standard_table_visible_rows_for(geometry, 1)
}

fn truncate_to_width(value: &str, width: usize) -> String {
    let chars = value.chars().collect::<Vec<_>>();
    if chars.len() <= width {
        return value.to_string();
    }
    if width == 0 {
        return String::new();
    }
    if width <= 3 {
        return ".".repeat(width);
    }
    let mut truncated = chars.into_iter().take(width - 3).collect::<String>();
    truncated.push_str("...");
    truncated
}

fn fleet_list_dismiss_prompt(geometry: ScreenGeometry, table_col: usize, message: &str) -> String {
    let prefix = "(slap a key)";
    let prompt_width = geometry
        .width()
        .saturating_sub(table_col + COMMAND_LABEL.chars().count() + 4);
    if prompt_width <= prefix.chars().count() {
        return truncate_to_width(prefix, prompt_width);
    }
    let message_width = prompt_width.saturating_sub(prefix.chars().count() + 1);
    format!("{prefix} {}", truncate_to_width(message, message_width))
}

fn filter_prompt_dismiss_prompt(message: &str) -> String {
    format!("{message} (slap a key)")
}

fn fit_fleet_table_ships_summary(summary: &str, width: usize) -> String {
    if summary.chars().count() <= width {
        return summary.to_string();
    }
    if width == 0 {
        return String::new();
    }

    let mut kept = Vec::new();
    let mut used_width = 0usize;
    for token in summary.split_whitespace() {
        let token_width = token.chars().count();
        let separator_width = usize::from(!kept.is_empty());
        if used_width + separator_width + token_width > width {
            break;
        }
        used_width += separator_width + token_width;
        kept.push(token);
    }

    if kept.is_empty() {
        return "+".to_string();
    }

    while !kept.is_empty() {
        let joined = kept.join(" ");
        if joined.chars().count() + 2 <= width {
            return format!("{joined} +");
        }
        kept.pop();
    }

    "+".to_string()
}

/// The Ships cell of a fleet table row: the composition, and `N damaged`
/// after it only when the whole composition and the note fit the column.
/// Otherwise the cell is what it was before ships could be damaged.
fn fleet_table_ships_cell(row: &FleetRow, width: usize) -> String {
    let ships = fit_fleet_table_ships_summary(&row.table_ships_label, width);
    if row.damaged_ships == 0 || ships != row.table_ships_label {
        return ships;
    }
    let with_note = format!("{ships} {} damaged", row.damaged_ships);
    if with_note.chars().count() <= width {
        with_note
    } else {
        ships
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FleetRow {
    pub fleet_record_index_1_based: usize,
    pub fleet_number: u16,
    pub coords: [u8; 2],
    pub target_coords: [u8; 2],
    pub order_code: u8,
    pub current_speed: u8,
    pub max_speed: u8,
    pub eta_label: String,
    pub list_eta_label: String,
    pub rules_of_engagement: u8,
    pub loaded_armies: u16,
    pub order_label: String,
    pub composition_label: String,
    pub table_ships_label: String,
    /// Crippled ships in the fleet, for the list's "N damaged" where it fits.
    pub damaged_ships: u32,
    /// The crippled ships by class (`BB 2DD`) for the review screen; empty
    /// when none.
    pub damaged_label: String,
    pub join_host_fleet_number: Option<u16>,
}

pub struct FleetMenuScreen;
pub struct FleetListScreen;
pub struct FleetReviewScreen;
pub struct FleetSingleOrderScreen;
pub struct FleetGroupScreen;
pub struct FleetMissionPickerScreen;
pub struct FleetTransferScreen;
pub struct FleetEtaScreen;
pub struct FleetDetachScreen;
pub struct FleetMessageScreen;

struct RenderedFleetList {
    buffer: PlayfieldBuffer,
    bottom_row: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FleetListSort {
    Id,
    Selected,
    Location,
    Order,
    Target,
    Speed,
    Eta,
    Roe,
    Armies,
    Strength,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FleetListFilter {
    All,
    Holding,
    Moving,
    Combat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FleetSingleOrderMode {
    EnteringTarget,
    EnteringTargetX,
    EnteringTargetY,
    /// ECGAME's Seek Home question for a fleet already in its nearest system.
    SeekingNextClosest,
    ConfirmingTarget,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FleetEtaMode {
    EnteringDestination,
    ConfirmingSystemEntry,
    ShowingResult,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FleetDetachClass {
    Battleships,
    Cruisers,
    Destroyers,
    FullTransports,
    EmptyTransports,
    Scouts,
    Etacs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FleetDetachMode {
    ChoosingClass,
    EnteringQuantity(FleetDetachClass),
    AdjustingDonorSpeed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FleetTransferMode {
    ChoosingClass,
    EnteringQuantity(FleetDetachClass),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FleetGroupOrderMode {
    SelectingFleets,
    EnteringTarget,
    EnteringTargetX,
    EnteringTargetY,
    /// ECGAME's Seek Home question for a lone fleet already in its nearest
    /// system.
    SeekingNextClosest,
    ConfirmingTarget,
}

const FLEET_COL_1: usize = 2;
const FLEET_COL_2: usize = 21;
const FLEET_COL_3: usize = 41;
const FLEET_COL_4: usize = 61;

const TOP_ROW: [MenuEntry<'static>; 0] = [];

const ROW_1: [MenuEntry<'static>; 4] = [
    MenuEntry::new(FLEET_COL_1, "H", "elp on Options"),
    MenuEntry::new(FLEET_COL_2, "S", "TARBASE MENU..."),
    MenuEntry::new(FLEET_COL_3, "C", "hg ROE,ID,Speed"),
    MenuEntry::new(FLEET_COL_4, "O", "rder a Fleet"),
];

const ROW_2: [MenuEntry<'static>; 4] = [
    MenuEntry::new(FLEET_COL_1, "Q", "uit: Main Menu"),
    MenuEntry::new(FLEET_COL_2, "E", "TA Calc"),
    MenuEntry::new(FLEET_COL_3, "I", "nfo about Planet"),
    MenuEntry::new(FLEET_COL_4, "M", "erge a Fleet"),
];

const ROW_3: [MenuEntry<'static>; 4] = [
    MenuEntry::new(FLEET_COL_1, "X", "pert Mode"),
    MenuEntry::featured(FLEET_COL_2, "F", "LEET LIST"),
    MenuEntry::new(FLEET_COL_3, "D", "etach Ships"),
    MenuEntry::new(FLEET_COL_4, "L", "oad TTs w/Armies"),
];

const ROW_4: [MenuEntry<'static>; 4] = [
    MenuEntry::new(FLEET_COL_1, "V", "iew Partial Map"),
    MenuEntry::new(FLEET_COL_2, "R", "eview a Fleet"),
    MenuEntry::new(FLEET_COL_3, "T", "ransfer Ships"),
    MenuEntry::new(FLEET_COL_4, "U", "nload TT Armies"),
];

const FLEET_LIST_BROWSE_HOTKEYS: &str = "? F S O C E D M T L U SPACE <Q>";
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FleetListFilterPromptMode {
    Column,
    Value,
}

fn fleet_list_title(
    sort: FleetListSort,
    direction: SortDirection,
    filter: FleetListFilter,
    filter_clause: Option<&TableFilterClause>,
) -> String {
    format!(
        "FLEET LIST: {} {} {}",
        fleet_sort_label(sort),
        direction.title_label(),
        filter_clause
            .map(|clause| clause.summary.as_str())
            .unwrap_or(filter_label(filter))
    )
}

const fn fleet_sort_label(sort: FleetListSort) -> &'static str {
    match sort {
        FleetListSort::Id => "ID",
        FleetListSort::Selected => "SEL",
        FleetListSort::Location => "LOC",
        FleetListSort::Order => "ORD",
        FleetListSort::Target => "TAR",
        FleetListSort::Speed => "SPD",
        FleetListSort::Eta => "ETA",
        FleetListSort::Roe => "ROE",
        FleetListSort::Armies => "ARS",
        FleetListSort::Strength => "SHI",
    }
}

fn filter_label(filter: FleetListFilter) -> &'static str {
    match filter {
        FleetListFilter::All => "ALL",
        FleetListFilter::Holding => "HOLD",
        FleetListFilter::Moving => "MOVE",
        FleetListFilter::Combat => "COMBAT",
    }
}

fn mission_picker_columns() -> Vec<TableColumn<'static>> {
    let columns = [
        TableColumn::right("No.", 3),
        TableColumn::left("Mission", "Mission".len()),
        TableColumn::left("Requirements (if any)", "Requirements (if any)".len()),
    ];
    let rows = FLEET_MISSION_OPTIONS
        .iter()
        .map(|option| {
            vec![
                option.code.to_string(),
                option.mission.to_string(),
                option.requirements.to_string(),
            ]
        })
        .collect::<Vec<_>>();
    fit_table_columns(&columns, &rows)
}

impl FleetMenuScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn render_with_notice(
        &mut self,
        notice: Option<&str>,
        expert_mode: bool,
        inline_planet_info: bool,
        menu_prompt_mode: Option<FleetMenuPromptMode>,
        menu_prompt_label: Option<&str>,
        menu_prompt_default: &str,
        menu_prompt_input: &str,
        menu_prompt_status: Option<&PromptFeedback>,
        inline_transport_mode: Option<PlanetTransportMode>,
        inline_transport_summary: Option<&str>,
        info_default_coords: [u8; 2],
        info_input: &str,
        info_notice: Option<&str>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        if expert_mode {
            if inline_planet_info {
                draw_inline_planet_info_prompt_padded(
                    &mut buffer,
                    EXPERT_MENU_PROMPT_ROW,
                    info_default_coords,
                    info_input,
                    info_notice,
                    notice,
                );
            } else if menu_prompt_mode.is_some() {
                draw_command_line_default_input_padded(
                    &mut buffer,
                    EXPERT_MENU_PROMPT_ROW,
                    "FLEET COMMAND",
                    menu_prompt_label.unwrap_or("Command "),
                    menu_prompt_default,
                    menu_prompt_input,
                );
                if let Some(status) = menu_prompt_status {
                    draw_prompt_feedback_after_padded(&mut buffer, EXPERT_MENU_PROMPT_ROW, status);
                }
            } else {
                draw_expert_menu_padded(
                    &mut buffer,
                    "FLEET COMMAND",
                    "? X V S F R E C I D T O M L U <Q>",
                    notice,
                );
            }
            return Ok(buffer);
        }
        buffer.fill_row(0, classic::menu_style());
        draw_title_bar_at_col(
            &mut buffer,
            PRIMARY_MENU_ROW,
            PRIMARY_MENU_TITLE_COL,
            "FLEET COMMAND CENTER:",
        );
        for entry in TOP_ROW {
            draw_menu_entry_item(&mut buffer, PRIMARY_MENU_ROW, entry);
        }
        for (row_idx, row) in [
            ROW_1.as_slice(),
            ROW_2.as_slice(),
            ROW_3.as_slice(),
            ROW_4.as_slice(),
        ]
        .into_iter()
        .enumerate()
        {
            buffer.fill_row(PRIMARY_MENU_ROW + row_idx + 1, classic::menu_style());
            for entry in row {
                draw_menu_entry_item(&mut buffer, PRIMARY_MENU_ROW + row_idx + 1, *entry);
            }
        }
        let command_row = menu_prompt_row(PRIMARY_MENU_ROW + 4);
        if inline_planet_info {
            draw_inline_planet_info_prompt_padded(
                &mut buffer,
                command_row,
                info_default_coords,
                info_input,
                info_notice,
                notice,
            );
        } else if let Some(mode) = inline_transport_mode {
            draw_title_bar_padded(&mut buffer, 6, mode.title());
            if let Some(summary) = inline_transport_summary {
                buffer.write_text(
                    8,
                    LEFT_WINDOW_PAD_COL,
                    summary,
                    classic::status_value_style(),
                );
            }
            const TRANSPORT_COMMAND_ROW: usize = 10;
            draw_command_line_default_input_padded(
                &mut buffer,
                TRANSPORT_COMMAND_ROW,
                "FLEET COMMAND",
                menu_prompt_label.unwrap_or("How many armies? "),
                menu_prompt_default,
                menu_prompt_input,
            );
            if let Some(status) = menu_prompt_status {
                draw_prompt_feedback_after_padded(&mut buffer, TRANSPORT_COMMAND_ROW, status);
            }
        } else if menu_prompt_mode.is_some() {
            draw_command_line_default_input_padded(
                &mut buffer,
                command_row,
                "FLEET COMMAND",
                menu_prompt_label.unwrap_or("Command "),
                menu_prompt_default,
                menu_prompt_input,
            );
            if let Some(status) = menu_prompt_status {
                draw_prompt_feedback_after_padded(&mut buffer, command_row, status);
            }
        } else if let Some(notice) = notice {
            draw_menu_notice_padded(&mut buffer, command_row, notice);
        }
        if !inline_planet_info && menu_prompt_mode.is_none() {
            draw_command_prompt_padded(
                &mut buffer,
                command_row,
                "FLEET COMMAND",
                "? X V S F R E C I D T O M L U <Q>",
            );
        }
        Ok(buffer)
    }
}

impl Screen for FleetMenuScreen {
    fn render(
        &mut self,
        _frame: &ScreenFrame<'_>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        self.render_with_notice(
            None,
            false,
            false,
            None,
            None,
            "",
            "",
            None,
            None,
            None,
            [0, 0],
            "",
            None,
        )
    }

    fn handle_key(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Char('f') | KeyCode::Char('F') => Action::Fleet(FleetAction::OpenList),
            KeyCode::Char('r') | KeyCode::Char('R') => Action::Fleet(FleetAction::OpenReviewPrompt),
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => Action::OpenMainMenu,
            KeyCode::Char('h') | KeyCode::Char('H') => Action::OpenPopupHelp,
            KeyCode::Char('s') | KeyCode::Char('S') => Action::Starbase(StarbaseAction::OpenMenu),
            KeyCode::Char('d') | KeyCode::Char('D') => Action::Fleet(FleetAction::OpenDetach),
            KeyCode::Char('m') | KeyCode::Char('M') => Action::Fleet(FleetAction::OpenMerge),
            KeyCode::Char('o') | KeyCode::Char('O') => Action::Fleet(FleetAction::OpenOrder),
            KeyCode::Char('t') | KeyCode::Char('T') => Action::Fleet(FleetAction::OpenTransfer),
            KeyCode::Char('c') | KeyCode::Char('C') => Action::Fleet(FleetAction::OpenChangePrompt),
            KeyCode::Char('l') | KeyCode::Char('L') => {
                Action::Fleet(FleetAction::OpenTransportLoad)
            }
            KeyCode::Char('u') | KeyCode::Char('U') => {
                Action::Fleet(FleetAction::OpenTransportUnload)
            }
            KeyCode::Char('e') | KeyCode::Char('E') => Action::Fleet(FleetAction::OpenEta),
            KeyCode::Char('v') | KeyCode::Char('V') => Action::Starmap(
                StarmapAction::OpenPartialView(crate::screen::CommandMenu::Fleet),
            ),
            KeyCode::Char('i') | KeyCode::Char('I') => Action::Planet(
                PlanetAction::OpenInfoPrompt(crate::screen::CommandMenu::Fleet),
            ),
            KeyCode::Char('x') | KeyCode::Char('X') => Action::ToggleExpertMode,
            _ => Action::Noop,
        }
    }
}

impl FleetListScreen {
    pub fn new() -> Self {
        Self
    }

    fn render_table(
        &mut self,
        geometry: ScreenGeometry,
        rows: &[FleetRow],
        selected_fleet_record_indexes: &BTreeSet<usize>,
        sort: FleetListSort,
        direction: SortDirection,
        filter: FleetListFilter,
        filter_clause: Option<&TableFilterClause>,
        scroll_offset: usize,
        cursor: usize,
        footer: TableFooter<'_>,
        prompt_feedback_inline: Option<&PromptFeedback>,
    ) -> Result<RenderedFleetList, Box<dyn std::error::Error>> {
        let mut buffer = crate::screen::layout::new_playfield_for(geometry);
        let title = fleet_list_title(sort, direction, filter, filter_clause);
        let max_fleet_number = max_fleet_number(rows);
        let columns = full_columns(max_fleet_number);
        let ships_width = columns
            .last()
            .map(|column| column.width)
            .unwrap_or_default();
        let table_rows = rows
            .iter()
            .map(|row| {
                vec![
                    format_fleet_number(row.fleet_number, max_fleet_number),
                    if selected_fleet_record_indexes.contains(&row.fleet_record_index_1_based) {
                        "X".to_string()
                    } else {
                        "".to_string()
                    },
                    format_sector_coords_table(row.coords),
                    fleet_table_order_label(row.order_code).to_string(),
                    fleet_row_target_label(row),
                    row.current_speed.to_string(),
                    row.list_eta_label.clone(),
                    row.rules_of_engagement.to_string(),
                    row.loaded_armies.to_string(),
                    fleet_table_ships_cell(row, ships_width),
                ]
            })
            .collect::<Vec<_>>();
        let visible_rows = fleet_list_visible_rows(geometry)
            .saturating_sub(usize::from(prompt_feedback_inline.is_some()));
        let layout = layout_standard_table_block(
            LayoutRect::new(0, 0, buffer.width(), buffer.height()),
            &columns,
            visible_rows,
            Some(&title),
            Some(footer),
            table_rows.len() > visible_rows,
            HorizontalAlign::Left,
            VerticalAlign::Top,
        );
        draw_table_title(&mut buffer, layout.table_row, layout.table_col, &title);
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
            layout.command_col,
            metrics.bottom_row,
            footer,
        );
        Ok(RenderedFleetList {
            buffer,
            bottom_row: metrics.bottom_row,
        })
    }

    pub fn render(
        &mut self,
        geometry: ScreenGeometry,
        rows: &[FleetRow],
        sort: FleetListSort,
        direction: SortDirection,
        filter: FleetListFilter,
        scroll_offset: usize,
        cursor: usize,
        input: &str,
        status: Option<&str>,
        dismiss_message: Option<&str>,
        prompt_label: Option<&str>,
        prompt_default: &str,
        prompt_input: &str,
        prompt_status: Option<&PromptFeedback>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        self.render_with_filter_clause(
            geometry,
            rows,
            &BTreeSet::new(),
            sort,
            direction,
            filter,
            None,
            scroll_offset,
            cursor,
            input,
            status,
            dismiss_message,
            prompt_label,
            prompt_default,
            prompt_input,
            prompt_status,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render_with_filter_clause(
        &mut self,
        geometry: ScreenGeometry,
        rows: &[FleetRow],
        selected_fleet_record_indexes: &BTreeSet<usize>,
        sort: FleetListSort,
        direction: SortDirection,
        filter: FleetListFilter,
        filter_clause: Option<&TableFilterClause>,
        scroll_offset: usize,
        cursor: usize,
        input: &str,
        status: Option<&str>,
        dismiss_message: Option<&str>,
        prompt_label: Option<&str>,
        prompt_default: &str,
        prompt_input: &str,
        prompt_status: Option<&PromptFeedback>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let max_fleet_number = max_fleet_number(rows);
        let default_fleet_number = rows
            .get(cursor)
            .map(|row| format_fleet_number(row.fleet_number, max_fleet_number))
            .unwrap_or_else(|| {
                rows.first()
                    .map(|row| format_fleet_number(row.fleet_number, max_fleet_number))
                    .unwrap_or_else(|| format_fleet_number(1, max_fleet_number))
            });
        let prompt_feedback_inline =
            prompt_status.filter(|feedback| fleet_list_feedback_fits_inline(geometry, *feedback));
        let dismiss_prompt = dismiss_message
            .map(|message| fleet_list_dismiss_prompt(geometry, TABLE_TEXT_INSET, message));
        let footer = if rows.is_empty() {
            TableFooter::CommandText {
                label: COMMAND_LABEL,
                text: if filter_clause.is_some() {
                    "No fleets match current filter."
                } else {
                    "You have no active fleets."
                },
            }
        } else if let Some(prompt) = dismiss_prompt.as_deref() {
            TableFooter::CommandPrompt {
                label: COMMAND_LABEL,
                prompt,
            }
        } else if let Some(prompt_label) = prompt_label {
            TableFooter::CommandInput {
                label: COMMAND_LABEL,
                prompt: prompt_label,
                default: prompt_default,
                input: prompt_input,
            }
        } else if let Some(status) = status {
            TableFooter::CommandText {
                label: COMMAND_LABEL,
                text: status,
            }
        } else {
            TableFooter::CommandBar {
                hotkeys_markup: FLEET_LIST_BROWSE_HOTKEYS,
                default: Some(&default_fleet_number),
                input,
            }
        };
        let RenderedFleetList {
            mut buffer,
            bottom_row,
        } = self.render_table(
            geometry,
            rows,
            selected_fleet_record_indexes,
            sort,
            direction,
            filter,
            filter_clause,
            scroll_offset,
            cursor,
            footer,
            prompt_feedback_inline,
        )?;
        if let Some(feedback) = prompt_feedback_inline {
            let status_row = crate::screen::layout::table_prompt_row_for(geometry, bottom_row) + 1;
            match feedback {
                PromptFeedback::Notice(value) => {
                    draw_notice_line_at_col(&mut buffer, status_row, TABLE_TEXT_INSET, value);
                }
                PromptFeedback::Error(value) => {
                    draw_alert_line_at_col(
                        &mut buffer,
                        status_row,
                        TABLE_TEXT_INSET,
                        "Error: ",
                        value,
                    );
                }
                PromptFeedback::Warning(value) => {
                    draw_alert_line_at_col(
                        &mut buffer,
                        status_row,
                        TABLE_TEXT_INSET,
                        "Warning: ",
                        value,
                    );
                }
            }
        }
        Ok(buffer)
    }

    pub fn render_sort_prompt(
        &mut self,
        geometry: ScreenGeometry,
        rows: &[FleetRow],
        sort: FleetListSort,
        direction: SortDirection,
        filter: FleetListFilter,
        scroll_offset: usize,
        cursor: usize,
        prompt_default: &str,
        input: &str,
        status: Option<&str>,
        dismiss_message: Option<&str>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        self.render_sort_prompt_with_filter_clause(
            geometry,
            rows,
            &BTreeSet::new(),
            sort,
            direction,
            filter,
            None,
            scroll_offset,
            cursor,
            prompt_default,
            input,
            status,
            dismiss_message,
        )
    }

    pub fn render_sort_prompt_with_filter_clause(
        &mut self,
        geometry: ScreenGeometry,
        rows: &[FleetRow],
        selected_fleet_record_indexes: &BTreeSet<usize>,
        sort: FleetListSort,
        direction: SortDirection,
        filter: FleetListFilter,
        filter_clause: Option<&TableFilterClause>,
        scroll_offset: usize,
        cursor: usize,
        prompt_default: &str,
        input: &str,
        status: Option<&str>,
        dismiss_message: Option<&str>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let prompt;
        let footer = if let Some(message) = dismiss_message {
            prompt = filter_prompt_dismiss_prompt(message);
            TableFooter::CommandPrompt {
                label: "COMMAND",
                prompt: &prompt,
            }
        } else {
            prompt = status
                .filter(|value| value.trim_start().starts_with("Ambiguous:"))
                .map(|value| value.trim_start().to_string())
                .unwrap_or_else(|| "Sort column [?] ".to_string());
            TableFooter::CommandInput {
                label: "COMMAND",
                prompt: &prompt,
                default: prompt_default,
                input,
            }
        };
        Ok(self
            .render_table(
                geometry,
                rows,
                selected_fleet_record_indexes,
                sort,
                direction,
                filter,
                filter_clause,
                scroll_offset,
                cursor,
                footer,
                None,
            )?
            .buffer)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render_filter_prompt(
        &mut self,
        geometry: ScreenGeometry,
        rows: &[FleetRow],
        sort: FleetListSort,
        direction: SortDirection,
        filter: FleetListFilter,
        scroll_offset: usize,
        cursor: usize,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        self.render_filter_prompt_with_filter_clause(
            geometry,
            rows,
            sort,
            direction,
            filter,
            None,
            scroll_offset,
            cursor,
            FleetListFilterPromptMode::Column,
            "all",
            "",
            None,
            None,
            None,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render_filter_prompt_with_filter_clause(
        &mut self,
        geometry: ScreenGeometry,
        rows: &[FleetRow],
        sort: FleetListSort,
        direction: SortDirection,
        filter: FleetListFilter,
        filter_clause: Option<&TableFilterClause>,
        scroll_offset: usize,
        cursor: usize,
        prompt_mode: FleetListFilterPromptMode,
        prompt_default: &str,
        input: &str,
        status: Option<&str>,
        dismiss_message: Option<&str>,
        pending_column_code: Option<&str>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        self.render_filter_prompt_with_selected_filter_clause(
            geometry,
            rows,
            &BTreeSet::new(),
            sort,
            direction,
            filter,
            filter_clause,
            scroll_offset,
            cursor,
            prompt_mode,
            prompt_default,
            input,
            status,
            dismiss_message,
            pending_column_code,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render_filter_prompt_with_selected_filter_clause(
        &mut self,
        geometry: ScreenGeometry,
        rows: &[FleetRow],
        selected_fleet_record_indexes: &BTreeSet<usize>,
        sort: FleetListSort,
        direction: SortDirection,
        filter: FleetListFilter,
        filter_clause: Option<&TableFilterClause>,
        scroll_offset: usize,
        cursor: usize,
        prompt_mode: FleetListFilterPromptMode,
        prompt_default: &str,
        input: &str,
        status: Option<&str>,
        dismiss_message: Option<&str>,
        pending_column_code: Option<&str>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let prompt;
        let footer = if let Some(message) = dismiss_message {
            prompt = filter_prompt_dismiss_prompt(message);
            TableFooter::CommandPrompt {
                label: "COMMAND",
                prompt: &prompt,
            }
        } else {
            prompt = match prompt_mode {
                FleetListFilterPromptMode::Column => status
                    .filter(|value| value.trim_start().starts_with("Ambiguous:"))
                    .map(|value| value.trim_start().to_string())
                    .unwrap_or_else(|| "Filter column [?] ".to_string()),
                FleetListFilterPromptMode::Value => {
                    let mut prompt = format!("Filter {} ", pending_column_code.unwrap_or("value"));
                    if let Some(status) = status {
                        prompt.push_str(status);
                    }
                    prompt
                }
            };
            TableFooter::CommandInput {
                label: "COMMAND",
                prompt: &prompt,
                default: prompt_default,
                input,
            }
        };
        Ok(self
            .render_table(
                geometry,
                rows,
                selected_fleet_record_indexes,
                sort,
                direction,
                filter,
                filter_clause,
                scroll_offset,
                cursor,
                footer,
                None,
            )?
            .buffer)
    }

    pub fn handle_key(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                Action::Fleet(FleetAction::MoveList(-1))
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                Action::Fleet(FleetAction::MoveList(1))
            }
            KeyCode::PageUp => Action::Fleet(FleetAction::MoveList(-8)),
            KeyCode::PageDown => Action::Fleet(FleetAction::MoveList(8)),
            KeyCode::Enter => Action::Fleet(FleetAction::OpenReview),
            KeyCode::Char('f') | KeyCode::Char('F') => {
                Action::Fleet(FleetAction::OpenListFilterPrompt)
            }
            KeyCode::Char('s') | KeyCode::Char('S') => {
                Action::Fleet(FleetAction::OpenListSortPrompt)
            }
            KeyCode::Char('o') | KeyCode::Char('O') => Action::Fleet(FleetAction::OpenOrder),
            KeyCode::Char('c') | KeyCode::Char('C') => Action::Fleet(FleetAction::OpenChangePrompt),
            KeyCode::Char('e') | KeyCode::Char('E') => Action::Fleet(FleetAction::OpenEta),
            KeyCode::Char('d') | KeyCode::Char('D') => Action::Fleet(FleetAction::OpenDetach),
            KeyCode::Char('m') | KeyCode::Char('M') => Action::Fleet(FleetAction::OpenMerge),
            KeyCode::Char('t') | KeyCode::Char('T') => Action::Fleet(FleetAction::OpenTransfer),
            KeyCode::Char(' ') => Action::Fleet(FleetAction::ToggleGroupOrderSelection),
            KeyCode::Char('l') | KeyCode::Char('L') => {
                Action::Fleet(FleetAction::OpenTransportLoad)
            }
            KeyCode::Char('u') | KeyCode::Char('U') => {
                Action::Fleet(FleetAction::OpenTransportUnload)
            }
            KeyCode::Backspace => Action::Fleet(FleetAction::BackspaceListInput),
            KeyCode::Char(ch) if ch.is_ascii_digit() => {
                Action::Fleet(FleetAction::AppendListChar(ch))
            }
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                Action::Fleet(FleetAction::OpenMenu)
            }
            _ => Action::Noop,
        }
    }

    pub fn handle_sort_prompt_key(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Char('?') => Action::OpenPopupHelp,
            KeyCode::Enter => Action::Fleet(FleetAction::SubmitListSortPrompt),
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                Action::Fleet(FleetAction::CloseListPrompt)
            }
            KeyCode::Backspace => Action::Fleet(FleetAction::BackspaceListFilterPromptInput),
            KeyCode::Char(ch) if is_filter_column_char(ch) => {
                Action::Fleet(FleetAction::AppendListFilterPromptChar(ch))
            }
            _ => Action::Noop,
        }
    }

    pub fn handle_filter_prompt_key(
        &self,
        key: KeyEvent,
        prompt_mode: FleetListFilterPromptMode,
    ) -> Action {
        match key.code {
            KeyCode::Char('?') => Action::OpenPopupHelp,
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                Action::Fleet(FleetAction::CloseListPrompt)
            }
            KeyCode::Enter => Action::Fleet(FleetAction::SubmitListFilterPrompt),
            KeyCode::Backspace => Action::Fleet(FleetAction::BackspaceListFilterPromptInput),
            KeyCode::Char(ch)
                if matches!(prompt_mode, FleetListFilterPromptMode::Column)
                    && is_filter_column_char(ch) =>
            {
                Action::Fleet(FleetAction::AppendListFilterPromptChar(ch))
            }
            KeyCode::Char(ch)
                if matches!(prompt_mode, FleetListFilterPromptMode::Value)
                    && (matches!(
                        ch,
                        ' ' | '-' | '#' | '*' | '/' | '?' | '=' | '!' | '>' | '<' | '+' | ','
                    ) || ch.is_ascii_alphanumeric()) =>
            {
                Action::Fleet(FleetAction::AppendListFilterPromptChar(ch))
            }
            _ => Action::Noop,
        }
    }
}

impl FleetMessageScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn render(&mut self, message: &str) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        draw_title_bar_padded(&mut buffer, 0, "FLEET COMMAND:");
        draw_wrapped_message(&mut buffer, 3, 19, "", message);
        draw_dismiss_prompt_padded(&mut buffer, dismiss_prompt_row(21));
        Ok(buffer)
    }
}

impl FleetReviewScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn render(
        &mut self,
        row: &FleetRow,
        selected_index: usize,
        total: usize,
        _return_to_list: bool,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        buffer.fill_row(0, classic::menu_style());
        buffer.write_text(
            0,
            0,
            &format!("REVIEW FLEET {}/{}:", selected_index + 1, total),
            classic::title_style(),
        );
        draw_status_line(&mut buffer, 2, "Fleet ID: ", &row.fleet_number.to_string());
        draw_status_line(
            &mut buffer,
            3,
            "Location: ",
            &format_sector_coords(row.coords),
        );
        draw_status_line(
            &mut buffer,
            4,
            "Current / Max Speed: ",
            &format!("{}/{}", row.current_speed, row.max_speed),
        );
        draw_status_line(
            &mut buffer,
            5,
            "Rules of Engagement: ",
            &row.rules_of_engagement.to_string(),
        );
        draw_status_line(&mut buffer, 7, "Standing Order: ", &row.order_label);
        draw_status_line(&mut buffer, 9, "Composition: ", &row.table_ships_label);
        if !row.damaged_label.is_empty() {
            draw_status_line(&mut buffer, 10, "Damaged: ", &row.damaged_label);
        }
        draw_dismiss_prompt_padded(&mut buffer, dismiss_prompt_row(9));
        Ok(buffer)
    }

    pub fn handle_key(&self, _key: KeyEvent) -> Action {
        Action::Fleet(FleetAction::CloseReview)
    }
}

impl FleetSingleOrderScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn render(
        &mut self,
        row: &FleetRow,
        current_order_label: &str,
        new_order_label: &str,
        mode: FleetSingleOrderMode,
        header_text: &str,
        target_prompt: &str,
        target_default: &str,
        input: &str,
        target_x_default: &str,
        target_x_input: &str,
        target_y_default: &str,
        target_y_input: &str,
        confirm_input: &str,
        current_year: u16,
        status: Option<&str>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        if matches!(
            mode,
            FleetSingleOrderMode::ConfirmingTarget | FleetSingleOrderMode::SeekingNextClosest
        ) {
            return self.render_confirm_target(
                row,
                new_order_label,
                header_text,
                mode == FleetSingleOrderMode::SeekingNextClosest,
                confirm_input,
                current_year,
                status,
            );
        }
        if mode == FleetSingleOrderMode::EnteringTarget {
            return self.render_named_target_entry(
                row,
                current_order_label,
                header_text,
                target_prompt,
                target_default,
                input,
                status,
            );
        }
        if matches!(
            mode,
            FleetSingleOrderMode::EnteringTargetX | FleetSingleOrderMode::EnteringTargetY
        ) {
            return self.render_coordinate_target_entry(
                row,
                current_order_label,
                new_order_label,
                mode,
                target_x_default,
                target_x_input,
                target_y_default,
                target_y_input,
                status,
            );
        }
        unreachable!("target-entry render path handled above")
    }

    fn render_coordinate_target_entry(
        &mut self,
        row: &FleetRow,
        current_order_label: &str,
        new_order_label: &str,
        mode: FleetSingleOrderMode,
        target_x_default: &str,
        target_x_input: &str,
        target_y_default: &str,
        target_y_input: &str,
        status: Option<&str>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        buffer.fill_row(0, classic::menu_style());
        buffer.write_text(
            0,
            0,
            &format!("ORDER FLEET #{}:", row.fleet_number),
            classic::title_style(),
        );
        draw_status_line(
            &mut buffer,
            2,
            "Location: ",
            &format_sector_coords_table(row.coords),
        );
        draw_status_line(
            &mut buffer,
            3,
            "Current / Max Speed: ",
            &format!("{}/{}", row.current_speed, row.max_speed),
        );
        draw_status_line(
            &mut buffer,
            4,
            "ROE: ",
            &row.rules_of_engagement.to_string(),
        );
        draw_status_line(&mut buffer, 5, "Order: ", current_order_label);
        draw_status_line(&mut buffer, 7, "Ships: ", &row.table_ships_label);
        draw_status_line(
            &mut buffer,
            9,
            "Enter target coordinates for new order: ",
            new_order_label,
        );
        let command_row = menu_prompt_row(9);
        let active_row = match mode {
            FleetSingleOrderMode::EnteringTargetX => {
                draw_command_line_default_input_padded(
                    &mut buffer,
                    command_row,
                    COMMAND_LABEL,
                    "Target XX ",
                    target_x_default,
                    target_x_input,
                );
                command_row
            }
            FleetSingleOrderMode::EnteringTargetY => {
                draw_command_line_default_input_padded(
                    &mut buffer,
                    command_row,
                    COMMAND_LABEL,
                    "Target XX ",
                    target_x_default,
                    target_x_input,
                );
                draw_command_line_default_input_padded(
                    &mut buffer,
                    command_row + 2,
                    COMMAND_LABEL,
                    "Target YY ",
                    target_y_default,
                    target_y_input,
                );
                command_row + 2
            }
            _ => command_row,
        };
        if let Some(status) = status {
            draw_prompt_error_after_padded(&mut buffer, active_row, status);
        }
        Ok(buffer)
    }

    fn render_named_target_entry(
        &mut self,
        row: &FleetRow,
        current_order_label: &str,
        header_text: &str,
        target_prompt: &str,
        target_default: &str,
        input: &str,
        status: Option<&str>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        buffer.fill_row(0, classic::menu_style());
        buffer.write_text(
            0,
            0,
            &format!("ORDER FLEET #{}:", row.fleet_number),
            classic::title_style(),
        );
        draw_status_line(
            &mut buffer,
            2,
            "Location: ",
            &format_sector_coords_table(row.coords),
        );
        draw_status_line(
            &mut buffer,
            3,
            "Current / Max Speed: ",
            &format!("{}/{}", row.current_speed, row.max_speed),
        );
        draw_status_line(
            &mut buffer,
            4,
            "ROE: ",
            &row.rules_of_engagement.to_string(),
        );
        draw_status_line(&mut buffer, 5, "Order: ", current_order_label);
        draw_status_line(&mut buffer, 7, "Ships: ", &row.table_ships_label);
        draw_status_line(&mut buffer, 9, "", header_text);
        let command_row = menu_prompt_row(9);
        draw_command_line_default_input_padded(
            &mut buffer,
            command_row,
            COMMAND_LABEL,
            target_prompt,
            target_default,
            input,
        );
        if let Some(status) = status {
            draw_prompt_error_after_padded(&mut buffer, command_row, status);
        }
        Ok(buffer)
    }

    fn render_confirm_target(
        &mut self,
        row: &FleetRow,
        new_order_label: &str,
        header_text: &str,
        seek_next_closest: bool,
        confirm_input: &str,
        current_year: u16,
        status: Option<&str>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        buffer.fill_row(0, classic::menu_style());
        buffer.write_text(
            0,
            0,
            &format!("ORDER FLEET #{}:", row.fleet_number),
            classic::title_style(),
        );
        draw_status_line(&mut buffer, 2, "Stardate: ", &current_year.to_string());
        draw_status_line(&mut buffer, 4, "", header_text);
        draw_status_line(&mut buffer, 6, "New Orders: ", new_order_label);
        let command_row = menu_prompt_row(6);
        draw_order_confirm_prompt(&mut buffer, command_row, seek_next_closest, confirm_input);
        if let Some(status) = status {
            draw_prompt_error_after_padded(&mut buffer, command_row, status);
        }
        Ok(buffer)
    }
}

impl FleetEtaScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn render(
        &mut self,
        row: &FleetRow,
        mode: FleetEtaMode,
        destination_default: [u8; 2],
        destination_input: &str,
        include_system_input: &str,
        status: Option<&str>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        buffer.fill_row(0, classic::menu_style());
        buffer.write_text(
            0,
            LEFT_WINDOW_PAD_COL,
            "CALCULATE FLEET ETA:",
            classic::title_style(),
        );
        // row 1: blank
        draw_status_line(&mut buffer, 2, "Fleet ID: ", &row.fleet_number.to_string());
        // row 3: blank
        draw_status_line(
            &mut buffer,
            4,
            "Location: ",
            &format_sector_coords_table(row.coords),
        );
        draw_status_line(&mut buffer, 5, "Speed: ", &row.current_speed.to_string());
        // row 6: blank
        draw_status_line(&mut buffer, 7, "Orders: ", &row.order_label);
        draw_status_line(
            &mut buffer,
            8,
            "Target: ",
            &format_sector_coords_table(row.target_coords),
        );
        // row 9: blank
        draw_status_line(&mut buffer, 10, "Ships: ", &row.table_ships_label);
        const LAST_CONTENT_ROW: usize = 10;
        match mode {
            FleetEtaMode::EnteringDestination => {
                let command_row = menu_prompt_row(LAST_CONTENT_ROW);
                draw_command_line_default_input_padded(
                    &mut buffer,
                    command_row,
                    COMMAND_LABEL,
                    "Destination ",
                    &format!("{},{}", destination_default[0], destination_default[1]),
                    destination_input,
                );
                if let Some(err) = status {
                    draw_prompt_error_after_padded(&mut buffer, command_row, err);
                }
            }
            FleetEtaMode::ConfirmingSystemEntry => {
                let command_row = menu_prompt_row(LAST_CONTENT_ROW);
                draw_command_line_default_input_padded(
                    &mut buffer,
                    command_row,
                    COMMAND_LABEL,
                    "Include time to enter system? ",
                    "N",
                    include_system_input,
                );
                if let Some(err) = status {
                    draw_prompt_error_after_padded(&mut buffer, command_row, err);
                }
            }
            FleetEtaMode::ShowingResult => {
                let result_row = dismiss_prompt_row(LAST_CONTENT_ROW);
                draw_status_line(&mut buffer, result_row, "", status.unwrap_or(""));
                draw_dismiss_prompt_padded(&mut buffer, dismiss_prompt_row(result_row));
            }
        }
        Ok(buffer)
    }
}

fn fleet_list_feedback_fits_inline(geometry: ScreenGeometry, feedback: &PromptFeedback) -> bool {
    let label = match feedback {
        PromptFeedback::Notice(_) => "",
        PromptFeedback::Error(_) => "Error: ",
        PromptFeedback::Warning(_) => "Warning: ",
    };
    let width = geometry
        .width()
        .saturating_sub(TABLE_TEXT_INSET + label.len() + 1);
    width > 0 && wrap_text(feedback.message(), width, width).len() <= 1
}

impl FleetGroupScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn render(
        &mut self,
        geometry: ScreenGeometry,
        rows: &[FleetRow],
        scroll_offset: usize,
        cursor: usize,
        selected_fleet_record_indexes: &BTreeSet<usize>,
        mode: FleetGroupOrderMode,
        target_status_line: &str,
        new_order_label: &str,
        target_prompt: &str,
        target_default: &str,
        input: &str,
        target_x_default: &str,
        target_x_input: &str,
        target_y_default: &str,
        target_y_input: &str,
        confirm_input: &str,
        current_year: u16,
        status: Option<&str>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        if mode == FleetGroupOrderMode::SelectingFleets {
            return self.render_selection_table(
                geometry,
                rows,
                scroll_offset,
                cursor,
                selected_fleet_record_indexes,
                status,
            );
        }
        let selected_fleet_label =
            format_selected_fleet_numbers(rows, selected_fleet_record_indexes);
        if matches!(
            mode,
            FleetGroupOrderMode::ConfirmingTarget | FleetGroupOrderMode::SeekingNextClosest
        ) {
            return self.render_confirm_target(
                &selected_fleet_label,
                target_status_line,
                new_order_label,
                mode == FleetGroupOrderMode::SeekingNextClosest,
                confirm_input,
                current_year,
                status,
            );
        }
        return self.render_target_entry(
            &selected_fleet_label,
            mode,
            target_status_line,
            new_order_label,
            target_prompt,
            target_default,
            input,
            target_x_default,
            target_x_input,
            target_y_default,
            target_y_input,
            status,
        );
    }

    fn render_selection_table(
        &mut self,
        geometry: ScreenGeometry,
        rows: &[FleetRow],
        scroll_offset: usize,
        cursor: usize,
        selected_fleet_record_indexes: &BTreeSet<usize>,
        _status: Option<&str>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = crate::screen::layout::new_playfield_for(geometry);
        buffer.fill_row(0, classic::menu_style());
        let max_fleet_number = max_fleet_number(rows);
        let base_columns = group_selection_columns(max_fleet_number);
        let ships_width = base_columns
            .last()
            .map(|column| column.width)
            .unwrap_or_default();
        let table_rows = rows
            .iter()
            .map(|row| {
                vec![
                    format_fleet_number(row.fleet_number, max_fleet_number),
                    if selected_fleet_record_indexes.contains(&row.fleet_record_index_1_based) {
                        "X".to_string()
                    } else {
                        "".to_string()
                    },
                    format_sector_coords_table(row.coords),
                    fleet_table_order_label(row.order_code).to_string(),
                    fleet_row_target_label(row),
                    row.current_speed.to_string(),
                    row.list_eta_label.clone(),
                    row.rules_of_engagement.to_string(),
                    row.loaded_armies.to_string(),
                    fleet_table_ships_cell(row, ships_width),
                ]
            })
            .collect::<Vec<_>>();
        let visible_rows = fleet_visible_rows(geometry);
        let scrollable = table_rows.len() > visible_rows;
        let footer = if table_rows.is_empty() {
            TableFooter::CommandBar {
                hotkeys_markup: "<Q>",
                default: None,
                input: "",
            }
        } else {
            TableFooter::CommandBar {
                hotkeys_markup: "? SPACE <Q>",
                default: None,
                input: "",
            }
        };
        let columns = resolve_table_columns_for_widget(
            &base_columns,
            &table_rows,
            buffer.width(),
            scrollable,
            TableWidthMode::Compact,
            Some("GROUP FLEET ORDER:"),
            Some(footer),
        );
        let layout = layout_standard_table_block(
            LayoutRect::new(0, 0, buffer.width(), buffer.height()),
            &columns,
            visible_rows,
            Some("GROUP FLEET ORDER:"),
            Some(footer),
            scrollable,
            HorizontalAlign::Center,
            VerticalAlign::Top,
        );
        draw_table_title(
            &mut buffer,
            layout.table_row,
            layout.table_col,
            "GROUP FLEET ORDER:",
        );
        let metrics = crate::screen::table::write_table_window_with_states_at(
            &mut buffer,
            layout.table_row,
            layout.table_col,
            &columns,
            &table_rows,
            scroll_offset,
            visible_rows,
            classic::status_value_style(),
            classic::status_value_style(),
            if table_rows.is_empty() {
                None
            } else {
                Some(cursor)
            },
            0,
            None,
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

    fn render_target_entry(
        &mut self,
        selected_fleet_label: &str,
        mode: FleetGroupOrderMode,
        header_text: &str,
        new_order_label: &str,
        target_prompt: &str,
        target_default: &str,
        input: &str,
        target_x_default: &str,
        target_x_input: &str,
        target_y_default: &str,
        target_y_input: &str,
        status: Option<&str>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        buffer.fill_row(0, classic::menu_style());
        buffer.write_text(
            0,
            LEFT_WINDOW_PAD_COL,
            "GROUP FLEET ORDER:",
            classic::title_style(),
        );
        draw_status_line(&mut buffer, 2, "Selected fleets: ", selected_fleet_label);
        if mode == FleetGroupOrderMode::EnteringTarget {
            draw_status_line(&mut buffer, 4, "", header_text);
        } else {
            draw_status_line(
                &mut buffer,
                4,
                "Enter target coordinates for new order: ",
                new_order_label,
            );
        }
        let command_row = menu_prompt_row(4);
        let active_row = match mode {
            FleetGroupOrderMode::EnteringTarget => {
                draw_command_line_default_input_padded(
                    &mut buffer,
                    command_row,
                    COMMAND_LABEL,
                    target_prompt,
                    target_default,
                    input,
                );
                command_row
            }
            FleetGroupOrderMode::EnteringTargetX => {
                draw_command_line_default_input_padded(
                    &mut buffer,
                    command_row,
                    COMMAND_LABEL,
                    "Target XX ",
                    target_x_default,
                    target_x_input,
                );
                command_row
            }
            FleetGroupOrderMode::EnteringTargetY => {
                draw_command_line_default_input_padded(
                    &mut buffer,
                    command_row,
                    COMMAND_LABEL,
                    "Target XX ",
                    target_x_default,
                    target_x_input,
                );
                draw_command_line_default_input_padded(
                    &mut buffer,
                    command_row + 2,
                    COMMAND_LABEL,
                    "Target YY ",
                    target_y_default,
                    target_y_input,
                );
                command_row + 2
            }
            FleetGroupOrderMode::SelectingFleets
            | FleetGroupOrderMode::SeekingNextClosest
            | FleetGroupOrderMode::ConfirmingTarget => command_row,
        };
        if let Some(status) = status {
            draw_prompt_error_after_padded(&mut buffer, active_row, status);
        }
        Ok(buffer)
    }

    fn render_confirm_target(
        &mut self,
        selected_fleet_label: &str,
        header_text: &str,
        new_order_label: &str,
        seek_next_closest: bool,
        confirm_input: &str,
        current_year: u16,
        status: Option<&str>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        buffer.fill_row(0, classic::menu_style());
        buffer.write_text(
            0,
            LEFT_WINDOW_PAD_COL,
            "GROUP FLEET ORDER:",
            classic::title_style(),
        );
        draw_status_line(&mut buffer, 2, "Stardate: ", &current_year.to_string());
        draw_status_line(&mut buffer, 3, "Selected fleets: ", selected_fleet_label);
        draw_status_line(&mut buffer, 5, "", header_text);
        draw_status_line(&mut buffer, 7, "New Orders: ", new_order_label);
        let command_row = menu_prompt_row(7);
        draw_order_confirm_prompt(&mut buffer, command_row, seek_next_closest, confirm_input);
        if let Some(status) = status {
            draw_prompt_error_after_padded(&mut buffer, command_row, status);
        }
        Ok(buffer)
    }
}

impl FleetTransferScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn render(
        &mut self,
        donor_row: &FleetRow,
        host_row: &FleetRow,
        _mode: FleetTransferMode,
        input: &str,
        status: Option<&str>,
        prompt: &str,
        default: &str,
        source_ships: &str,
        destination_ships: &str,
        staged_summary: &str,
        remaining_summary: &str,
        destination_summary: &str,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        const SOURCE_FLEET_ROW: usize = 2;
        const SOURCE_LOCATION_ROW: usize = 3;
        const SOURCE_ORDERS_ROW: usize = 4;
        const SOURCE_TARGET_ROW: usize = 5;
        const SOURCE_SPEED_ROE_ROW: usize = 6;
        const SOURCE_SHIPS_ROW: usize = 7;
        const DEST_FLEET_ROW: usize = 9;
        const DEST_LOCATION_ROW: usize = 10;
        const DEST_ORDERS_ROW: usize = 11;
        const DEST_TARGET_ROW: usize = 12;
        const DEST_SPEED_ROE_ROW: usize = 13;
        const DEST_SHIPS_ROW: usize = 14;
        const ACTION_ROW: usize = 16;
        const COMMAND_ROW: usize = 18;
        const STAGED_ROW: usize = 20;

        let mut buffer = new_playfield();
        buffer.fill_row(0, classic::menu_style());
        buffer.write_text(
            0,
            0,
            "TRANSFER SHIPS BETWEEN FLEETS:",
            classic::title_style(),
        );
        draw_status_line(
            &mut buffer,
            SOURCE_FLEET_ROW,
            "Source Fleet: ",
            &format!("Fleet #{}", donor_row.fleet_number),
        );
        draw_status_line(
            &mut buffer,
            SOURCE_LOCATION_ROW,
            "Location: ",
            &format_sector_coords_table(donor_row.coords),
        );
        draw_status_line(
            &mut buffer,
            SOURCE_ORDERS_ROW,
            "Orders: ",
            &donor_row.order_label,
        );
        draw_status_line(
            &mut buffer,
            SOURCE_TARGET_ROW,
            "Target: ",
            &fleet_list_target_label(donor_row.target_coords),
        );
        draw_status_line(
            &mut buffer,
            SOURCE_SPEED_ROE_ROW,
            "Speed / ROE: ",
            &format!(
                "{} / {}",
                donor_row.current_speed, donor_row.rules_of_engagement
            ),
        );
        draw_status_line(&mut buffer, SOURCE_SHIPS_ROW, "Ships: ", source_ships);

        draw_status_line(
            &mut buffer,
            DEST_FLEET_ROW,
            "Destination Fleet: ",
            &format!("Fleet #{}", host_row.fleet_number),
        );
        draw_status_line(
            &mut buffer,
            DEST_LOCATION_ROW,
            "Location: ",
            &format_sector_coords_table(host_row.coords),
        );
        draw_status_line(
            &mut buffer,
            DEST_ORDERS_ROW,
            "Orders: ",
            &host_row.order_label,
        );
        draw_status_line(
            &mut buffer,
            DEST_TARGET_ROW,
            "Target: ",
            &fleet_list_target_label(host_row.target_coords),
        );
        draw_status_line(
            &mut buffer,
            DEST_SPEED_ROE_ROW,
            "Speed / ROE: ",
            &format!(
                "{} / {}",
                host_row.current_speed, host_row.rules_of_engagement
            ),
        );
        draw_status_line(&mut buffer, DEST_SHIPS_ROW, "Ships: ", destination_ships);
        buffer.write_spans(
            ACTION_ROW,
            0,
            &[
                StyledSpan::new("<", classic::prompt_style()),
                StyledSpan::new("C", classic::prompt_hotkey_style()),
                StyledSpan::new(">ommit, <", classic::prompt_style()),
                StyledSpan::new("X", classic::prompt_hotkey_style()),
                StyledSpan::new("> Cancel", classic::prompt_style()),
            ],
        );
        draw_command_line_default_input_padded(
            &mut buffer,
            COMMAND_ROW,
            COMMAND_LABEL,
            prompt,
            default,
            input,
        );
        let staged_end_row = if staged_summary != "none" {
            draw_status_line(
                &mut buffer,
                STAGED_ROW,
                "Staged to Transfer: ",
                staged_summary,
            );
            draw_status_line(
                &mut buffer,
                STAGED_ROW + 1,
                "Remaining on Source: ",
                remaining_summary,
            );
            draw_status_line(
                &mut buffer,
                STAGED_ROW + 2,
                "Destination After Transfer: ",
                destination_summary,
            );
            STAGED_ROW + 2
        } else {
            draw_status_line(
                &mut buffer,
                STAGED_ROW,
                "Staged to Transfer: ",
                staged_summary,
            );
            STAGED_ROW
        };
        if let Some(status) = status {
            let status_row = (staged_end_row + 2).min(last_body_row());
            let max_rows = last_body_row().saturating_sub(status_row) + 1;
            if max_rows > 0 {
                draw_wrapped_message(&mut buffer, status_row, max_rows, "", status);
            }
        }
        Ok(buffer)
    }
}

impl FleetMissionPickerScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn render(
        &mut self,
        cursor: usize,
        input: &str,
        enabled: &[bool],
        _status: Option<&str>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        let rows = FLEET_MISSION_OPTIONS
            .iter()
            .map(|option| {
                vec![
                    format!("{:02}", option.code),
                    option.mission.to_string(),
                    option.requirements.to_string(),
                ]
            })
            .collect::<Vec<_>>();
        let default = FLEET_MISSION_OPTIONS
            .get(cursor)
            .map(|option| option.code.to_string())
            .unwrap_or_else(|| "1".to_string());
        let footer = TableFooter::CommandBar {
            hotkeys_markup: "? <Q>",
            default: Some(&default),
            input,
        };
        let footer_scaffold_floor = FLEET_MISSION_OPTIONS
            .iter()
            .map(|option| {
                table_footer_scaffold_width(TableFooter::CommandBar {
                    hotkeys_markup: "? <Q>",
                    default: Some(&option.code.to_string()),
                    input: "",
                })
            })
            .max()
            .unwrap_or(0);
        let columns = resolve_table_columns_for_widget_with_footer_floor(
            &mission_picker_columns(),
            &rows,
            buffer.width(),
            false,
            TableWidthMode::Compact,
            Some("FLEET MISSION ORDERS:"),
            Some(footer),
            footer_scaffold_floor,
        );
        let layout = layout_standard_table_block(
            LayoutRect::new(0, 0, buffer.width(), buffer.height()),
            &columns,
            rows.len(),
            Some("FLEET MISSION ORDERS:"),
            Some(footer),
            false,
            HorizontalAlign::Center,
            VerticalAlign::Top,
        );
        let _ = layout.title_row;
        draw_table_title(
            &mut buffer,
            layout.table_row,
            layout.table_col,
            "FLEET MISSION ORDERS:",
        );
        let row_states = enabled
            .iter()
            .map(|enabled| {
                if *enabled {
                    TableRowState::Normal
                } else {
                    TableRowState::Disabled
                }
            })
            .collect::<Vec<_>>();
        let metrics = write_table_window_with_states_at(
            &mut buffer,
            layout.table_row,
            layout.table_col,
            &columns,
            &rows,
            0,
            rows.len(),
            classic::status_value_style(),
            classic::status_value_style(),
            Some(cursor),
            0,
            Some(&row_states),
        );
        draw_table_footer(
            &mut buffer,
            ScreenGeometry::local_default(),
            layout.command_col,
            metrics.bottom_row,
            footer,
        );
        Ok(buffer)
    }
}

impl FleetDetachScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn render(
        &mut self,
        donor_row: &FleetRow,
        prompt: &str,
        default: &str,
        input: &str,
        staged_summary: &str,
        remaining_summary: &str,
        status: Option<&str>,
        last_commissioned: Option<&str>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        const FLEET_ROW: usize = 2;
        const LOCATION_ROW: usize = 4;
        const ORDERS_ROW: usize = 5;
        const TARGET_ROW: usize = 6;
        const SPEED_ROW: usize = 7;
        const ROE_ROW: usize = 8;
        const SHIPS_ROW: usize = 10;
        const ACTION_ROW: usize = 12;
        const COMMAND_ROW: usize = 14;
        const STAGED_ROW: usize = 16;

        let mut buffer = new_playfield();
        buffer.fill_row(0, classic::menu_style());
        buffer.write_text(
            0,
            LEFT_WINDOW_PAD_COL,
            "DETACH FLEET SHIPS:",
            classic::title_style(),
        );
        draw_status_line(
            &mut buffer,
            FLEET_ROW,
            "Fleet: ",
            &format!("Fleet #{}", donor_row.fleet_number),
        );
        draw_status_line(
            &mut buffer,
            LOCATION_ROW,
            "Location: ",
            &format_sector_coords_table(donor_row.coords),
        );
        draw_status_line(&mut buffer, ORDERS_ROW, "Orders: ", &donor_row.order_label);
        draw_status_line(
            &mut buffer,
            TARGET_ROW,
            "Target: ",
            &fleet_list_target_label(donor_row.target_coords),
        );
        draw_status_line(
            &mut buffer,
            SPEED_ROW,
            "Speed: ",
            &donor_row.current_speed.to_string(),
        );
        draw_status_line(
            &mut buffer,
            ROE_ROW,
            "ROE: ",
            &donor_row.rules_of_engagement.to_string(),
        );
        draw_status_line(
            &mut buffer,
            SHIPS_ROW,
            "Ships: ",
            &donor_row.table_ships_label,
        );
        buffer.write_spans(
            ACTION_ROW,
            0,
            &[
                StyledSpan::new("<", classic::prompt_style()),
                StyledSpan::new("C", classic::prompt_hotkey_style()),
                StyledSpan::new(">ommission, <", classic::prompt_style()),
                StyledSpan::new("X", classic::prompt_hotkey_style()),
                StyledSpan::new("> Cancel", classic::prompt_style()),
            ],
        );
        draw_command_line_default_input_padded(
            &mut buffer,
            COMMAND_ROW,
            COMMAND_LABEL,
            prompt,
            default,
            input,
        );
        let staged_rows = draw_wrapped_message(
            &mut buffer,
            STAGED_ROW,
            last_body_row().saturating_sub(STAGED_ROW) + 1,
            "Staged for New Fleet: ",
            staged_summary,
        );
        let staged_end_row = if staged_summary != "none" {
            let remaining_row = STAGED_ROW + staged_rows;
            let remaining_rows = draw_wrapped_message(
                &mut buffer,
                remaining_row,
                last_body_row().saturating_sub(remaining_row) + 1,
                "Remaining on Donor: ",
                remaining_summary,
            );
            remaining_row + remaining_rows.saturating_sub(1)
        } else {
            STAGED_ROW + staged_rows.saturating_sub(1)
        };
        if let Some(status) = status {
            let status_row = (staged_end_row + 2).min(last_body_row().saturating_sub(1));
            let max_rows = last_body_row().saturating_sub(status_row);
            if max_rows > 0 {
                draw_wrapped_message(&mut buffer, status_row, max_rows, "", status);
            }
        }
        if let Some(last_commissioned) = last_commissioned {
            draw_wrapped_message(&mut buffer, last_body_row(), 1, "", last_commissioned);
        }
        Ok(buffer)
    }
}

fn max_fleet_number(rows: &[FleetRow]) -> u16 {
    rows.iter().map(|row| row.fleet_number).max().unwrap_or(1)
}

fn format_selected_fleet_numbers(
    rows: &[FleetRow],
    selected_fleet_record_indexes: &BTreeSet<usize>,
) -> String {
    let mut fleet_numbers = rows
        .iter()
        .filter(|row| selected_fleet_record_indexes.contains(&row.fleet_record_index_1_based))
        .map(|row| row.fleet_number)
        .collect::<Vec<_>>();
    fleet_numbers.sort_unstable();
    if fleet_numbers.is_empty() {
        return "0".to_string();
    }
    fleet_numbers
        .into_iter()
        .map(|fleet_number| format!("{fleet_number:02}"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn full_columns(max_fleet_number: u16) -> [TableColumn<'static>; 10] {
    let id_width = fleet_id_column_width(max_fleet_number);
    let ships_width = 67usize.saturating_sub(id_width + 3 + 8 + 15 + 8 + 3 + 4 + 3 + 3);
    [
        TableColumn::right("ID", id_width),
        TableColumn::center("Sel", 3),
        TableColumn::left("Location", 8),
        TableColumn::left("Order", 15),
        TableColumn::left("Target", 8),
        TableColumn::right("Spd", 3),
        TableColumn::right("ETA", 4),
        TableColumn::right("ROE", 3),
        TableColumn::right("ARs", 3),
        TableColumn::left("Ships", ships_width),
    ]
}

fn group_selection_columns(max_fleet_number: u16) -> [TableColumn<'static>; 10] {
    let id_width = fleet_id_column_width(max_fleet_number);
    let ships_width = 69usize.saturating_sub(id_width + 3 + 8 + 15 + 8 + 3 + 4 + 3 + 3);
    [
        TableColumn::right("ID", id_width),
        TableColumn::center("Sel", 3),
        TableColumn::left("Location", 8),
        TableColumn::left("Order", 15),
        TableColumn::left("Target", 8),
        TableColumn::right("Spd", 3),
        TableColumn::right("ETA", 4),
        TableColumn::right("ROE", 3),
        TableColumn::right("ARs", 3),
        TableColumn::left("Ships", ships_width),
    ]
}

pub(crate) fn fleet_order_label(order_code: u8) -> &'static str {
    match nc_data::Order::from_code(order_code) {
        nc_data::Order::HoldPosition => "Hold",
        nc_data::Order::MoveOnly => "Move",
        nc_data::Order::SeekHome => "Seek home",
        nc_data::Order::PatrolSector => "Patrol",
        nc_data::Order::GuardStarbase => "Guard starbase",
        nc_data::Order::GuardBlockadeWorld => "Guard/blockade",
        nc_data::Order::BombardWorld => "Bombard",
        nc_data::Order::InvadeWorld => "Invade",
        nc_data::Order::BlitzWorld => "Blitz",
        nc_data::Order::ViewWorld => "View",
        nc_data::Order::ScoutSector => "Scout sector",
        nc_data::Order::ScoutSolarSystem => "Scout system",
        nc_data::Order::ColonizeWorld => "Colonize",
        nc_data::Order::JoinAnotherFleet => "Join fleet",
        nc_data::Order::RendezvousSector => "Rendezvous",
        nc_data::Order::Salvage => "Salvage",
        nc_data::Order::Unknown(_) => "Unknown",
    }
}

pub(crate) fn fleet_table_order_label(order_code: u8) -> &'static str {
    match nc_data::Order::from_code(order_code) {
        nc_data::Order::HoldPosition => "Hold",
        nc_data::Order::MoveOnly => "Move",
        nc_data::Order::SeekHome => "Seek home",
        nc_data::Order::PatrolSector => "Patrol",
        nc_data::Order::GuardStarbase => "Guard SB",
        nc_data::Order::GuardBlockadeWorld => "Guard/Blkd",
        nc_data::Order::BombardWorld => "Bombard",
        nc_data::Order::InvadeWorld => "Invade",
        nc_data::Order::BlitzWorld => "Blitz",
        nc_data::Order::ViewWorld => "View",
        nc_data::Order::ScoutSector => "Scout sect",
        nc_data::Order::ScoutSolarSystem => "Scout sys",
        nc_data::Order::ColonizeWorld => "Colonize",
        nc_data::Order::JoinAnotherFleet => "Join fleet",
        nc_data::Order::RendezvousSector => "Rendezvous",
        nc_data::Order::Salvage => "Salvage",
        nc_data::Order::Unknown(_) => "Unknown",
    }
}

fn fleet_row_target_label(row: &FleetRow) -> String {
    if let Some(host_num) = row.join_host_fleet_number {
        format!("Flt #{host_num}")
    } else {
        fleet_list_target_label(row.target_coords)
    }
}

fn fleet_list_target_label(target_coords: [u8; 2]) -> String {
    if target_coords[0] == 0 || target_coords[1] == 0 {
        String::new()
    } else {
        format_sector_coords_table(target_coords)
    }
}

/// The confirm prompt of an order, or ECGAME's Seek Home question in its
/// place.
fn draw_order_confirm_prompt(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    seek_next_closest: bool,
    input: &str,
) {
    if seek_next_closest {
        draw_command_line_default_input_padded(
            buffer,
            row,
            COMMAND_LABEL,
            &format!("{} ", nc_engine::SEEK_NEXT_CLOSEST_PROMPT),
            "Y",
            input,
        );
    } else {
        draw_confirm_prompt_at(buffer, row, COMMAND_LABEL, input);
    }
}

fn draw_confirm_prompt_at(buffer: &mut PlayfieldBuffer, row: usize, label: &str, input: &str) {
    buffer.fill_row(row, classic::prompt_style());
    let prefix = buffer.write_spans(
        row,
        0,
        &[
            StyledSpan::new(label, classic::title_style()),
            StyledSpan::new(" <- Confirm ", classic::prompt_style()),
            StyledSpan::new("[", classic::prompt_square_delimiter_style()),
            StyledSpan::new("Y", classic::prompt_hotkey_style()),
            StyledSpan::new("]", classic::prompt_square_delimiter_style()),
            StyledSpan::new("/N ", classic::prompt_style()),
            StyledSpan::new("<", classic::prompt_angle_delimiter_style()),
            StyledSpan::new("Q", classic::prompt_hotkey_style()),
            StyledSpan::new(">", classic::prompt_angle_delimiter_style()),
            StyledSpan::new(" -> ", classic::prompt_style()),
        ],
    );
    let written = buffer.write_text(row, prefix, input, classic::prompt_hotkey_style());
    buffer.set_cursor((prefix + written) as u16, row as u16);
}
