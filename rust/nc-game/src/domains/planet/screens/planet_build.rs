use crossterm::event::{KeyCode, KeyEvent};
use nc_data::{EmpirePlanetEconomyRow, ProductionItemKind};
use nc_engine::{
    BuildUnitSpec, build_kind_count_label, build_kind_name, build_unit_spec_by_kind,
    planet_build_max_selectable_unit_number, planet_build_specify_entries,
};

use crate::app::Action;
use crate::domains::planet::PlanetAction;
use crate::domains::starmap::StarmapAction;
use crate::screen::layout::{
    CMD_COL_1, EXPERT_MENU_PROMPT_ROW, LEFT_WINDOW_PAD_COL, MenuEntry, ScreenGeometry,
    centered_row, dismiss_prompt_row, draw_command_line_default_input_padded,
    draw_command_prompt_padded, draw_dismiss_prompt_padded, draw_expert_menu_padded,
    draw_inline_planet_info_prompt_padded, draw_menu_notice_padded, draw_menu_row,
    draw_title_bar_padded, last_body_row, menu_prompt_row, new_playfield, new_playfield_for,
    standard_table_visible_rows_for,
};
use crate::screen::table::{
    SplitTableRow, TABLE_TEXT_INSET, TableColumn, TableFooter, centered_table_start_col,
    draw_table_footer, draw_table_title, fit_table_columns_for_widget, table_render_width,
    write_split_table_at, write_table_window_with_cursor,
};
use crate::screen::{
    COMMAND_LABEL, CommandMenu, PlayfieldBuffer, Screen, ScreenFrame, format_sector_coords,
    format_sector_coords_table,
};
use crate::theme::classic;

pub struct PlanetBuildScreen;

pub fn planet_build_list_visible_rows(geometry: ScreenGeometry) -> usize {
    standard_table_visible_rows_for(geometry, 1)
}

pub fn planet_build_change_visible_rows(geometry: ScreenGeometry) -> usize {
    standard_table_visible_rows_for(geometry, 1)
}

const CHANGE_COLUMNS: [TableColumn<'static>; 5] = [
    TableColumn::left("Planet Name", 20),
    TableColumn::left("Location", 9),
    TableColumn::left("Production", 16),
    TableColumn::right("PP", 4),
    TableColumn::right("Spent", 5),
];

const BUILD_LIST_COLUMNS: [TableColumn<'static>; 3] = [
    TableColumn::left("Unit", 24),
    TableColumn::right("Points", 6),
    TableColumn::right("Queue", 5),
];

const BUILD_HALF_COLUMNS: [TableColumn<'static>; 4] = [
    TableColumn::left("NO.", 4),
    TableColumn::left("UNIT TYPE", 19),
    TableColumn::right("COST", 4),
    TableColumn::right("QTY.", 5),
];

const ROW_1: [MenuEntry<'static>; 3] = [
    MenuEntry::new(CMD_COL_1, "H", "elp with commands"),
    MenuEntry::new(29, "P", "lanets, List your"),
    MenuEntry::new(57, "S", "pecify Build Orders"),
];

const ROW_2: [MenuEntry<'static>; 3] = [
    MenuEntry::new(CMD_COL_1, "Q", "uit to Planet Menu"),
    MenuEntry::new(29, "R", "eview current planet"),
    MenuEntry::new(57, "A", "bort planet's builds"),
];

const ROW_3: [MenuEntry<'static>; 3] = [
    MenuEntry::new(CMD_COL_1, "X", "pert mode ON/OFF"),
    MenuEntry::new(29, "C", "hange current planet"),
    MenuEntry::new(57, "D", "isplay queue"),
];

