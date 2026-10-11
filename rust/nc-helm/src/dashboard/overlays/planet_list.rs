//! P overlay: dashboard-sized planet management table.

use std::cmp::Ordering;

use crate::chrome_tags;
use crate::dashboard::buffer::PlayfieldBuffer;
use crate::dashboard::coords::{format_sector_coords_default, format_sector_coords_table};
use crate::dashboard::table::{
    SplitTableRow, TableColumn, TableFooter, TableWidthMode, centered_table_start_col,
    resolve_table_columns, table_footer_scaffold_width, table_render_width,
    with_command_line_toast, write_split_table_at, write_stacked_table_window_with_theme_at,
};
use crate::dashboard::table_filter::{FilterKind, TableFilterClause, TableFilterColumn};
use crate::dashboard::table_selection;
use nc_data::{EmpirePlanetEconomyRow, Planet, ProductionItemKind, STARDOCK_SLOT_COUNT};
use nc_engine::{BUILD_UNITS, planet_build_view};

use crate::dashboard::app::state::{
    ActiveOverlay, DashApp, PlanetOverlayFilter, PlanetOverlayPromptMode, PlanetOverlaySort,
    SortDirection,
};
use crate::dashboard::layout::MapWidgetFrame;
use crate::dashboard::layout::dashboard;
use crate::dashboard::modal::{MODAL_CLOSE_BUTTON, Rect};
use crate::dashboard::overlays::frame::{
    OverlayAxisSize, OverlaySizePolicy, assert_overlay_body_write_fits,
    dashboard_overlay_parent_rect, draw_overlay_frame_for_body_in_parent_with_policy_and_origin,
    max_overlay_body_width, overlay_popup_rect_for_body_in_parent, stacked_table_body_height,
    standard_table_body_height, write_clipped,
};
use crate::dashboard::theme;

pub(crate) const HOTKEYS: &str = "? F S B C A L U X <ESC>";
const TOP_HEADERS: [&str; 13] = [
    "Coord", "", "Max", "Curr", "Trsry", "", "", "", "Build", "Star", "", "", "",
];
const COLUMNS: [TableColumn<'static>; 13] = [
    TableColumn::left("(XX,YY)", 7),
    TableColumn::left("Planet Name", 13),
    TableColumn::right("Prod", 4),
    TableColumn::right("Prod", 4),
    TableColumn::right("Points", 6),
    TableColumn::right("Bdgt", 5),
    TableColumn::right("Rev", 3),
    TableColumn::right("Grow", 4),
    TableColumn::right("Queue", 5),
    TableColumn::right("Dock", 4),
    TableColumn::right("SBs", 3),
    TableColumn::right("ARs", 3),
    TableColumn::right("GBs", 3),
];

const FILTER_COLUMNS: &[TableFilterColumn] = &[
    TableFilterColumn {
        code: "coo",
        label: "Coord",
        aliases: &["coordinates", "location"],
        kind: FilterKind::Coord,
    },
    TableFilterColumn {
        code: "pla",
        label: "Planet",
        aliases: &["name"],
        kind: FilterKind::Text,
    },
    TableFilterColumn {
        code: "max",
        label: "Max",
        aliases: &["maximum", "potential"],
        kind: FilterKind::Number,
    },
    TableFilterColumn {
        code: "cur",
        label: "Current",
        aliases: &["currentprod", "production", "current production"],
        kind: FilterKind::Number,
    },
    TableFilterColumn {
        code: "trs",
        label: "Treasury",
        aliases: &["points", "treasury points"],
        kind: FilterKind::Number,
    },
    TableFilterColumn {
        code: "bdg",
        label: "Budget",
        aliases: &["bdgt", "bgdt"],
        kind: FilterKind::Number,
    },
    TableFilterColumn {
        code: "rev",
        label: "Revenue",
        aliases: &[],
        kind: FilterKind::Number,
    },
    TableFilterColumn {
        code: "gro",
        label: "Growth",
        aliases: &[],
        kind: FilterKind::Number,
    },
    TableFilterColumn {
        code: "bui",
        label: "Build",
        aliases: &["queue"],
        kind: FilterKind::Number,
    },
    TableFilterColumn {
        code: "sta",
        label: "Dock",
        aliases: &["stardock"],
        kind: FilterKind::Number,
    },
    TableFilterColumn {
        code: "sbs",
        label: "Starbase",
        aliases: &["starbases"],
        kind: FilterKind::Number,
    },
    TableFilterColumn {
        code: "ars",
        label: "Armies",
        aliases: &[],
        kind: FilterKind::Number,
    },
    TableFilterColumn {
        code: "gbs",
        label: "Batteries",
        aliases: &["groundbatteries"],
        kind: FilterKind::Number,
    },
];

fn overlay_parent_rect(app: &DashApp) -> Rect {
    dashboard_overlay_parent_rect(dashboard::dashboard_layout(app).widgets)
}

