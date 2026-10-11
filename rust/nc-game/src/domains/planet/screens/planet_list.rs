use crossterm::event::{KeyCode, KeyEvent};
use nc_data::{EmpirePlanetEconomyRow, STARDOCK_SLOT_COUNT};
use nc_engine::planet_build_view;
use nc_ui::table_filter::{TableFilterClause, is_filter_column_char};

use crate::app::Action;
use crate::app::helpers::is_coordinate_input_char;
use crate::domains::planet::PlanetAction;
use crate::screen::layout::{
    dismiss_prompt_row_for, draw_dismiss_prompt_padded, draw_status_line, draw_title_bar_padded,
    new_playfield_for, stacked_table_visible_rows_for,
};
use crate::screen::table::{
    HorizontalAlign, LayoutRect, TableColumn, TableFooter, TableWidthMode, VerticalAlign,
    draw_table_footer, draw_table_title, layout_stacked_table_block,
    resolve_table_columns_for_widget_with_footer_floor, table_footer_scaffold_width,
    write_stacked_table_window_with_states_at,
};
use crate::screen::{
    PlayfieldBuffer, ScreenFrame, SortDirection, build_quantity_from_points,
    format_sector_coords_default, format_sector_coords_table,
};
use crate::theme::classic;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanetListMode {
    Brief,
    BuildSelect,
    Stub(&'static str),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanetListSort {
    Location,
    PlanetName,
    PotentialProduction,
    CurrentProduction,
    Treasury,
    Budget,
    Revenue,
    Growth,
    BuildQueue,
    Stardock,
    Starbase,
    Armies,
    Batteries,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanetListFilterMode {
    All,
    Range,
    Starbase,
    Stardock,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanetListFilter {
    All,
    Range { anchor: [u8; 2], radius: u8 },
    Starbase,
    Stardock,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanetListFilterPromptMode {
    FilterMenu,
    ValueInput,
}

pub struct PlanetListScreen;

struct RenderedPlanetList {
    buffer: PlayfieldBuffer,
}

const BRIEF_COLUMNS: [TableColumn<'static>; 13] = [
    TableColumn::left("(XX,YY)", 7),
    TableColumn::left("Planet Name", 11),
    TableColumn::right("Prod", 4),
    TableColumn::right("Prod", 4),
    TableColumn::right("Points", 6),
    TableColumn::right("Bdgt", 5),
    TableColumn::right("Rev", 3),
    TableColumn::right("Grow", 4),
    TableColumn::right("Queue", 6),
    TableColumn::right("Dock", 6),
    TableColumn::right("SBs", 3),
    TableColumn::right("ARs", 3),
    TableColumn::right("GBs", 3),
];

const BRIEF_TOP_HEADER_CELLS: [&str; 13] = [
    "Coord", "", "Max", "Curr", "Trsry", "", "", "", "Build", "Star", "", "", "",
];

const BRIEF_BROWSE_HOTKEYS: &str = "? F S B D A C M L U X <Q>";
const BRIEF_FILTER_HOTKEYS: &str = "? A R S T <Q>";
pub(crate) const PLANET_LIST_MASS_COMMISSION_PROMPT: &str =
    "Commission all ships and starbases? [Y]/N -> ";
pub(crate) const PLANET_LIST_LOAD_FLEET_PROMPT: &str = "Load Armies: Fleet # ";
pub(crate) const PLANET_LIST_UNLOAD_FLEET_PROMPT: &str = "Unload Armies: Fleet # ";
pub(crate) const PLANET_LIST_LOAD_QTY_PROMPT: &str = "Load Armies: How many armies? ";
pub(crate) const PLANET_LIST_UNLOAD_QTY_PROMPT: &str = "Unload Armies: How many armies? ";
pub(crate) const PLANET_LIST_ABORT_BUILD_PROMPT: &str = "Abort queued builds? Y/[N] -> ";
pub(crate) const PLANET_LIST_SCORCH_CONFIRM_PROMPT: &str = "Scorch Planet: Are you sure? Y/[N] -> ";
pub(crate) const PLANET_LIST_SCORCH_REALLY_CONFIRM_PROMPT: &str =
    "Scorch Planet: Are you really sure? Y/[N] -> ";
pub(crate) const PLANET_LIST_SCORCH_LAST_CONFIRM_PROMPT: &str =
    "Scorch Planet: Are you sure-sure? Y/[N] -> ";
pub(crate) const PLANET_LIST_SCORCH_RESCIND_PROMPT: &str =
    "Rescind Scorch-Earth: Are you sure? Y/[N] -> ";
const PLANET_LIST_MAX_QTY_DEFAULT: &str = "255";

fn filter_prompt_dismiss_prompt(message: &str) -> String {
    format!("{message} (slap a key)")
}

fn planet_list_title(
    mode: PlanetListMode,
    sort: PlanetListSort,
    direction: SortDirection,
    filter: PlanetListFilter,
    filter_clause: Option<&TableFilterClause>,
) -> String {
    match mode {
        PlanetListMode::Brief => {
            format!(
                "PLANET LIST: {} {} {}",
                planet_list_sort_label(sort),
                direction.title_label(),
                filter_clause
                    .map(|clause| clause.summary.as_str())
                    .unwrap_or(filter_label(filter))
            )
        }
        PlanetListMode::BuildSelect => "CHANGE CURRENT PLANET:".to_string(),
        PlanetListMode::Stub(_) => "PLANET COMMAND:".to_string(),
    }
}

const fn planet_list_sort_label(sort: PlanetListSort) -> &'static str {
    match sort {
        PlanetListSort::Location => "COO",
        PlanetListSort::PlanetName => "PLA",
        PlanetListSort::PotentialProduction => "MAX",
        PlanetListSort::CurrentProduction => "CUR",
        PlanetListSort::Treasury => "TRS",
        PlanetListSort::Budget => "BDG",
        PlanetListSort::Revenue => "REV",
        PlanetListSort::Growth => "GRO",
        PlanetListSort::BuildQueue => "BUI",
        PlanetListSort::Stardock => "STA",
        PlanetListSort::Starbase => "SBS",
        PlanetListSort::Armies => "ARS",
        PlanetListSort::Batteries => "GBS",
    }
}

fn sort_footer_label(direction: SortDirection) -> String {
    format!("SORT {}", direction.label())
}

fn filter_label(filter: PlanetListFilter) -> &'static str {
    match filter {
        PlanetListFilter::All => "ALL",
        PlanetListFilter::Range { .. } => "RNG",
        PlanetListFilter::Starbase => "SB",
        PlanetListFilter::Stardock => "DOCK",
    }
}

impl PlanetListScreen {
    pub fn new() -> Self {
        Self
    }

    fn render_table(
        &mut self,
        frame: &ScreenFrame<'_>,
        mode: PlanetListMode,
        rows: &[EmpirePlanetEconomyRow],
        sort: PlanetListSort,
        direction: SortDirection,
        filter: PlanetListFilter,
        filter_clause: Option<&TableFilterClause>,
        scroll_offset: usize,
        cursor: usize,
        footer: TableFooter<'_>,
    ) -> Result<RenderedPlanetList, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield_for(frame.geometry);
        let visible_rows = stacked_table_visible_rows_for(frame.geometry, 1);
        let table_rows = planet_table_rows(frame, rows);
        let scrollable = table_rows.len() > visible_rows;
        let displayed_rows = table_rows
            .len()
            .saturating_sub(scroll_offset)
            .min(visible_rows);
        let title = planet_list_title(mode, sort, direction, filter, filter_clause);
        let columns = resolve_table_columns_for_widget_with_footer_floor(
            &BRIEF_COLUMNS,
            &table_rows,
            buffer.width(),
            scrollable,
            TableWidthMode::Compact,
            Some(&title),
            Some(footer),
            planet_list_footer_floor(frame, mode),
        );
        let layout = layout_stacked_table_block(
            LayoutRect::new(0, 0, buffer.width(), buffer.height()),
            &columns,
            displayed_rows,
            Some(&title),
            Some(footer),
            scrollable,
            HorizontalAlign::Center,
            VerticalAlign::Top,
        );
        let _ = layout.title_row;
        draw_table_title(&mut buffer, layout.table_row, layout.table_col, &title);

        let metrics = write_stacked_table_window_with_states_at(
            &mut buffer,
            layout.table_row,
            layout.table_col,
            &BRIEF_TOP_HEADER_CELLS,
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
            frame.geometry,
            layout.command_col,
            metrics.bottom_row,
            footer,
        );
        Ok(RenderedPlanetList { buffer })
    }

    pub fn render_sort_prompt(
        &mut self,
        frame: &ScreenFrame<'_>,
        mode: PlanetListMode,
        rows: &[EmpirePlanetEconomyRow],
        sort: PlanetListSort,
        direction: SortDirection,
        filter: PlanetListFilter,
        scroll_offset: usize,
        cursor: usize,
        prompt_default: &str,
        input: &str,
        status: Option<&str>,
        dismiss_message: Option<&str>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        if let PlanetListMode::Stub(message) = mode {
            let mut buffer = new_playfield_for(frame.geometry);
            draw_title_bar_padded(&mut buffer, 0, "PLANET COMMAND:");
            draw_status_line(&mut buffer, 3, "Notice: ", message);
            draw_dismiss_prompt_padded(&mut buffer, dismiss_prompt_row_for(frame.geometry, 3));
            return Ok(buffer);
        }

        let footer_label = sort_footer_label(direction);
        let prompt;
        let footer = if let Some(message) = dismiss_message {
            prompt = filter_prompt_dismiss_prompt(message);
            TableFooter::CommandPrompt {
                label: &footer_label,
                prompt: &prompt,
            }
        } else {
            prompt = status
                .filter(|value| value.trim_start().starts_with("Ambiguous:"))
                .map(|value| value.trim_start().to_string())
                .unwrap_or_else(|| "Sort column [?] ".to_string());
            TableFooter::CommandInput {
                label: &footer_label,
                prompt: &prompt,
                default: prompt_default,
                input,
            }
        };
        Ok(self
            .render_table(
                frame,
                mode,
                rows,
                sort,
                direction,
                filter,
                None,
                scroll_offset,
                cursor,
                footer,
            )?
            .buffer)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render_filter_prompt(
        &mut self,
        frame: &ScreenFrame<'_>,
        mode: PlanetListMode,
        rows: &[EmpirePlanetEconomyRow],
        sort: PlanetListSort,
        direction: SortDirection,
        filter: PlanetListFilter,
        scroll_offset: usize,
        cursor: usize,
        prompt_mode: PlanetListFilterPromptMode,
        prompt_default: &str,
        input: &str,
        status: Option<&str>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        self.render_filter_prompt_with_filter_clause(
            frame,
            mode,
            rows,
            sort,
            direction,
            filter,
            None,
            scroll_offset,
            cursor,
            prompt_mode,
            prompt_default,
            input,
            status,
            None,
            None,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render_filter_prompt_with_filter_clause(
        &mut self,
        frame: &ScreenFrame<'_>,
        mode: PlanetListMode,
        rows: &[EmpirePlanetEconomyRow],
        sort: PlanetListSort,
        direction: SortDirection,
        filter: PlanetListFilter,
        filter_clause: Option<&TableFilterClause>,
        scroll_offset: usize,
        cursor: usize,
        prompt_mode: PlanetListFilterPromptMode,
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
                PlanetListFilterPromptMode::FilterMenu => status
                    .filter(|value| value.trim_start().starts_with("Ambiguous:"))
                    .map(|value| value.trim_start().to_string())
                    .unwrap_or_else(|| "Filter column [?] ".to_string()),
                PlanetListFilterPromptMode::ValueInput => {
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
                frame,
                mode,
                rows,
                sort,
                direction,
                filter,
                filter_clause,
                scroll_offset,
                cursor,
                footer,
            )?
            .buffer)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render_brief_list(
        &mut self,
        frame: &ScreenFrame<'_>,
        mode: PlanetListMode,
        rows: &[EmpirePlanetEconomyRow],
        sort: PlanetListSort,
        direction: SortDirection,
        filter: PlanetListFilter,
        scroll_offset: usize,
        cursor: usize,
        input: &str,
        status: Option<&str>,
        auto_commission_prompt: bool,
        build_abort_prompt: bool,
        transport_prompt_label: Option<&str>,
        transport_prompt_default: &str,
        transport_prompt_input: &str,
        transport_summary: Option<&str>,
        scorch_prompt_label: Option<&str>,
        scorch_summary: Option<&str>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        self.render_brief_list_with_filter_clause(
            frame,
            mode,
            rows,
            sort,
            direction,
            filter,
            None,
            scroll_offset,
            cursor,
            input,
            status,
            auto_commission_prompt,
            build_abort_prompt,
            transport_prompt_label,
            transport_prompt_default,
            transport_prompt_input,
            transport_summary,
            scorch_prompt_label,
            scorch_summary,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render_brief_list_with_filter_clause(
        &mut self,
        frame: &ScreenFrame<'_>,
        mode: PlanetListMode,
        rows: &[EmpirePlanetEconomyRow],
        sort: PlanetListSort,
        direction: SortDirection,
        filter: PlanetListFilter,
        filter_clause: Option<&TableFilterClause>,
        scroll_offset: usize,
        cursor: usize,
        input: &str,
        status: Option<&str>,
        auto_commission_prompt: bool,
        build_abort_prompt: bool,
        transport_prompt_label: Option<&str>,
        transport_prompt_default: &str,
        transport_prompt_input: &str,
        _transport_summary: Option<&str>,
        scorch_prompt_label: Option<&str>,
        _scorch_summary: Option<&str>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let default_coords = rows
            .get(cursor)
            .map(|row| format_sector_coords_default(row.coords))
            .unwrap_or_else(|| "00,00".to_string());
        let footer = if rows.is_empty() {
            TableFooter::CommandText {
                label: "COMMAND",
                text: if filter_clause.is_some() {
                    "No planets match current filter."
                } else {
                    "You do not currently control any planets."
                },
            }
        } else if auto_commission_prompt {
            TableFooter::CommandPrompt {
                label: "COMMAND",
                prompt: PLANET_LIST_MASS_COMMISSION_PROMPT,
            }
        } else if build_abort_prompt {
            TableFooter::CommandPrompt {
                label: "COMMAND",
                prompt: PLANET_LIST_ABORT_BUILD_PROMPT,
            }
        } else if let Some(prompt) = transport_prompt_label {
            TableFooter::CommandInput {
                label: "COMMAND",
                prompt,
                default: transport_prompt_default,
                input: transport_prompt_input,
            }
        } else if let Some(prompt) = scorch_prompt_label {
            TableFooter::CommandPrompt {
                label: "COMMAND",
                prompt,
            }
        } else if let Some(status) = status {
            TableFooter::CommandText {
                label: "COMMAND",
                text: status,
            }
        } else {
            TableFooter::CommandBar {
                hotkeys_markup: match mode {
                    PlanetListMode::Brief => BRIEF_BROWSE_HOTKEYS,
                    PlanetListMode::BuildSelect | PlanetListMode::Stub(_) => "? S <Q>",
                },
                default: Some(&default_coords),
                input,
            }
        };
        Ok(self
            .render_table(
                frame,
                mode,
                rows,
                sort,
                direction,
                filter,
                filter_clause,
                scroll_offset,
                cursor,
                footer,
            )?
            .buffer)
    }

    pub fn handle_sort_prompt_key(&self, key: KeyEvent, mode: PlanetListMode) -> Action {
        match key.code {
            KeyCode::Char('?') => Action::OpenPopupHelp,
            KeyCode::Enter => Action::Planet(PlanetAction::SubmitListSortPrompt(mode)),
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                Action::Planet(PlanetAction::CloseListSortPrompt(mode))
            }
            KeyCode::Backspace => Action::Planet(PlanetAction::BackspaceListPromptInput),
            KeyCode::Char(ch) if is_filter_column_char(ch) => {
                Action::Planet(PlanetAction::AppendListPromptChar(ch))
            }
            _ => Action::Noop,
        }
    }

    pub fn handle_filter_prompt_key(
        &self,
        key: KeyEvent,
        mode: PlanetListMode,
        prompt_mode: PlanetListFilterPromptMode,
    ) -> Action {
        match prompt_mode {
            PlanetListFilterPromptMode::FilterMenu => match key.code {
                KeyCode::Char('?') => Action::OpenPopupHelp,
                KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                    Action::Planet(PlanetAction::CloseListFilterPrompt(mode))
                }
                KeyCode::Enter => Action::Planet(PlanetAction::SubmitListFilterPrompt(mode)),
                KeyCode::Backspace => Action::Planet(PlanetAction::BackspaceListPromptInput),
                KeyCode::Char(ch) if is_filter_column_char(ch) => {
                    Action::Planet(PlanetAction::AppendListPromptChar(ch))
                }
                _ => Action::Noop,
            },
            PlanetListFilterPromptMode::ValueInput => match key.code {
                KeyCode::Char('?') => Action::OpenPopupHelp,
                KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                    Action::Planet(PlanetAction::CloseListFilterPrompt(mode))
                }
                KeyCode::Enter => Action::Planet(PlanetAction::SubmitListFilterPrompt(mode)),
                KeyCode::Backspace => Action::Planet(PlanetAction::BackspaceListPromptInput),
                KeyCode::Char(ch)
                    if matches!(
                        ch,
                        ' ' | '-' | '#' | '*' | '/' | '?' | '=' | '!' | '>' | '<' | '+' | ','
                    ) || ch.is_ascii_alphanumeric() =>
                {
                    Action::Planet(PlanetAction::AppendListPromptChar(ch))
                }
                _ => Action::Noop,
            },
        }
    }

    pub fn handle_brief_key(&self, key: KeyEvent, mode: PlanetListMode) -> Action {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                Action::Planet(PlanetAction::MoveBrief(-1))
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                Action::Planet(PlanetAction::MoveBrief(1))
            }
            KeyCode::PageUp => Action::Planet(PlanetAction::MoveBrief(-5)),
            KeyCode::PageDown => Action::Planet(PlanetAction::MoveBrief(5)),
            KeyCode::Char('f') | KeyCode::Char('F') if mode == PlanetListMode::Brief => {
                Action::Planet(PlanetAction::OpenListFilterPrompt(mode))
            }
            KeyCode::Char('s') | KeyCode::Char('S') => {
                Action::Planet(PlanetAction::OpenListSortPrompt(mode))
            }
            KeyCode::Char('b') | KeyCode::Char('B') if mode == PlanetListMode::Brief => {
                Action::Planet(PlanetAction::OpenBuildSpecify)
            }
            KeyCode::Char('a') | KeyCode::Char('A') if mode == PlanetListMode::Brief => {
                Action::Planet(PlanetAction::OpenBuildAbortPrompt)
            }
            KeyCode::Char('d') | KeyCode::Char('D') if mode == PlanetListMode::Brief => {
                Action::Planet(PlanetAction::OpenBuildList)
            }
            KeyCode::Char('m') | KeyCode::Char('M') if mode == PlanetListMode::Brief => {
                Action::Planet(PlanetAction::OpenAutoCommissionPrompt)
            }
            KeyCode::Char('c') | KeyCode::Char('C') if mode == PlanetListMode::Brief => {
                Action::Planet(PlanetAction::OpenCommissionMenu)
            }
            KeyCode::Char('l') | KeyCode::Char('L') if mode == PlanetListMode::Brief => {
                Action::Planet(PlanetAction::OpenTransportPrompt(
                    crate::screen::PlanetTransportMode::Load,
                ))
            }
            KeyCode::Char('u') | KeyCode::Char('U') if mode == PlanetListMode::Brief => {
                Action::Planet(PlanetAction::OpenTransportPrompt(
                    crate::screen::PlanetTransportMode::Unload,
                ))
            }
            KeyCode::Char('x') | KeyCode::Char('X') if mode == PlanetListMode::Brief => {
                Action::Planet(PlanetAction::OpenScorchPrompt)
            }
            KeyCode::Char('i') | KeyCode::Char('I') if mode == PlanetListMode::Brief => {
                Action::Planet(PlanetAction::SubmitBriefInput)
            }
            KeyCode::Enter => Action::Planet(PlanetAction::SubmitBriefInput),
            KeyCode::Backspace => Action::Planet(PlanetAction::BackspaceBriefInput),
            KeyCode::Char(ch) if is_coordinate_input_char(ch) => {
                Action::Planet(PlanetAction::AppendBriefChar(ch))
            }
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => match mode {
                PlanetListMode::Brief => Action::Planet(PlanetAction::OpenMenu),
                PlanetListMode::BuildSelect => Action::Planet(PlanetAction::OpenBuildMenu),
                PlanetListMode::Stub(_) => Action::Planet(PlanetAction::OpenMenu),
            },
            _ => Action::Noop,
        }
    }
}

fn planet_list_footer_floor(frame: &ScreenFrame<'_>, mode: PlanetListMode) -> usize {
    match mode {
        PlanetListMode::Brief => {
            let max_fleet_default = frame
                .game_data
                .fleets
                .iter()
                .filter(|fleet| fleet.owner.0 as usize == frame.player.record_index_1_based)
                .map(|fleet| fleet.fleet_number.0)
                .max()
                .unwrap_or(1)
                .to_string();
            [
                table_footer_scaffold_width(TableFooter::CommandBar {
                    hotkeys_markup: BRIEF_BROWSE_HOTKEYS,
                    default: Some("00,00"),
                    input: "",
                }),
                table_footer_scaffold_width(TableFooter::CommandPrompt {
                    label: "COMMAND",
                    prompt: PLANET_LIST_MASS_COMMISSION_PROMPT,
                }),
                table_footer_scaffold_width(TableFooter::CommandPrompt {
                    label: "COMMAND",
                    prompt: PLANET_LIST_ABORT_BUILD_PROMPT,
                }),
                table_footer_scaffold_width(TableFooter::CommandInput {
                    label: "COMMAND",
                    prompt: PLANET_LIST_LOAD_FLEET_PROMPT,
                    default: &max_fleet_default,
                    input: "",
                }),
                table_footer_scaffold_width(TableFooter::CommandInput {
                    label: "COMMAND",
                    prompt: PLANET_LIST_UNLOAD_FLEET_PROMPT,
                    default: &max_fleet_default,
                    input: "",
                }),
                table_footer_scaffold_width(TableFooter::CommandInput {
                    label: "COMMAND",
                    prompt: PLANET_LIST_LOAD_QTY_PROMPT,
                    default: PLANET_LIST_MAX_QTY_DEFAULT,
                    input: "",
                }),
                table_footer_scaffold_width(TableFooter::CommandInput {
                    label: "COMMAND",
                    prompt: PLANET_LIST_UNLOAD_QTY_PROMPT,
                    default: PLANET_LIST_MAX_QTY_DEFAULT,
                    input: "",
                }),
                table_footer_scaffold_width(TableFooter::CommandPrompt {
                    label: "COMMAND",
                    prompt: PLANET_LIST_SCORCH_CONFIRM_PROMPT,
                }),
                table_footer_scaffold_width(TableFooter::CommandPrompt {
                    label: "COMMAND",
                    prompt: PLANET_LIST_SCORCH_REALLY_CONFIRM_PROMPT,
                }),
                table_footer_scaffold_width(TableFooter::CommandPrompt {
                    label: "COMMAND",
                    prompt: PLANET_LIST_SCORCH_LAST_CONFIRM_PROMPT,
                }),
                table_footer_scaffold_width(TableFooter::CommandPrompt {
                    label: "COMMAND",
                    prompt: PLANET_LIST_SCORCH_RESCIND_PROMPT,
                }),
                table_footer_scaffold_width(TableFooter::CommandInput {
                    label: "SORT",
                    prompt: "Sort column [?] ",
                    default: "gbs",
                    input: "",
                }),
                table_footer_scaffold_width(TableFooter::LabeledCommandBar {
                    label: "FILTER",
                    hotkeys_markup: BRIEF_FILTER_HOTKEYS,
                    default: None,
                    input: "",
                }),
            ]
            .into_iter()
            .max()
            .unwrap_or(0)
        }
        PlanetListMode::BuildSelect | PlanetListMode::Stub(_) => [
            table_footer_scaffold_width(TableFooter::CommandBar {
                hotkeys_markup: "? S <Q>",
                default: Some("00,00"),
                input: "",
            }),
            table_footer_scaffold_width(TableFooter::CommandInput {
                label: "SORT",
                prompt: "Sort column [?] ",
                default: "gbs",
                input: "",
            }),
        ]
        .into_iter()
        .max()
        .unwrap_or(0),
    }
}

fn planet_table_rows(frame: &ScreenFrame<'_>, rows: &[EmpirePlanetEconomyRow]) -> Vec<Vec<String>> {
    rows.iter()
        .map(|row| {
            let (treasury, budget) = effective_points_left(frame, row);
            vec![
                format_sector_coords_table(row.coords),
                row.planet_name.clone(),
                row.potential_production.to_string(),
                row.present_production.to_string(),
                treasury.to_string(),
                budget.to_string(),
                row.yearly_tax_revenue.to_string(),
                format_signed_growth(row.yearly_growth_delta),
                queued_build_units(frame, row).to_string(),
                docked_units(frame, row).to_string(),
                if row.has_friendly_starbase {
                    "1".to_string()
                } else {
                    "0".to_string()
                },
                row.armies.to_string(),
                row.ground_batteries.to_string(),
            ]
        })
        .collect()
}

fn effective_points_left(frame: &ScreenFrame<'_>, row: &EmpirePlanetEconomyRow) -> (u32, u32) {
    planet_build_view(frame.game_data, row)
        .map(|view| (view.treasury_left, view.points_left))
        .unwrap_or_else(|_| {
            (
                row.stored_production_points,
                u32::from(row.build_capacity).min(row.stored_production_points),
            )
        })
}

fn format_signed_growth(growth: u16) -> String {
    format!("+{growth}")
}

fn docked_units(frame: &ScreenFrame<'_>, row: &EmpirePlanetEconomyRow) -> u32 {
    frame
        .game_data
        .planets
        .get(row.planet_record_index_1_based.saturating_sub(1))
        .map(|planet| {
            (0..STARDOCK_SLOT_COUNT)
                .map(|slot| u32::from(planet.stardock[slot].min(u32::from(u16::MAX)) as u16))
                .sum()
        })
        .unwrap_or(0)
}

fn queued_build_units(frame: &ScreenFrame<'_>, row: &EmpirePlanetEconomyRow) -> u32 {
    frame
        .game_data
        .planets
        .get(row.planet_record_index_1_based.saturating_sub(1))
        .map(|planet| {
            (0..10)
                .map(|slot| {
                    build_quantity_from_points(
                        nc_data::ProductionItemKind::from_code(
                            planet.build_queue[slot].kind.code(),
                        ),
                        planet.build_queue[slot].points_owed,
                    )
                })
                .sum()
        })
        .unwrap_or(0)
}