const ROW_4: [MenuEntry<'static>; 3] = [
    MenuEntry::new(CMD_COL_1, "V", "iew partial star map"),
    MenuEntry::new(29, "N", "ext planet"),
    MenuEntry::new(57, "I", "nfo about a Planet"),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlanetBuildOrder {
    pub kind: ProductionItemKind,
    pub points_remaining: u32,
}

#[derive(Debug, Clone)]
pub struct PlanetBuildMenuView {
    pub row: EmpirePlanetEconomyRow,
    pub committed_points: u32,
    pub budget: u32,
    pub points_left: u32,
    pub building_count: u32,
    pub docked_count: u32,
}

#[derive(Debug, Clone)]
pub struct PlanetBuildListRow {
    pub kind: ProductionItemKind,
    pub unit_label: String,
    pub points: u32,
    pub queue_qty: u32,
}

#[derive(Debug, Clone)]
pub struct PlanetBuildChangeRow {
    pub planet_name: String,
    pub coords: [u8; 2],
    pub present_production: u16,
    pub potential_production: u16,
    pub budget: u32,
    pub committed_points: u32,
}

impl PlanetBuildScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn render_menu(
        &mut self,
        view: &PlanetBuildMenuView,
        _orders: &[PlanetBuildOrder],
        status: Option<&str>,
        expert_mode: bool,
        inline_planet_info: bool,
        info_default_coords: [u8; 2],
        info_input: &str,
        info_notice: Option<&str>,
        inline_abort_prompt: bool,
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
                    status,
                );
            } else if inline_abort_prompt {
                draw_command_prompt_padded(
                    &mut buffer,
                    EXPERT_MENU_PROMPT_ROW,
                    "COMMAND",
                    "Abort queued builds? Y/[N] -> ",
                );
            } else {
                draw_expert_menu_padded(
                    &mut buffer,
                    "BUILD COMMAND",
                    "? X V P R C N S A D I <Q>",
                    status,
                );
            }
            return Ok(buffer);
        }
        draw_title_bar_padded(
            &mut buffer,
            0,
            &format!(
                "BUILD ON CURRENT PLANET: \"{}\" IN SYSTEM {}:",
                view.row.planet_name,
                format_sector_coords(view.row.coords)
            ),
        );

        let spent = view.committed_points.min(view.budget);

        draw_menu_row(&mut buffer, 2, &ROW_1);
        draw_menu_row(&mut buffer, 3, &ROW_2);
        draw_menu_row(&mut buffer, 4, &ROW_3);
        draw_menu_row(&mut buffer, 5, &ROW_4);

        let command_row = menu_prompt_row(5);
        if inline_planet_info {
            draw_inline_planet_info_prompt_padded(
                &mut buffer,
                command_row,
                info_default_coords,
                info_input,
                info_notice,
                status,
            );
        } else if inline_abort_prompt {
            draw_command_prompt_padded(
                &mut buffer,
                command_row,
                "COMMAND",
                "Abort queued builds? Y/[N] -> ",
            );
        } else {
            let lower_block_row = centered_row(command_row + 1, last_body_row(), 5);

            let starbase_line = if view.row.has_friendly_starbase {
                format!(
                    "There is a starbase orbiting planet \"{}\".",
                    view.row.planet_name
                )
            } else {
                format!(
                    "There are no starbases orbiting planet \"{}\".",
                    view.row.planet_name
                )
            };
            let restrictions_line = if view.row.has_friendly_starbase {
                "Standard building restrictions do not apply.".to_string()
            } else {
                "Standard building restrictions apply, except to a first starbase.".to_string()
            };
            buffer.write_text(
                lower_block_row,
                LEFT_WINDOW_PAD_COL,
                &starbase_line,
                classic::status_value_style(),
            );
            buffer.write_text(
                lower_block_row + 1,
                LEFT_WINDOW_PAD_COL,
                &restrictions_line,
                classic::status_value_style(),
            );
            buffer.write_text(
                lower_block_row + 2,
                LEFT_WINDOW_PAD_COL,
                &format!(
                    "Spent: {} of {} PP.  Budget remaining: {}.",
                    spent, view.budget, view.points_left
                ),
                classic::status_value_style(),
            );
            buffer.write_text(
                lower_block_row + 4,
                LEFT_WINDOW_PAD_COL,
                &format!(
                    "Building: {}   Docked: {}",
                    view.building_count, view.docked_count,
                ),
                classic::status_value_style(),
            );
            draw_command_prompt_padded(
                &mut buffer,
                command_row,
                "BUILD COMMAND",
                "? X V P R C N S A D I <Q>",
            );
            if let Some(status) = status {
                draw_menu_notice_padded(&mut buffer, command_row, status);
            }
        }
        Ok(buffer)
    }

    pub fn render_list(
        &mut self,
        geometry: ScreenGeometry,
        view: &PlanetBuildMenuView,
        rows: &[PlanetBuildListRow],
        scroll_offset: usize,
        cursor: usize,
        confirming: bool,
        delete_qty_prompt_active: bool,
        delete_qty_input: &str,
        delete_qty_status: Option<&str>,
        pending_delete_qty: Option<u32>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield_for(geometry);
        let title = format!(
            "BUILD LIST: \"{}\" AT {}:",
            view.row.planet_name,
            format_sector_coords_table(view.row.coords)
        );
        draw_table_title(&mut buffer, 1, 0, &title);

        let table_rows: Vec<Vec<String>> = rows
            .iter()
            .map(|row| {
                vec![
                    row.unit_label.clone(),
                    row.points.to_string(),
                    row.queue_qty.to_string(),
                ]
            })
            .collect();
        let confirm_summary = rows
            .get(cursor)
            .map(|row| {
                let quantity = pending_delete_qty.unwrap_or(row.queue_qty);
                format!(
                    "Delete {} {}?",
                    quantity,
                    build_kind_count_label(row.kind, quantity)
                )
            })
            .unwrap_or_else(|| "Delete queued build(s) for this unit?".to_string());
        let confirm_prompt = format!("{confirm_summary} Y/[N] -> ");
        let delete_prompt = rows
            .get(cursor)
            .map(|row| {
                format!(
                    "Delete <A> or 1-{} <Q> -> {}",
                    row.queue_qty, delete_qty_input
                )
            })
            .unwrap_or_else(|| "? D <Q> -> ".to_string());
        let footer = if confirming {
            TableFooter::CommandPrompt {
                label: COMMAND_LABEL,
                prompt: &confirm_prompt,
            }
        } else if delete_qty_prompt_active {
            TableFooter::CommandPrompt {
                label: COMMAND_LABEL,
                prompt: &delete_prompt,
            }
        } else if rows.is_empty() {
            TableFooter::CommandText {
                label: COMMAND_LABEL,
                text: "No build orders are queued.",
            }
        } else {
            TableFooter::CommandPrompt {
                label: COMMAND_LABEL,
                prompt: "? D <Q> -> ",
            }
        };
        let columns =
            fit_table_columns_for_widget(&BUILD_LIST_COLUMNS, &table_rows, Some(&title), None);

        let selected = if rows.is_empty() { None } else { Some(cursor) };
        let metrics = write_table_window_with_cursor(
            &mut buffer,
            1,
            &columns,
            &table_rows,
            scroll_offset,
            planet_build_list_visible_rows(geometry),
            classic::status_value_style(),
            classic::status_value_style(),
            selected,
            0,
        );

        draw_table_footer(
            &mut buffer,
            geometry,
            TABLE_TEXT_INSET,
            metrics.bottom_row,
            footer,
        );
        let _ = delete_qty_status;
        Ok(buffer)
    }

    pub fn render_abort_confirm(
        &mut self,
        view: &PlanetBuildMenuView,
        orders: &[PlanetBuildOrder],
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        draw_title_bar_padded(&mut buffer, 0, "BUILD COMMAND:");
        let style = classic::status_value_style();

        buffer.write_text(
            2,
            LEFT_WINDOW_PAD_COL,
            &format!(
                "Abort all build orders for \"{}\" at {}.",
                view.row.planet_name,
                format_sector_coords(view.row.coords)
            ),
            style,
        );

        if orders.is_empty() {
            buffer.write_text(4, LEFT_WINDOW_PAD_COL, "No build orders are queued.", style);
        } else {
            buffer.write_text(
                4,
                LEFT_WINDOW_PAD_COL,
                "Queued orders to be cancelled:",
                style,
            );
            for (i, line) in build_abort_order_lines(orders).iter().enumerate() {
                buffer.write_text(5 + i, 2, &format!("- {line}"), style);
            }
        }

        buffer.write_text(
            12,
            LEFT_WINDOW_PAD_COL,
            &format!(
                "Cancelling releases {} unpaid points. Points already spent are not refunded.",
                view.committed_points
            ),
            classic::prompt_hotkey_style(),
        );

        draw_command_line_default_input_padded(
            &mut buffer,
            14,
            COMMAND_LABEL,
            "Cancel these orders? ",
            "N",
            "",
        );
        Ok(buffer)
    }

    pub fn render_specify(
        &mut self,
        view: &PlanetBuildMenuView,
        orders: &[PlanetBuildOrder],
        input: &str,
        error: Option<&str>,
        notice: Option<&str>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        let shared_orders = orders
            .iter()
            .map(|order| nc_engine::PlanetBuildOrderLine {
                kind: order.kind,
                points_remaining: order.points_remaining,
            })
            .collect::<Vec<_>>();
        let entries = planet_build_specify_entries(view.points_left, &shared_orders);
        let (table_metrics, table_col) =
            draw_specify_table(&mut buffer, view.points_left, &entries);
        let max_unit_num = planet_build_max_selectable_unit_number(&entries);
        draw_table_footer(
            &mut buffer,
            ScreenGeometry::local_default(),
            table_col + TABLE_TEXT_INSET,
            table_metrics.bottom_row,
            TableFooter::CommandInput {
                label: COMMAND_LABEL,
                prompt: &format!("Unit number or 0 if done (0 - {}) ", max_unit_num),
                default: "0",
                input,
            },
        );
        let _ = (notice, error);
        Ok(buffer)
    }

    pub fn render_quantity_prompt(
        &mut self,
        view: &PlanetBuildMenuView,
        orders: &[PlanetBuildOrder],
        unit: BuildUnitSpec,
        max_qty: u32,
        input: &str,
        status: Option<&str>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        let shared_orders = orders
            .iter()
            .map(|order| nc_engine::PlanetBuildOrderLine {
                kind: order.kind,
                points_remaining: order.points_remaining,
            })
            .collect::<Vec<_>>();
        let entries = planet_build_specify_entries(view.points_left, &shared_orders);
        let (table_metrics, table_col) =
            draw_specify_table(&mut buffer, view.points_left, &entries);

        let prompt = format!(
            "How many new {} to build (0 - {}) ",
            unit.singular_label, max_qty
        );
        let default_qty = max_qty.to_string();
        draw_table_footer(
            &mut buffer,
            ScreenGeometry::local_default(),
            table_col + TABLE_TEXT_INSET,
            table_metrics.bottom_row,
            TableFooter::CommandInput {
                label: COMMAND_LABEL,
                prompt: &prompt,
                default: &default_qty,
                input,
            },
        );
        let _ = status;
        Ok(buffer)
    }

    pub fn render_change(
        &mut self,
        geometry: ScreenGeometry,
        rows: &[PlanetBuildChangeRow],
        scroll_offset: usize,
        cursor: usize,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield_for(geometry);
        draw_table_title(&mut buffer, 1, 0, "CHANGE CURRENT PLANET:");

        let table_rows: Vec<Vec<String>> = rows
            .iter()
            .map(|row| {
                vec![
                    row.planet_name.clone(),
                    format_sector_coords_table(row.coords),
                    format!(
                        "{:>3} of {:>3}",
                        row.present_production, row.potential_production
                    ),
                    row.budget.to_string(),
                    row.committed_points.to_string(),
                ]
            })
            .collect();
        let footer = if rows.is_empty() {
            TableFooter::CommandText {
                label: COMMAND_LABEL,
                text: "No owned planets available.",
            }
        } else {
            TableFooter::CommandPrompt {
                label: COMMAND_LABEL,
                prompt: "? <Q> -> ",
            }
        };
        let columns = fit_table_columns_for_widget(
            &CHANGE_COLUMNS,
            &table_rows,
            Some("CHANGE CURRENT PLANET:"),
            Some(footer),
        );

        let selected = if rows.is_empty() { None } else { Some(cursor) };
        let metrics = write_table_window_with_cursor(
            &mut buffer,
            1,
            &columns,
            &table_rows,
            scroll_offset,
            planet_build_change_visible_rows(geometry),
            classic::status_value_style(),
            classic::status_value_style(),
            selected,
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

    pub fn handle_change_key(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                Action::Planet(PlanetAction::MoveBuildChange(-1))
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                Action::Planet(PlanetAction::MoveBuildChange(1))
            }
            KeyCode::PageUp => Action::Planet(PlanetAction::MoveBuildChange(-8)),
            KeyCode::PageDown => Action::Planet(PlanetAction::MoveBuildChange(8)),
            KeyCode::Enter => Action::Planet(PlanetAction::ConfirmBuildChange),
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                Action::Planet(PlanetAction::OpenBuildMenu)
            }
            _ => Action::Noop,
        }
    }

    pub fn handle_menu_key(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                Action::Planet(PlanetAction::OpenMenu)
            }
            KeyCode::Char('v') | KeyCode::Char('V') => {
                Action::Starmap(StarmapAction::OpenPartialView(CommandMenu::PlanetBuild))
            }
            KeyCode::Char('i') | KeyCode::Char('I') => {
                Action::Planet(PlanetAction::OpenInfoPrompt(CommandMenu::PlanetBuild))
            }
            KeyCode::Char('h') | KeyCode::Char('H') => Action::OpenPopupHelp,
            KeyCode::Char('c') | KeyCode::Char('C') => {
                Action::Planet(PlanetAction::OpenBuildChange)
            }
            KeyCode::Char('n') | KeyCode::Char('N') => Action::Planet(PlanetAction::MoveBuild(1)),
            KeyCode::Char('r') | KeyCode::Char('R') => {
                Action::Planet(PlanetAction::OpenCurrentBuildPlanetInfo)
            }
            KeyCode::Char('d') | KeyCode::Char('D') => Action::Planet(PlanetAction::OpenBuildList),
            KeyCode::Char('a') | KeyCode::Char('A') => {
                Action::Planet(PlanetAction::OpenBuildAbortPrompt)
            }
            KeyCode::Char('s') | KeyCode::Char('S') => {
                Action::Planet(PlanetAction::OpenBuildSpecify)
            }
            KeyCode::Char('p') | KeyCode::Char('P') => {
                Action::Planet(PlanetAction::SubmitListSort(
                    crate::screen::PlanetListMode::BuildSelect,
                    crate::screen::PlanetListSort::CurrentProduction,
                ))
            }
            KeyCode::Char('x') | KeyCode::Char('X') => Action::ToggleExpertMode,
            _ => Action::Noop,
        }
    }

    pub fn handle_list_key(
        &self,
        key: KeyEvent,
        confirming: bool,
        delete_qty_prompt_active: bool,
    ) -> Action {
        if confirming {
            return match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') => {
                    Action::Planet(PlanetAction::ConfirmDeleteBuildSlot)
                }
                _ => Action::Planet(PlanetAction::CancelDeleteBuildSlot),
            };
        }
        if delete_qty_prompt_active {
            return match key.code {
                KeyCode::Char('a') | KeyCode::Char('A') | KeyCode::Enter => {
                    Action::Planet(PlanetAction::SubmitDeleteBuildQty)
                }
                KeyCode::Backspace => Action::Planet(PlanetAction::BackspaceDeleteBuildQtyInput),
                KeyCode::Char(ch) if ch.is_ascii_digit() => {
                    Action::Planet(PlanetAction::AppendDeleteBuildQtyChar(ch))
                }
                KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                    Action::Planet(PlanetAction::CancelDeleteBuildSlot)
                }
                _ => Action::Noop,
            };
        }
        match key.code {
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                Action::Planet(PlanetAction::MoveBuildList(-1))
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                Action::Planet(PlanetAction::MoveBuildList(1))
            }
            KeyCode::PageUp => Action::Planet(PlanetAction::MoveBuildList(-8)),
            KeyCode::PageDown => Action::Planet(PlanetAction::MoveBuildList(8)),
            KeyCode::Char('d') | KeyCode::Char('D') => {
                Action::Planet(PlanetAction::DeleteBuildSlotRequest)
            }
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                Action::Planet(PlanetAction::OpenBuildMenu)
            }
            _ => Action::Noop,
        }
    }

    pub fn handle_abort_key(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                Action::Planet(PlanetAction::ConfirmBuildAbort)
            }
            KeyCode::Char('q')
            | KeyCode::Char('Q')
            | KeyCode::Char('n')
            | KeyCode::Char('N')
            | KeyCode::Enter
            | KeyCode::Esc => Action::Planet(PlanetAction::CloseBuildAbortPrompt),
            _ => Action::Noop,
        }
    }

    pub fn handle_specify_key(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                Action::Planet(PlanetAction::OpenBuildMenu)
            }
            KeyCode::Enter => Action::Planet(PlanetAction::SubmitBuildUnit),
            KeyCode::Backspace => Action::Planet(PlanetAction::BackspaceBuildUnitInput),
            KeyCode::Char(ch) if ch.is_ascii_digit() => {
                Action::Planet(PlanetAction::AppendBuildUnitChar(ch))
            }
            _ => Action::Noop,
        }
    }

    pub fn handle_quantity_key(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                Action::Planet(PlanetAction::OpenBuildSpecify)
            }
            KeyCode::Enter => Action::Planet(PlanetAction::SubmitBuildQuantity),
            KeyCode::Backspace => Action::Planet(PlanetAction::BackspaceBuildQuantityInput),
            KeyCode::Char(ch) if ch.is_ascii_digit() => {
                Action::Planet(PlanetAction::AppendBuildQuantityChar(ch))
            }
            _ => Action::Noop,
        }
    }
}