fn command_line_footer<'a>(app: &'a DashApp, footer: TableFooter<'a>) -> TableFooter<'a> {
    with_command_line_toast(footer, app.active_command_line_toast())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PlanetOverlayRow {
    pub planet_record_index_1_based: usize,
    pub coords: [u8; 2],
    pub planet_name: String,
    pub current_prod: u16,
    pub max_prod: u16,
    pub treasury: u32,
    pub budget: u32,
    pub revenue: i16,
    pub growth: i16,
    pub build_queue: u32,
    pub has_starbase: bool,
    pub docked: u32,
    pub armies: u8,
    pub batteries: u8,
    pub cells: Vec<String>,
}

pub fn draw(buf: &mut PlayfieldBuffer, app: &DashApp, map_frame: MapWidgetFrame) {
    match app.planet_overlay.prompt_mode {
        PlanetOverlayPromptMode::BuildSpecify => {
            draw_build_specify(buf, app, map_frame);
            return;
        }
        PlanetOverlayPromptMode::BuildQuantity => {
            draw_build_quantity(buf, app, map_frame);
            return;
        }
        _ => {}
    }
    let rows = table_rows(app);
    let selected = app
        .planet_overlay
        .selected
        .min(rows.len().saturating_sub(1));
    let selected_default = rows
        .get(selected)
        .map(|row| format_sector_coords_default(row.coords));
    let title = overlay_title(app);
    let command_bar = TableFooter::CommandBar {
        hotkeys_markup: HOTKEYS,
        default: selected_default.as_deref(),
        input: &app.planet_overlay.jump_input,
    };
    let filter_prompt;
    let footer = match app.planet_overlay.prompt_mode {
        PlanetOverlayPromptMode::None => command_line_footer(app, command_bar),
        PlanetOverlayPromptMode::SortMenu => TableFooter::CommandInput {
            label: "COMMAND",
            prompt: {
                filter_prompt = app
                    .planet_overlay
                    .prompt_status
                    .as_deref()
                    .filter(|value| value.trim_start().starts_with("Ambiguous:"))
                    .map(|value| value.trim_start().to_string())
                    .unwrap_or_else(|| "Sort column [?] ".to_string());
                filter_prompt.as_str()
            },
            default: &app.planet_overlay.prompt_default,
            input: &app.planet_overlay.prompt_input,
        },
        PlanetOverlayPromptMode::FilterMenu => TableFooter::CommandInput {
            label: "COMMAND",
            prompt: {
                filter_prompt = "Filter column [?] ".to_string();
                filter_prompt.as_str()
            },
            default: &app.planet_overlay.prompt_default,
            input: &app.planet_overlay.prompt_input,
        },
        PlanetOverlayPromptMode::FilterValueInput => {
            filter_prompt = format!(
                "Filter {} ",
                app.planet_overlay
                    .pending_filter_column
                    .map(|column| column.code)
                    .unwrap_or("value")
            );
            TableFooter::CommandInput {
                label: "COMMAND",
                prompt: filter_prompt.as_str(),
                default: &app.planet_overlay.prompt_default,
                input: &app.planet_overlay.prompt_input,
            }
        }
        PlanetOverlayPromptMode::CommissionSelect => TableFooter::CommandInput {
            label: "COMMAND",
            prompt: "Commission slot #",
            default: &app.planet_overlay.prompt_default,
            input: &app.planet_overlay.prompt_input,
        },
        PlanetOverlayPromptMode::MassCommissionConfirm => TableFooter::CommandPrompt {
            label: "COMMAND",
            prompt: "Mass commission? Y/[N] ->",
        },
        PlanetOverlayPromptMode::TransportFleetSelect { mode } => {
            filter_prompt = match mode {
                nc_engine::ArmyTransportMode::Load => "Load Fleet #".to_string(),
                nc_engine::ArmyTransportMode::Unload => "Unload Fleet #".to_string(),
            };
            TableFooter::CommandInput {
                label: "COMMAND",
                prompt: filter_prompt.as_str(),
                default: &app.planet_overlay.prompt_default,
                input: &app.planet_overlay.prompt_input,
            }
        }
        PlanetOverlayPromptMode::TransportQuantity { mode } => {
            filter_prompt = match mode {
                nc_engine::ArmyTransportMode::Load => "How many armies to load?".to_string(),
                nc_engine::ArmyTransportMode::Unload => "How many armies to unload?".to_string(),
            };
            TableFooter::CommandInput {
                label: "COMMAND",
                prompt: filter_prompt.as_str(),
                default: &app.planet_overlay.prompt_default,
                input: &app.planet_overlay.prompt_input,
            }
        }
        PlanetOverlayPromptMode::BuildSpecify | PlanetOverlayPromptMode::BuildQuantity => {
            unreachable!("build flows render separately")
        }
    };
    let footer = command_line_footer(app, footer);
    let table_cells = rows.iter().map(|row| row.cells.clone()).collect::<Vec<_>>();
    let natural_visible_rows = table_cells.len().max(1);
    let columns = resolve_table_columns(
        &COLUMNS,
        &table_cells,
        max_overlay_body_width(map_frame),
        false,
        TableWidthMode::Compact,
    );
    let body_width = table_render_width(&columns)
        .max("You do not currently control any planets.".chars().count() + 4);
    let frame = draw_overlay_frame_for_body_in_parent_with_policy_and_origin(
        buf,
        overlay_parent_rect(app),
        &title,
        body_width,
        stacked_table_body_height(natural_visible_rows),
        OverlaySizePolicy::default(),
        footer,
        app.overlay_position_for(ActiveOverlay::PlanetList),
    );
    let visible_rows = frame.body_height.saturating_sub(5);
    assert_overlay_body_write_fits(
        frame,
        &title,
        table_render_width(&columns),
        stacked_table_body_height(visible_rows),
    );
    let scroll = clamp_scroll(
        app.planet_overlay.scroll,
        selected,
        visible_rows,
        rows.len(),
    );
    let table_col = frame.body_col + centered_table_start_col(frame.body_width, &columns);
    let metrics = write_stacked_table_window_with_theme_at(
        buf,
        frame.body_row,
        table_col,
        &TOP_HEADERS,
        &columns,
        &table_cells,
        scroll,
        visible_rows,
        theme::table_theme(),
        table_cells.get(selected).map(|_| selected),
        0,
        None,
    );

    if rows.is_empty() {
        write_clipped(
            buf,
            metrics.bottom_row.saturating_sub(1),
            frame.body_col,
            frame.body_width,
            if app.planet_overlay.filter_clause.is_some() {
                "No planets match current filter."
            } else {
                "You do not currently control any planets."
            },
            theme::dim_style(),
        );
    }
}

pub(crate) fn popup_rect(app: &DashApp, map_frame: MapWidgetFrame) -> Option<Rect> {
    match app.planet_overlay.prompt_mode {
        PlanetOverlayPromptMode::BuildSpecify => {
            return Some(build_specify_popup_rect(app));
        }
        PlanetOverlayPromptMode::BuildQuantity => {
            return Some(build_quantity_popup_rect(app));
        }
        _ => {}
    }
    let rows = table_rows(app);
    let selected = app
        .planet_overlay
        .selected
        .min(rows.len().saturating_sub(1));
    let selected_default = rows
        .get(selected)
        .map(|row| format_sector_coords_default(row.coords));
    let title = overlay_title(app);
    let command_bar = TableFooter::CommandBar {
        hotkeys_markup: HOTKEYS,
        default: selected_default.as_deref(),
        input: &app.planet_overlay.jump_input,
    };
    let filter_prompt;
    let footer = match app.planet_overlay.prompt_mode {
        PlanetOverlayPromptMode::None => command_line_footer(app, command_bar),
        PlanetOverlayPromptMode::SortMenu => TableFooter::CommandInput {
            label: "COMMAND",
            prompt: {
                filter_prompt = app
                    .planet_overlay
                    .prompt_status
                    .as_deref()
                    .filter(|value| value.trim_start().starts_with("Ambiguous:"))
                    .map(|value| value.trim_start().to_string())
                    .unwrap_or_else(|| "Sort column [?] ".to_string());
                filter_prompt.as_str()
            },
            default: &app.planet_overlay.prompt_default,
            input: &app.planet_overlay.prompt_input,
        },
        PlanetOverlayPromptMode::FilterMenu => TableFooter::CommandInput {
            label: "COMMAND",
            prompt: {
                filter_prompt = "Filter column [?] ".to_string();
                filter_prompt.as_str()
            },
            default: &app.planet_overlay.prompt_default,
            input: &app.planet_overlay.prompt_input,
        },
        PlanetOverlayPromptMode::FilterValueInput => {
            filter_prompt = format!(
                "Filter {} ",
                app.planet_overlay
                    .pending_filter_column
                    .map(|column| column.code)
                    .unwrap_or("value")
            );
            TableFooter::CommandInput {
                label: "COMMAND",
                prompt: filter_prompt.as_str(),
                default: &app.planet_overlay.prompt_default,
                input: &app.planet_overlay.prompt_input,
            }
        }
        PlanetOverlayPromptMode::CommissionSelect => TableFooter::CommandInput {
            label: "COMMAND",
            prompt: "Commission slot #",
            default: &app.planet_overlay.prompt_default,
            input: &app.planet_overlay.prompt_input,
        },
        PlanetOverlayPromptMode::MassCommissionConfirm => TableFooter::CommandPrompt {
            label: "COMMAND",
            prompt: "Mass commission? Y/[N] ->",
        },
        PlanetOverlayPromptMode::TransportFleetSelect { mode } => {
            filter_prompt = match mode {
                nc_engine::ArmyTransportMode::Load => "Load Fleet #".to_string(),
                nc_engine::ArmyTransportMode::Unload => "Unload Fleet #".to_string(),
            };
            TableFooter::CommandInput {
                label: "COMMAND",
                prompt: filter_prompt.as_str(),
                default: &app.planet_overlay.prompt_default,
                input: &app.planet_overlay.prompt_input,
            }
        }
        PlanetOverlayPromptMode::TransportQuantity { mode } => {
            filter_prompt = match mode {
                nc_engine::ArmyTransportMode::Load => "How many armies to load?".to_string(),
                nc_engine::ArmyTransportMode::Unload => "How many armies to unload?".to_string(),
            };
            TableFooter::CommandInput {
                label: "COMMAND",
                prompt: filter_prompt.as_str(),
                default: &app.planet_overlay.prompt_default,
                input: &app.planet_overlay.prompt_input,
            }
        }
        PlanetOverlayPromptMode::BuildSpecify | PlanetOverlayPromptMode::BuildQuantity => {
            unreachable!("build flows are not draggable")
        }
    };
    let footer = command_line_footer(app, footer);
    let table_cells = rows.iter().map(|row| row.cells.clone()).collect::<Vec<_>>();
    let natural_visible_rows = table_cells.len().max(1);
    let columns = resolve_table_columns(
        &COLUMNS,
        &table_cells,
        max_overlay_body_width(map_frame),
        false,
        TableWidthMode::Compact,
    );
    let body_width = table_render_width(&columns)
        .max("You do not currently control any planets.".chars().count() + 4);
    Some(overlay_popup_rect_for_body_in_parent(
        overlay_parent_rect(app),
        &title,
        body_width,
        stacked_table_body_height(natural_visible_rows),
        OverlaySizePolicy::default(),
        footer,
        app.overlay_position_for(ActiveOverlay::PlanetList),
    ))
}

fn draw_build_specify(buf: &mut PlayfieldBuffer, app: &DashApp, _map_frame: MapWidgetFrame) {
    let entries = app.planet_build_specify_entries();
    let split_rows = build_specify_split_rows(&entries, app.planet_overlay.build_selected_kind);
    let title = app.planet_build_title();
    let frame = draw_overlay_frame_for_body_in_parent_with_policy_and_origin(
        buf,
        overlay_parent_rect(app),
        &title,
        build_overlay_body_width(app),
        standard_table_body_height(split_rows.len()),
        build_overlay_size_policy(app),
        command_line_footer(app, build_browse_footer(app)),
        app.overlay_position_for(ActiveOverlay::PlanetList),
    );
    assert_overlay_body_write_fits(
        frame,
        &title,
        build_specify_table_width(),
        standard_table_body_height(split_rows.len()),
    );
    draw_build_budget_tag(buf, frame, app);
    draw_build_specify_body(buf, app, frame, &split_rows);
}

fn draw_build_quantity(buf: &mut PlayfieldBuffer, app: &DashApp, map_frame: MapWidgetFrame) {
    let Some(kind) = app.planet_overlay.build_selected_kind else {
        draw_build_specify(buf, app, map_frame);
        return;
    };
    if BUILD_UNITS.iter().all(|unit| unit.kind != kind) {
        draw_build_specify(buf, app, map_frame);
        return;
    }
    let max_qty = app.planet_build_max_quantity_for(kind).unwrap_or(0);
    let prompt = build_quantity_footer_prompt(max_qty);
    let entries = app.planet_build_specify_entries();
    let split_rows = build_specify_split_rows(&entries, Some(kind));
    let title = app.planet_build_title();
    let frame = draw_overlay_frame_for_body_in_parent_with_policy_and_origin(
        buf,
        overlay_parent_rect(app),
        &title,
        build_overlay_body_width(app),
        standard_table_body_height(split_rows.len()),
        build_overlay_size_policy(app),
        command_line_footer(
            app,
            TableFooter::CommandPromptInput {
                label: "COMMAND",
                prompt: &prompt,
                input: &app.planet_overlay.build_quantity_input,
            },
        ),
        app.overlay_position_for(ActiveOverlay::PlanetList),
    );
    assert_overlay_body_write_fits(
        frame,
        &title,
        build_specify_table_width(),
        standard_table_body_height(split_rows.len()),
    );
    draw_build_budget_tag(buf, frame, app);
    draw_build_specify_body(buf, app, frame, &split_rows);
}

fn build_specify_popup_rect(app: &DashApp) -> Rect {
    let entries = app.planet_build_specify_entries();
    let split_rows = build_specify_split_rows(&entries, app.planet_overlay.build_selected_kind);
    overlay_popup_rect_for_body_in_parent(
        overlay_parent_rect(app),
        &app.planet_build_title(),
        build_overlay_body_width(app),
        standard_table_body_height(split_rows.len()),
        build_overlay_size_policy(app),
        command_line_footer(app, build_browse_footer(app)),
        app.overlay_position_for(ActiveOverlay::PlanetList),
    )
}

fn build_quantity_popup_rect(app: &DashApp) -> Rect {
    let Some(kind) = app.planet_overlay.build_selected_kind else {
        return build_specify_popup_rect(app);
    };
    if BUILD_UNITS.iter().all(|unit| unit.kind != kind) {
        return build_specify_popup_rect(app);
    }
    let max_qty = app.planet_build_max_quantity_for(kind).unwrap_or(0);
    let prompt = build_quantity_footer_prompt(max_qty);
    let entries = app.planet_build_specify_entries();
    let split_rows = build_specify_split_rows(&entries, Some(kind));
    overlay_popup_rect_for_body_in_parent(
        overlay_parent_rect(app),
        &app.planet_build_title(),
        build_overlay_body_width(app),
        standard_table_body_height(split_rows.len()),
        build_overlay_size_policy(app),
        command_line_footer(
            app,
            TableFooter::CommandPromptInput {
                label: "COMMAND",
                prompt: &prompt,
                input: &app.planet_overlay.build_quantity_input,
            },
        ),
        app.overlay_position_for(ActiveOverlay::PlanetList),
    )
}

const BUILD_HALF_COLUMNS: [TableColumn<'static>; 4] = [
    TableColumn::left("NO.", 4),
    TableColumn::left("UNIT TYPE", 19),
    TableColumn::right("COST", 4),
    TableColumn::right("QTY.", 5),
];

fn build_specify_all_columns() -> [TableColumn<'static>; 8] {
    [
        BUILD_HALF_COLUMNS[0],
        BUILD_HALF_COLUMNS[1],
        BUILD_HALF_COLUMNS[2],
        BUILD_HALF_COLUMNS[3],
        BUILD_HALF_COLUMNS[0],
        BUILD_HALF_COLUMNS[1],
        BUILD_HALF_COLUMNS[2],
        BUILD_HALF_COLUMNS[3],
    ]
}

fn build_specify_table_width() -> usize {
    table_render_width(&build_specify_all_columns())
}

fn build_overlay_size_policy(app: &DashApp) -> OverlaySizePolicy {
    OverlaySizePolicy {
        width: OverlayAxisSize::Fixed(build_overlay_body_width(app)),
        height: OverlayAxisSize::FitContent,
    }
}

fn build_overlay_body_width(app: &DashApp) -> usize {
    let chrome_width = chrome_tags::tag_width(&app.planet_build_title())
        + chrome_tags::tag_width(&build_budget_label(app))
        + chrome_tags::tag_width(MODAL_CLOSE_BUTTON)
        + 4;
    let quantity_footer_width = BUILD_UNITS
        .iter()
        .copied()
        .map(|unit| build_quantity_footer_scaffold_width(app, unit.kind))
        .max()
        .unwrap_or(0);
    build_specify_table_width()
        .max(table_footer_scaffold_width(build_browse_footer(app)))
        .max(quantity_footer_width)
        .max(chrome_width.saturating_sub(4))
}

fn build_browse_footer<'a>(app: &'a DashApp) -> TableFooter<'a> {
    TableFooter::CommandPromptInput {
        label: "COMMAND",
        prompt: "? + - D <ESC> [0] -> ",
        input: &app.planet_overlay.build_unit_input,
    }
}

fn build_quantity_footer_scaffold_width(app: &DashApp, kind: ProductionItemKind) -> usize {
    let max_qty = app.planet_build_max_quantity_for(kind).unwrap_or(0);
    let prompt = build_quantity_footer_prompt(max_qty);
    table_footer_scaffold_width(TableFooter::CommandPromptInput {
        label: "COMMAND",
        prompt: &prompt,
        input: "",
    })
}

fn build_quantity_footer_prompt(max_qty: u32) -> String {
    format!("Qty [{}] -> ", max_qty)
}

fn build_budget_label(app: &DashApp) -> String {
    app.planet_build_view()
        .map(|view| format!("BUDGET: {}", view.points_left))
        .unwrap_or_else(|| "BUDGET: 0".to_string())
}

fn draw_build_budget_tag(
    buf: &mut PlayfieldBuffer,
    frame: crate::dashboard::overlays::frame::OverlayFrame,
    app: &DashApp,
) {
    let popup_left = frame.body_col.saturating_sub(2);
    let popup_width = frame.body_width + 4;
    let label = build_budget_label(app);
    let tag_width = chrome_tags::tag_width(&label);
    let centered_col = popup_left + popup_width.saturating_sub(tag_width) / 2;
    let min_col = popup_left + 2 + chrome_tags::tag_width(&app.planet_build_title()) + 1;
    let max_col = chrome_tags::close_tag_col(popup_left, popup_width)
        .unwrap_or(popup_left + popup_width)
        .saturating_sub(tag_width + 1);
    let col = centered_col.clamp(min_col, max_col.max(min_col));
    chrome_tags::draw_tag(
        frame.body_row.saturating_sub(1),
        col,
        popup_left
            .saturating_add(popup_width)
            .saturating_sub(col)
            .saturating_sub(1),
        &label,
        theme::border_style(),
        theme::title_style(),
        chrome_tags::TOP_TAG_LEFT,
        chrome_tags::TOP_TAG_RIGHT,
        |op| match op {
            chrome_tags::TagDrawOp::SetCell {
                row,
                col,
                ch,
                style,
            } => buf.set_cell(row, col, ch, style),
            chrome_tags::TagDrawOp::WriteText {
                row,
                col,
                text,
                style,
            } => {
                buf.write_text(row, col, text, style);
            }
        },
    );
}