fn infer_quantity(order: PlanetBuildOrder, cost: u32) -> Option<u32> {
    if cost == 0 {
        return None;
    }
    let points = order.points_remaining;
    if points % cost == 0 {
        Some(points / cost)
    } else {
        None
    }
}

fn build_abort_order_lines(orders: &[PlanetBuildOrder]) -> Vec<String> {
    #[derive(Clone, Copy)]
    struct Aggregate {
        kind: ProductionItemKind,
        points: u32,
        quantity: Option<u32>,
    }

    let mut grouped = Vec::<Aggregate>::new();

    for order in orders {
        let quantity =
            build_unit_spec_by_kind(order.kind).and_then(|unit| infer_quantity(*order, unit.cost));
        let index = if let Some(index) = grouped
            .iter()
            .position(|aggregate| aggregate.kind == order.kind)
        {
            index
        } else {
            let index = grouped.len();
            grouped.push(Aggregate {
                kind: order.kind,
                points: 0,
                quantity: Some(0),
            });
            index
        };

        let aggregate = &mut grouped[index];
        aggregate.points = aggregate.points.saturating_add(order.points_remaining);
        aggregate.quantity = match (aggregate.quantity, quantity) {
            (Some(total), Some(qty)) => Some(total.saturating_add(qty)),
            _ => None,
        };
    }

    grouped
        .into_iter()
        .map(|aggregate| match aggregate.quantity {
            Some(quantity) => format!(
                "{} {} ({} pts)",
                quantity,
                build_kind_count_label(aggregate.kind, quantity),
                aggregate.points
            ),
            None => format!(
                "{} ({} pts)",
                build_kind_name(aggregate.kind),
                aggregate.points
            ),
        })
        .collect()
}