fn draw_build_specify_body(
    buf: &mut PlayfieldBuffer,
    app: &DashApp,
    frame: crate::dashboard::overlays::frame::OverlayFrame,
    split_rows: &[SplitTableRow],
) {
    if app.planet_build_view().is_none() {
        write_clipped(
            buf,
            frame.body_row,
            frame.body_col,
            frame.body_width,
            "No owned planets available for building.",
            theme::dim_style(),
        );
        return;
    }
    let table_col =
        frame.body_col + centered_table_start_col(frame.body_width, &build_specify_all_columns());
    let _ = write_split_table_at(
        buf,
        frame.body_row,
        table_col,
        &BUILD_HALF_COLUMNS,
        &BUILD_HALF_COLUMNS,
        split_rows,
        theme::value_style(),
    );
}

fn build_specify_split_rows(
    entries: &[nc_engine::PlanetBuildSpecifyEntry],
    selected_kind: Option<ProductionItemKind>,
) -> Vec<SplitTableRow> {
    let left_units = [0usize, 1, 2, 3, 4];
    let right_units = [5usize, 6, 7, 8];

    (0..left_units.len())
        .map(|idx| {
            let left = entries[left_units[idx]];
            let right = right_units
                .get(idx)
                .and_then(|entry_idx| entries.get(*entry_idx).copied());
            SplitTableRow {
                left_cells: build_specify_cells(left),
                left_selected: selected_kind == Some(left.kind),
                right_cells: right.map(build_specify_cells).unwrap_or_else(|| {
                    vec![String::new(), String::new(), String::new(), String::new()]
                }),
                right_selected: right
                    .map(|entry| selected_kind == Some(entry.kind))
                    .unwrap_or(false),
            }
        })
        .collect()
}

fn build_specify_cells(entry: nc_engine::PlanetBuildSpecifyEntry) -> Vec<String> {
    vec![
        if entry.selectable {
            format!("<{:02}>", entry.number)
        } else {
            String::new()
        },
        entry.label.to_string(),
        format!("{:02}", entry.cost),
        format!("({})", entry.queued_qty),
    ]
}

pub(crate) fn selection_rows(app: &DashApp) -> Vec<Vec<String>> {
    table_rows(app)
        .into_iter()
        .map(|row| vec![format_sector_coords_table(row.coords)])
        .collect()
}

pub(crate) fn table_rows(app: &DashApp) -> Vec<PlanetOverlayRow> {
    let starbase_coords = app
        .game_data
        .bases
        .iter()
        .filter(|base| base.owner.0 == app.player_record_index_1_based as u8 && base.base_id.0 != 0)
        .map(|base| base.sector.to_array())
        .collect::<std::collections::BTreeSet<_>>();

    let mut rows = app
        .game_data
        .empire_planet_economy_rows(app.player_record_index_1_based)
        .iter()
        .filter_map(|row| {
            app.game_data
                .planets
                .get(row.planet_record_index_1_based.saturating_sub(1))
                .map(|planet| {
                    format_planet_row_cells(
                        &app.game_data,
                        row,
                        planet,
                        starbase_coords.contains(&row.coords),
                    )
                })
        })
        .collect::<Vec<_>>();

    rows.retain(|row| match app.planet_overlay.filter {
        PlanetOverlayFilter::All => true,
        PlanetOverlayFilter::Range { anchor, radius } => {
            distance_sq(anchor, row.coords) <= u32::from(radius) * u32::from(radius)
        }
        PlanetOverlayFilter::Starbase => row.has_starbase,
    });
    if let Some(clause) = &app.planet_overlay.filter_clause {
        rows.retain(|row| planet_row_matches_clause(row, clause));
    }

    rows.sort_by(|left, right| match app.planet_overlay.sort {
        PlanetOverlaySort::Location => apply_sort_direction(
            app.planet_overlay.sort_direction,
            left.coords.cmp(&right.coords),
        ),
        PlanetOverlaySort::PlanetName => apply_sort_direction(
            app.planet_overlay.sort_direction,
            left.planet_name.cmp(&right.planet_name),
        )
        .then_with(|| left.coords.cmp(&right.coords)),
        PlanetOverlaySort::MaxProduction => apply_sort_direction(
            app.planet_overlay.sort_direction,
            left.max_prod.cmp(&right.max_prod),
        )
        .then_with(|| left.coords.cmp(&right.coords)),
        PlanetOverlaySort::CurrentProduction => apply_sort_direction(
            app.planet_overlay.sort_direction,
            left.current_prod.cmp(&right.current_prod),
        )
        .then_with(|| left.coords.cmp(&right.coords)),
        PlanetOverlaySort::Treasury => apply_sort_direction(
            app.planet_overlay.sort_direction,
            left.treasury.cmp(&right.treasury),
        )
        .then_with(|| left.coords.cmp(&right.coords)),
        PlanetOverlaySort::Budget => apply_sort_direction(
            app.planet_overlay.sort_direction,
            left.budget.cmp(&right.budget),
        )
        .then_with(|| left.coords.cmp(&right.coords)),
        PlanetOverlaySort::Revenue => apply_sort_direction(
            app.planet_overlay.sort_direction,
            left.revenue.cmp(&right.revenue),
        )
        .then_with(|| left.coords.cmp(&right.coords)),
        PlanetOverlaySort::Growth => apply_sort_direction(
            app.planet_overlay.sort_direction,
            left.growth.cmp(&right.growth),
        )
        .then_with(|| left.coords.cmp(&right.coords)),
        PlanetOverlaySort::BuildQueue => apply_sort_direction(
            app.planet_overlay.sort_direction,
            left.build_queue.cmp(&right.build_queue),
        )
        .then_with(|| left.coords.cmp(&right.coords)),
        PlanetOverlaySort::Stardock => apply_sort_direction(
            app.planet_overlay.sort_direction,
            left.docked.cmp(&right.docked),
        )
        .then_with(|| left.coords.cmp(&right.coords)),
        PlanetOverlaySort::Starbase => apply_sort_direction(
            app.planet_overlay.sort_direction,
            left.has_starbase.cmp(&right.has_starbase),
        )
        .then_with(|| left.coords.cmp(&right.coords)),
        PlanetOverlaySort::Armies => apply_sort_direction(
            app.planet_overlay.sort_direction,
            left.armies.cmp(&right.armies),
        )
        .then_with(|| left.coords.cmp(&right.coords)),
        PlanetOverlaySort::Batteries => apply_sort_direction(
            app.planet_overlay.sort_direction,
            left.batteries.cmp(&right.batteries),
        )
        .then_with(|| left.coords.cmp(&right.coords)),
    });

    rows
}

fn clamp_scroll(scroll: usize, selected: usize, max_rows: usize, total_rows: usize) -> usize {
    if max_rows == 0 || total_rows <= max_rows {
        return 0;
    }
    if selected < scroll {
        return selected;
    }
    if selected >= scroll + max_rows {
        return selected + 1 - max_rows;
    }
    scroll.min(total_rows.saturating_sub(max_rows))
}