impl Screen for PlanetBuildScreen {
    fn render(
        &mut self,
        _frame: &ScreenFrame<'_>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        draw_title_bar_padded(&mut buffer, 0, "BUILD COMMAND:");
        draw_dismiss_prompt_padded(&mut buffer, dismiss_prompt_row(0));
        Ok(buffer)
    }

    fn handle_key(&self, key: KeyEvent) -> Action {
        self.handle_menu_key(key)
    }
}

// Draw the shared header and two-column unit table used by both render_specify
// and render_quantity_prompt.
fn draw_specify_table(
    buffer: &mut PlayfieldBuffer,
    points_left: u32,
    entries: &[nc_engine::PlanetBuildSpecifyEntry],
) -> (crate::screen::table::TableRenderMetrics, usize) {
    let style = classic::status_value_style();
    let title = "SPECIFY BUILD ORDERS:";

    let left_units = [0usize, 1, 2, 3, 4];
    let right_units = [5usize, 6, 7, 8];

    let mut rows = Vec::with_capacity(5);
    for i in 0..left_units.len() {
        let left = entries[left_units[i]];
        let right = right_units.get(i).map(|idx| entries[*idx]);
        rows.push(SplitTableRow {
            left_cells: vec![
                if left.selectable {
                    format!("<{:02}>", left.number)
                } else {
                    String::new()
                },
                left.label.to_string(),
                format_build_cost(left.cost),
                format_queued_qty(left.queued_qty),
            ],
            right_cells: right
                .map(|right| {
                    vec![
                        if right.selectable {
                            format!("<{:02}>", right.number)
                        } else {
                            String::new()
                        },
                        right.label.to_string(),
                        format_build_cost(right.cost),
                        format_queued_qty(right.queued_qty),
                    ]
                })
                .unwrap_or_else(|| {
                    vec![String::new(), String::new(), String::new(), String::new()]
                }),
        });
    }

    let table_columns = BUILD_HALF_COLUMNS
        .iter()
        .chain(BUILD_HALF_COLUMNS.iter())
        .copied()
        .collect::<Vec<_>>();
    let table_width = table_render_width(&table_columns);
    let table_col = centered_table_start_col(buffer.width(), &table_columns);
    draw_table_title(buffer, 1, table_col, title);
    let points_left_label = format!("BUDGET: {}", points_left);
    let points_left_col = table_col + table_width - TABLE_TEXT_INSET - points_left_label.len();
    debug_assert!(
        points_left_col >= table_col + TABLE_TEXT_INSET + title.len() + 1,
        "specify build title row is too narrow for the points-left label"
    );
    buffer.write_text(
        0,
        points_left_col,
        &points_left_label,
        classic::title_style(),
    );

    let metrics = write_split_table_at(
        buffer,
        1,
        table_col,
        &BUILD_HALF_COLUMNS,
        &BUILD_HALF_COLUMNS,
        &rows,
        style,
    );

    debug_assert_eq!(
        table_col,
        buffer
            .width()
            .saturating_sub(table_render_width(&table_columns))
            / 2
    );

    (metrics, table_col)
}

fn format_build_cost(cost: u32) -> String {
    format!("{cost:02}")
}

/// A queued count for the QTY. column: in brackets while they fit, as bare
/// digits when the count is too big for them, and ">99k" past five digits.
fn format_queued_qty(quantity: u32) -> String {
    let width = BUILD_HALF_COLUMNS[3].width;
    let bracketed = format!("({quantity})");
    if bracketed.len() <= width {
        bracketed
    } else if quantity.to_string().len() <= width {
        quantity.to_string()
    } else {
        ">99k".to_string()
    }
}

pub fn build_order_summary(order: PlanetBuildOrder) -> String {
    let kind = build_kind_name(order.kind);
    format!("{kind} ({:>2} pts)", order.points_remaining)
}