fn format_planet_row_cells(
    game_data: &nc_data::CoreGameData,
    row: &EmpirePlanetEconomyRow,
    planet: &Planet,
    has_starbase: bool,
) -> PlanetOverlayRow {
    let coords = planet.sector.to_array();
    let (stored, budget) = effective_points_left(game_data, row);
    let queue = build_queue_total(planet);
    let docked = docked_total(planet);
    let growth = row.yearly_growth_delta as i16;

    PlanetOverlayRow {
        planet_record_index_1_based: row.planet_record_index_1_based,
        coords,
        planet_name: row.planet_name.clone(),
        current_prod: row.present_production,
        max_prod: row.potential_production,
        treasury: stored,
        budget,
        revenue: row.yearly_tax_revenue as i16,
        growth,
        build_queue: queue,
        has_starbase,
        docked,
        armies: (planet.armies.min(u16::from(u8::MAX)) as u8),
        batteries: (planet.ground_batteries.min(u16::from(u8::MAX)) as u8),
        cells: vec![
            format_sector_coords_table(coords),
            truncate(&row.planet_name, 13),
            row.potential_production.to_string(),
            row.present_production.to_string(),
            stored.to_string(),
            budget.to_string(),
            row.yearly_tax_revenue.to_string(),
            format!("{growth:+}"),
            queue.to_string(),
            docked.to_string(),
            u8::from(has_starbase).to_string(),
            (planet.armies.min(u16::from(u8::MAX)) as u8).to_string(),
            (planet.ground_batteries.min(u16::from(u8::MAX)) as u8).to_string(),
        ],
    }
}

fn effective_points_left(
    game_data: &nc_data::CoreGameData,
    row: &EmpirePlanetEconomyRow,
) -> (u32, u32) {
    planet_build_view(game_data, row)
        .map(|view| (view.treasury_left, view.points_left))
        .unwrap_or_else(|_| {
            (
                row.stored_production_points,
                u32::from(row.build_capacity).min(row.stored_production_points),
            )
        })
}

fn build_queue_total(planet: &Planet) -> u32 {
    (0..10)
        .map(|slot| {
            let points = u32::from(planet.build_queue[slot].points_owed);
            let kind = planet.build_queue[slot].kind;
            let Some(cost) = kind.build_cost() else {
                return 0;
            };
            points / cost
        })
        .sum()
}

fn docked_total(planet: &Planet) -> u32 {
    (0..STARDOCK_SLOT_COUNT)
        .map(|slot| planet.stardock[slot].min(u32::from(u16::MAX)))
        .sum()
}

fn truncate(value: &str, width: usize) -> String {
    value.chars().take(width).collect()
}

pub(crate) fn sync_cursor_to_jump_input(app: &mut DashApp) -> bool {
    let rows = selection_rows(app);
    let Some(matched) = table_selection::find_typed_jump(&rows, 0, &app.planet_overlay.jump_input)
    else {
        return false;
    };
    app.planet_overlay.selected = matched.index;
    matched.is_terminal_exact_match
}

fn distance_sq(a: [u8; 2], b: [u8; 2]) -> u32 {
    let dx = i32::from(a[0]) - i32::from(b[0]);
    let dy = i32::from(a[1]) - i32::from(b[1]);
    (dx * dx + dy * dy) as u32
}

fn overlay_title(app: &DashApp) -> String {
    format!(
        "PLANET LIST: {} {} {}",
        sort_label(app.planet_overlay.sort),
        app.planet_overlay.sort_direction.title_label(),
        app.planet_overlay
            .filter_clause
            .as_ref()
            .map(|clause| clause.summary.as_str())
            .unwrap_or(filter_label(app.planet_overlay.filter))
    )
}

const fn sort_label(sort: PlanetOverlaySort) -> &'static str {
    match sort {
        PlanetOverlaySort::Location => "COO",
        PlanetOverlaySort::PlanetName => "PLA",
        PlanetOverlaySort::MaxProduction => "MAX",
        PlanetOverlaySort::CurrentProduction => "CUR",
        PlanetOverlaySort::Treasury => "TRS",
        PlanetOverlaySort::Budget => "BDG",
        PlanetOverlaySort::Revenue => "REV",
        PlanetOverlaySort::Growth => "GRO",
        PlanetOverlaySort::BuildQueue => "BUI",
        PlanetOverlaySort::Stardock => "STA",
        PlanetOverlaySort::Starbase => "SBS",
        PlanetOverlaySort::Armies => "ARS",
        PlanetOverlaySort::Batteries => "GBS",
    }
}

pub(crate) fn filter_columns() -> &'static [TableFilterColumn] {
    FILTER_COLUMNS
}

pub(crate) fn filter_default_value(app: &DashApp, column: TableFilterColumn) -> String {
    let row = table_rows(app).get(app.planet_overlay.selected).cloned();
    let Some(row) = row else {
        return String::new();
    };
    match column.code {
        "coo" => format!("{},{}", row.coords[0], row.coords[1]),
        "pla" => row.cells[1].clone(),
        "max" => row.cells[2].clone(),
        "cur" => row.cells[3].clone(),
        "trs" => row.cells[4].clone(),
        "bdg" => row.cells[5].clone(),
        "rev" => row.cells[6].clone(),
        "gro" => row.cells[7].clone(),
        "bui" => row.cells[8].clone(),
        "sta" => row.cells[9].clone(),
        "sbs" => row.cells[10].clone(),
        "ars" => row.cells[11].clone(),
        "gbs" => row.cells[12].clone(),
        _ => String::new(),
    }
}

pub(crate) fn planet_row_matches_clause(
    row: &PlanetOverlayRow,
    clause: &TableFilterClause,
) -> bool {
    match clause.column.code {
        "coo" => clause.predicate.matches_coord(row.coords),
        "pla" => clause.predicate.matches_text(Some(&row.cells[1])),
        "max" => clause
            .predicate
            .matches_number(row.cells[2].parse::<i64>().ok()),
        "cur" => clause
            .predicate
            .matches_number(row.cells[3].parse::<i64>().ok()),
        "trs" => clause
            .predicate
            .matches_number(row.cells[4].parse::<i64>().ok()),
        "bdg" => clause
            .predicate
            .matches_number(row.cells[5].parse::<i64>().ok()),
        "rev" => clause
            .predicate
            .matches_number(row.cells[6].parse::<i64>().ok()),
        "gro" => clause
            .predicate
            .matches_number(row.cells[7].parse::<i64>().ok()),
        "bui" => clause
            .predicate
            .matches_number(row.cells[8].parse::<i64>().ok()),
        "sta" => clause
            .predicate
            .matches_number(row.cells[9].parse::<i64>().ok()),
        "sbs" => clause
            .predicate
            .matches_number(row.cells[10].parse::<i64>().ok()),
        "ars" => clause
            .predicate
            .matches_number(row.cells[11].parse::<i64>().ok()),
        "gbs" => clause
            .predicate
            .matches_number(row.cells[12].parse::<i64>().ok()),
        _ => true,
    }
}

#[cfg(test)]
fn sort_footer_label(app: &DashApp) -> String {
    format!("SORT {}", app.planet_overlay.sort_direction.label())
}

fn filter_label(filter: crate::dashboard::app::state::PlanetOverlayFilter) -> &'static str {
    match filter {
        crate::dashboard::app::state::PlanetOverlayFilter::All => "ALL",
        crate::dashboard::app::state::PlanetOverlayFilter::Range { .. } => "RNG",
        crate::dashboard::app::state::PlanetOverlayFilter::Starbase => "SB",
    }
}

fn apply_sort_direction(direction: SortDirection, ordering: Ordering) -> Ordering {
    match direction {
        SortDirection::Asc => ordering,
        SortDirection::Desc => ordering.reverse(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dashboard::buffer::PlayfieldBuffer;
    use crate::dashboard::geometry::ScreenGeometry;
    use crate::dashboard::theme;
    use nc_data::{GameStateBuilder, ProductionItemKind};
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::PathBuf;

    #[test]
    fn browse_hotkeys_match_supported_planet_list_commands() {
        assert_eq!(HOTKEYS, "? F S B C A L U X <ESC>");
    }

    #[test]
    fn titles_and_sort_footer_show_direction() {
        let mut app = DashApp::new_for_tests(
            PathBuf::from("."),
            GameStateBuilder::new()
                .with_player_count(4)
                .build_initialized_baseline()
                .expect("baseline"),
            BTreeMap::new(),
            BTreeSet::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            ScreenGeometry::new(160, 45),
            ScreenGeometry::new(0, 0),
            1,
        );
        app.planet_overlay.sort_direction = SortDirection::Asc;

        assert_eq!(overlay_title(&app), "PLANET LIST: CUR ASCENDING ALL");
        assert_eq!(sort_footer_label(&app), "SORT ASC");
    }

    #[test]
    fn build_specify_rows_keep_all_choices_and_blank_unselectable_tags() {
        let rows = build_specify_split_rows(
            &[
                nc_engine::PlanetBuildSpecifyEntry {
                    number: 1,
                    kind: ProductionItemKind::Destroyer,
                    label: "Destroyers",
                    cost: 5,
                    queued_qty: 0,
                    selectable: true,
                },
                nc_engine::PlanetBuildSpecifyEntry {
                    number: 2,
                    kind: ProductionItemKind::Cruiser,
                    label: "Cruisers",
                    cost: 15,
                    queued_qty: 0,
                    selectable: false,
                },
                nc_engine::PlanetBuildSpecifyEntry {
                    number: 3,
                    kind: ProductionItemKind::Battleship,
                    label: "Battleships",
                    cost: 45,
                    queued_qty: 0,
                    selectable: false,
                },
                nc_engine::PlanetBuildSpecifyEntry {
                    number: 4,
                    kind: ProductionItemKind::Scout,
                    label: "Scouts",
                    cost: 15,
                    queued_qty: 0,
                    selectable: false,
                },
                nc_engine::PlanetBuildSpecifyEntry {
                    number: 5,
                    kind: ProductionItemKind::Transport,
                    label: "Troop transports",
                    cost: 5,
                    queued_qty: 0,
                    selectable: true,
                },
                nc_engine::PlanetBuildSpecifyEntry {
                    number: 6,
                    kind: ProductionItemKind::Etac,
                    label: "ETACs",
                    cost: 20,
                    queued_qty: 0,
                    selectable: false,
                },
                nc_engine::PlanetBuildSpecifyEntry {
                    number: 7,
                    kind: ProductionItemKind::Starbase,
                    label: "Starbases",
                    cost: 50,
                    queued_qty: 0,
                    selectable: false,
                },
                nc_engine::PlanetBuildSpecifyEntry {
                    number: 9,
                    kind: ProductionItemKind::Army,
                    label: "Armies",
                    cost: 2,
                    queued_qty: 3,
                    selectable: true,
                },
                nc_engine::PlanetBuildSpecifyEntry {
                    number: 10,
                    kind: ProductionItemKind::GroundBattery,
                    label: "Ground batteries",
                    cost: 20,
                    queued_qty: 1,
                    selectable: false,
                },
            ],
            None,
        );

        assert_eq!(rows.len(), 5);
        assert_eq!(rows[0].left_cells[0], "<01>");
        assert!(!rows[0].left_selected);
        assert_eq!(rows[0].right_cells[0], "");
        assert!(!rows[0].right_selected);
        assert_eq!(rows[2].right_cells[0], "<09>");
        assert_eq!(rows[2].right_cells[3], "(3)");
        assert_eq!(rows[3].right_cells[0], "");
        assert_eq!(rows[4].left_cells[0], "<05>");
        assert!(rows[4].right_cells.iter().all(|cell| cell.is_empty()));
    }

    #[test]
    fn build_specify_rows_mark_selected_unit_half() {
        let rows = build_specify_split_rows(
            &[
                nc_engine::PlanetBuildSpecifyEntry {
                    number: 1,
                    kind: ProductionItemKind::Destroyer,
                    label: "Destroyers",
                    cost: 5,
                    queued_qty: 0,
                    selectable: true,
                },
                nc_engine::PlanetBuildSpecifyEntry {
                    number: 2,
                    kind: ProductionItemKind::Cruiser,
                    label: "Cruisers",
                    cost: 15,
                    queued_qty: 0,
                    selectable: true,
                },
                nc_engine::PlanetBuildSpecifyEntry {
                    number: 3,
                    kind: ProductionItemKind::Battleship,
                    label: "Battleships",
                    cost: 45,
                    queued_qty: 0,
                    selectable: true,
                },
                nc_engine::PlanetBuildSpecifyEntry {
                    number: 4,
                    kind: ProductionItemKind::Scout,
                    label: "Scouts",
                    cost: 15,
                    queued_qty: 0,
                    selectable: true,
                },
                nc_engine::PlanetBuildSpecifyEntry {
                    number: 5,
                    kind: ProductionItemKind::Transport,
                    label: "Troop transports",
                    cost: 5,
                    queued_qty: 0,
                    selectable: true,
                },
                nc_engine::PlanetBuildSpecifyEntry {
                    number: 6,
                    kind: ProductionItemKind::Etac,
                    label: "ETACs",
                    cost: 20,
                    queued_qty: 0,
                    selectable: true,
                },
                nc_engine::PlanetBuildSpecifyEntry {
                    number: 7,
                    kind: ProductionItemKind::Starbase,
                    label: "Starbases",
                    cost: 50,
                    queued_qty: 0,
                    selectable: true,
                },
                nc_engine::PlanetBuildSpecifyEntry {
                    number: 9,
                    kind: ProductionItemKind::Army,
                    label: "Armies",
                    cost: 2,
                    queued_qty: 0,
                    selectable: true,
                },
                nc_engine::PlanetBuildSpecifyEntry {
                    number: 10,
                    kind: ProductionItemKind::GroundBattery,
                    label: "Ground batteries",
                    cost: 20,
                    queued_qty: 0,
                    selectable: true,
                },
            ],
            Some(ProductionItemKind::Army),
        );

        assert!(rows[2].right_selected);
        assert!(!rows[2].left_selected);
        assert!(!rows[1].left_selected);
    }

    #[test]
    fn planet_table_rows_show_treasury_and_budget_after_pending_build_spend() {
        let mut game_data = GameStateBuilder::new()
            .with_player_count(4)
            .build_initialized_baseline()
            .expect("baseline");
        let planet = &mut game_data.planets[0];
        planet.stored_goods = nc_data::Real48::from_f64(f64::from(165));
        planet.build_queue[0].kind = nc_data::ProductionItemKind::from_code(1);
        planet.build_queue[0].points_owed = 5;
        planet.build_queue[1].kind = nc_data::ProductionItemKind::from_code(6);
        planet.build_queue[1].points_owed = 40;

        let app = DashApp::new_for_tests(
            PathBuf::from("."),
            game_data,
            BTreeMap::new(),
            BTreeSet::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            ScreenGeometry::new(160, 45),
            ScreenGeometry::new(0, 0),
            1,
        );

        let row = table_rows(&app)
            .into_iter()
            .find(|row| row.planet_record_index_1_based == 1)
            .expect("homeworld row");

        assert_eq!(row.cells[4], "120");
        assert_eq!(row.cells[5], "55");
    }

    #[test]
    fn build_popup_width_stays_stable_between_browse_and_quantity() {
        let mut app = DashApp::new_for_tests(
            PathBuf::from("."),
            GameStateBuilder::new()
                .with_player_count(4)
                .build_initialized_baseline()
                .expect("baseline"),
            BTreeMap::new(),
            BTreeSet::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            ScreenGeometry::new(160, 45),
            ScreenGeometry::new(0, 0),
            1,
        );
        app.overlay = ActiveOverlay::PlanetList;
        app.game_data.planets[0].stored_goods = nc_data::Real48::from_f64(f64::from(80));
        app.open_planet_build_specify();

        let browse_rect = build_specify_popup_rect(&app);
        app.planet_overlay.build_unit_input.push('1');
        app.submit_planet_build_browse_input()
            .expect("browse submit should open quantity");
        let quantity_rect = build_quantity_popup_rect(&app);

        assert_eq!(browse_rect.width, quantity_rect.width);
    }

    #[test]
    fn build_browse_footer_renders_exact_command_rail() {
        let mut app = DashApp::new_for_tests(
            PathBuf::from("."),
            GameStateBuilder::new()
                .with_player_count(4)
                .build_initialized_baseline()
                .expect("baseline"),
            BTreeMap::new(),
            BTreeSet::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            ScreenGeometry::new(160, 45),
            ScreenGeometry::new(0, 0),
            1,
        );
        app.overlay = ActiveOverlay::PlanetList;
        app.game_data.planets[0].stored_goods = nc_data::Real48::from_f64(f64::from(80));
        app.open_planet_build_specify();

        let mut buf = PlayfieldBuffer::new(160, 45, theme::body_style());
        draw(
            &mut buf,
            &app,
            crate::dashboard::layout::dashboard::dashboard_layout(&app)
                .widgets
                .center_map,
        );

        assert!((0..45).any(|row| {
            buf.plain_line(row)
                .contains("COMMAND <- ? + - D <ESC> [0] ->")
        }));
    }

    #[test]
    fn build_quantity_footer_renders_exact_command_rail() {
        let mut app = DashApp::new_for_tests(
            PathBuf::from("."),
            GameStateBuilder::new()
                .with_player_count(4)
                .build_initialized_baseline()
                .expect("baseline"),
            BTreeMap::new(),
            BTreeSet::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            ScreenGeometry::new(160, 45),
            ScreenGeometry::new(0, 0),
            1,
        );
        app.overlay = ActiveOverlay::PlanetList;
        app.game_data.planets[0].stored_goods = nc_data::Real48::from_f64(f64::from(80));
        app.open_planet_build_specify();
        app.planet_overlay.build_unit_input.push('1');
        app.submit_planet_build_browse_input()
            .expect("browse submit should open quantity");

        let mut buf = PlayfieldBuffer::new(160, 45, theme::body_style());
        draw(
            &mut buf,
            &app,
            crate::dashboard::layout::dashboard::dashboard_layout(&app)
                .widgets
                .center_map,
        );

        assert!((0..45).any(|row| buf.plain_line(row).contains("COMMAND <- Qty [16] ->")));
    }

    #[test]
    fn build_budget_renders_in_top_border_not_body() {
        let mut app = DashApp::new_for_tests(
            PathBuf::from("."),
            GameStateBuilder::new()
                .with_player_count(4)
                .build_initialized_baseline()
                .expect("baseline"),
            BTreeMap::new(),
            BTreeSet::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            ScreenGeometry::new(160, 45),
            ScreenGeometry::new(0, 0),
            1,
        );
        app.overlay = ActiveOverlay::PlanetList;
        app.game_data.planets[0].stored_goods = nc_data::Real48::from_f64(f64::from(80));
        app.open_planet_build_specify();

        let popup = build_specify_popup_rect(&app);
        let mut buf = PlayfieldBuffer::new(160, 45, theme::body_style());
        draw(
            &mut buf,
            &app,
            crate::dashboard::layout::dashboard::dashboard_layout(&app)
                .widgets
                .center_map,
        );

        assert!(buf.plain_line(popup.y as usize).contains("BUDGET: 80"));
        assert!(
            !buf.plain_line((popup.y + 1) as usize)
                .contains("BUDGET: 80")
        );
    }
}
