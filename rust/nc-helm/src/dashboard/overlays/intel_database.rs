//! I overlay: dashboard-sized total planet database.

use std::cmp::Ordering;
use std::collections::BTreeMap;

use crate::dashboard::buffer::PlayfieldBuffer;
use crate::dashboard::coords::{format_sector_coords_default, format_sector_coords_table};
use crate::dashboard::table::{
    TableColumn, TableFooter, TableWidthMode, centered_table_start_col, resolve_table_columns,
    table_render_width, with_command_line_toast, write_stacked_table_window_with_theme_at,
};
use crate::dashboard::table_filter::{FilterKind, TableFilterClause, TableFilterColumn};
use crate::dashboard::table_selection;
use nc_data::{PlanetIntelSnapshot, build_player_starmap_projection_from_snapshots};

use crate::dashboard::app::state::{
    ActiveOverlay, DashApp, IntelOverlayFilter, IntelOverlayPromptMode, IntelOverlaySort,
    SortDirection,
};
use crate::dashboard::layout::MapWidgetFrame;
use crate::dashboard::layout::dashboard;
use crate::dashboard::modal::Rect;
use crate::dashboard::overlays::frame::{
    OverlaySizePolicy, assert_overlay_body_write_fits, dashboard_overlay_parent_rect,
    draw_overlay_frame_for_body_in_parent_with_policy_and_origin, max_overlay_body_width,
    overlay_popup_rect_for_body_in_parent, stacked_table_body_height, write_clipped,
};
use crate::dashboard::theme;

pub(crate) const HOTKEYS: &str = "? F S <ESC>";
const TOP_HEADERS: [&str; 11] = ["Coord", "", "", "", "", "", "", "", "Curr", "", ""];
const COLUMNS: [TableColumn<'static>; 11] = [
    TableColumn::left("(XX,YY)", 7),
    TableColumn::left("Planet Name", 11),
    TableColumn::left("Owner", 7),
    TableColumn::right("Prod", 4),
    TableColumn::right("Seen", 4),
    TableColumn::right("ARs", 3),
    TableColumn::right("GBs", 3),
    TableColumn::right("SBs", 3),
    TableColumn::right("Prod", 4),
    TableColumn::right("Points", 6),
    TableColumn::right("Scout", 5),
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
        code: "own",
        label: "Owner",
        aliases: &["empire"],
        kind: FilterKind::Text,
    },
    TableFilterColumn {
        code: "max",
        label: "Max",
        aliases: &["maximum", "potential"],
        kind: FilterKind::Number,
    },
    TableFilterColumn {
        code: "see",
        label: "Seen",
        aliases: &["year", "yearseen", "seenyear"],
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
    TableFilterColumn {
        code: "sbs",
        label: "Starbase",
        aliases: &["starbases"],
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
        code: "sco",
        label: "Scout",
        aliases: &["scoutyear"],
        kind: FilterKind::Number,
    },
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct IntelOverlayRow {
    pub planet_record_index_1_based: usize,
    pub coords: [u8; 2],
    pub known_name: Option<String>,
    pub known_owner_empire_id: Option<u8>,
    pub known_owner_name: Option<String>,
    pub known_max_production: Option<u16>,
    pub known_year_seen: Option<u16>,
    pub known_armies: Option<u8>,
    pub known_batteries: Option<u8>,
    pub known_starbases: Option<u8>,
    pub known_current_production: Option<u8>,
    pub known_treasury: Option<u16>,
    pub known_scout_year: Option<u16>,
    pub cells: Vec<String>,
}

pub fn draw(buf: &mut PlayfieldBuffer, app: &DashApp, map_frame: MapWidgetFrame) {
    let rows = table_rows(app);
    let selected = app.intel_overlay.selected.min(rows.len().saturating_sub(1));
    let selected_default = rows
        .get(selected)
        .map(|row| format_sector_coords_default(row.coords));
    let title = overlay_title(app);
    let filter_prompt;
    let footer = match app.intel_overlay.prompt_mode {
        IntelOverlayPromptMode::None => TableFooter::CommandBar {
            hotkeys_markup: HOTKEYS,
            default: selected_default.as_deref(),
            input: &app.intel_overlay.jump_input,
        },
        IntelOverlayPromptMode::SortMenu => TableFooter::CommandInput {
            label: "COMMAND",
            prompt: {
                filter_prompt = app
                    .intel_overlay
                    .prompt_status
                    .as_deref()
                    .filter(|value| value.trim_start().starts_with("Ambiguous:"))
                    .map(|value| value.trim_start().to_string())
                    .unwrap_or_else(|| "Sort column [?] ".to_string());
                filter_prompt.as_str()
            },
            default: &app.intel_overlay.prompt_default,
            input: &app.intel_overlay.prompt_input,
        },
        IntelOverlayPromptMode::SortRangeInput => TableFooter::CommandInput {
            label: "COMMAND",
            prompt: "Sort range from ",
            default: &app.intel_overlay.prompt_default,
            input: &app.intel_overlay.prompt_input,
        },
        IntelOverlayPromptMode::FilterMenu => TableFooter::CommandInput {
            label: "COMMAND",
            prompt: {
                filter_prompt = "Filter column [?] ".to_string();
                filter_prompt.as_str()
            },
            default: &app.intel_overlay.prompt_default,
            input: &app.intel_overlay.prompt_input,
        },
        IntelOverlayPromptMode::FilterValueInput => {
            filter_prompt = format!(
                "Filter {} ",
                app.intel_overlay
                    .pending_filter_column
                    .map(|column| column.code)
                    .unwrap_or("value")
            );
            TableFooter::CommandInput {
                label: "COMMAND",
                prompt: filter_prompt.as_str(),
                default: &app.intel_overlay.prompt_default,
                input: &app.intel_overlay.prompt_input,
            }
        }
    };
    let footer = with_command_line_toast(footer, app.active_command_line_toast());
    let table_cells = rows.iter().map(|row| row.cells.clone()).collect::<Vec<_>>();
    let natural_visible_rows = table_cells.len().max(1);
    let columns = resolve_table_columns(
        &COLUMNS,
        &table_cells,
        max_overlay_body_width(map_frame),
        false,
        TableWidthMode::Compact,
    );
    let body_width =
        table_render_width(&columns).max("No planet intel is available yet.".chars().count() + 4);
    let frame = draw_overlay_frame_for_body_in_parent_with_policy_and_origin(
        buf,
        dashboard_overlay_parent_rect(dashboard::dashboard_layout(app).widgets),
        &title,
        body_width,
        stacked_table_body_height(natural_visible_rows),
        OverlaySizePolicy::default(),
        footer,
        app.overlay_position_for(ActiveOverlay::IntelDatabase),
    );
    let visible_rows = frame.body_height.saturating_sub(5);
    assert_overlay_body_write_fits(
        frame,
        &title,
        table_render_width(&columns),
        stacked_table_body_height(visible_rows),
    );
    let scroll = clamp_scroll(app.intel_overlay.scroll, selected, visible_rows, rows.len());
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
            if app.intel_overlay.filter_clause.is_some() {
                "No worlds match current filter."
            } else {
                "No planet intel is available yet."
            },
            theme::dim_style(),
        );
    }
}

pub(crate) fn popup_rect(app: &DashApp, map_frame: MapWidgetFrame) -> Rect {
    let rows = table_rows(app);
    let selected = app.intel_overlay.selected.min(rows.len().saturating_sub(1));
    let selected_default = rows
        .get(selected)
        .map(|row| format_sector_coords_default(row.coords));
    let title = overlay_title(app);
    let filter_prompt;
    let footer = match app.intel_overlay.prompt_mode {
        IntelOverlayPromptMode::None => TableFooter::CommandBar {
            hotkeys_markup: HOTKEYS,
            default: selected_default.as_deref(),
            input: &app.intel_overlay.jump_input,
        },
        IntelOverlayPromptMode::SortMenu => TableFooter::CommandInput {
            label: "COMMAND",
            prompt: {
                filter_prompt = app
                    .intel_overlay
                    .prompt_status
                    .as_deref()
                    .filter(|value| value.trim_start().starts_with("Ambiguous:"))
                    .map(|value| value.trim_start().to_string())
                    .unwrap_or_else(|| "Sort column [?] ".to_string());
                filter_prompt.as_str()
            },
            default: &app.intel_overlay.prompt_default,
            input: &app.intel_overlay.prompt_input,
        },
        IntelOverlayPromptMode::SortRangeInput => TableFooter::CommandInput {
            label: "COMMAND",
            prompt: "Sort range from ",
            default: &app.intel_overlay.prompt_default,
            input: &app.intel_overlay.prompt_input,
        },
        IntelOverlayPromptMode::FilterMenu => TableFooter::CommandInput {
            label: "COMMAND",
            prompt: {
                filter_prompt = "Filter column [?] ".to_string();
                filter_prompt.as_str()
            },
            default: &app.intel_overlay.prompt_default,
            input: &app.intel_overlay.prompt_input,
        },
        IntelOverlayPromptMode::FilterValueInput => {
            filter_prompt = format!(
                "Filter {} ",
                app.intel_overlay
                    .pending_filter_column
                    .map(|column| column.code)
                    .unwrap_or("value")
            );
            TableFooter::CommandInput {
                label: "COMMAND",
                prompt: filter_prompt.as_str(),
                default: &app.intel_overlay.prompt_default,
                input: &app.intel_overlay.prompt_input,
            }
        }
    };
    let footer = with_command_line_toast(footer, app.active_command_line_toast());
    let table_cells = rows.iter().map(|row| row.cells.clone()).collect::<Vec<_>>();
    let natural_visible_rows = table_cells.len().max(1);
    let columns = resolve_table_columns(
        &COLUMNS,
        &table_cells,
        max_overlay_body_width(map_frame),
        false,
        TableWidthMode::Compact,
    );
    let body_width =
        table_render_width(&columns).max("No planet intel is available yet.".chars().count() + 4);
    overlay_popup_rect_for_body_in_parent(
        dashboard_overlay_parent_rect(dashboard::dashboard_layout(app).widgets),
        &title,
        body_width,
        stacked_table_body_height(natural_visible_rows),
        OverlaySizePolicy::default(),
        footer,
        app.overlay_position_for(ActiveOverlay::IntelDatabase),
    )
}

pub(crate) fn selection_rows(app: &DashApp) -> Vec<Vec<String>> {
    table_rows(app)
        .into_iter()
        .map(|row| vec![format_sector_coords_table(row.coords)])
        .collect()
}

pub(crate) fn table_rows(app: &DashApp) -> Vec<IntelOverlayRow> {
    let snapshot_map = app
        .planet_intel_snapshots
        .iter()
        .cloned()
        .map(|snapshot| (snapshot.planet_record_index_1_based, snapshot))
        .collect::<BTreeMap<_, _>>();
    let projection = build_player_starmap_projection_from_snapshots(
        &app.game_data,
        &snapshot_map,
        app.player_record_index_1_based as u8,
    );
    let mut rows = projection
        .worlds
        .iter()
        .map(|world| {
            let snapshot = snapshot_map.get(&world.planet_record_index_1_based);
            format_intel_row(app, world, snapshot)
        })
        .collect::<Vec<_>>();

    rows.retain(|row| match app.intel_overlay.filter {
        IntelOverlayFilter::All => true,
        IntelOverlayFilter::Empire(empire_id) => row.known_owner_empire_id == Some(empire_id),
    });
    if let Some(clause) = &app.intel_overlay.filter_clause {
        rows.retain(|row| intel_row_matches_clause(row, clause));
    }

    rows.sort_by(|left, right| match app.intel_overlay.sort {
        IntelOverlaySort::Location => apply_sort_direction(
            app.intel_overlay.sort_direction,
            left.coords.cmp(&right.coords),
        ),
        IntelOverlaySort::Range(anchor) => apply_sort_direction(
            app.intel_overlay.sort_direction,
            distance_sq(anchor, left.coords).cmp(&distance_sq(anchor, right.coords)),
        )
        .then_with(|| left.coords.cmp(&right.coords)),
        IntelOverlaySort::PlanetName => apply_sort_direction(
            app.intel_overlay.sort_direction,
            left.known_name.as_deref().cmp(&right.known_name.as_deref()),
        )
        .then_with(|| left.coords.cmp(&right.coords)),
        IntelOverlaySort::Owner => apply_sort_direction(
            app.intel_overlay.sort_direction,
            left.known_owner_name
                .as_deref()
                .cmp(&right.known_owner_name.as_deref()),
        )
        .then_with(|| left.coords.cmp(&right.coords)),
        IntelOverlaySort::MaxProduction => apply_sort_direction(
            app.intel_overlay.sort_direction,
            left.known_max_production.cmp(&right.known_max_production),
        )
        .then_with(|| left.coords.cmp(&right.coords)),
        IntelOverlaySort::YearSeen => apply_sort_direction(
            app.intel_overlay.sort_direction,
            left.known_year_seen.cmp(&right.known_year_seen),
        )
        .then_with(|| left.coords.cmp(&right.coords)),
        IntelOverlaySort::Armies => apply_sort_direction(
            app.intel_overlay.sort_direction,
            left.known_armies.cmp(&right.known_armies),
        )
        .then_with(|| left.coords.cmp(&right.coords)),
        IntelOverlaySort::Batteries => apply_sort_direction(
            app.intel_overlay.sort_direction,
            left.known_batteries.cmp(&right.known_batteries),
        )
        .then_with(|| left.coords.cmp(&right.coords)),
        IntelOverlaySort::Starbases => apply_sort_direction(
            app.intel_overlay.sort_direction,
            left.known_starbases.cmp(&right.known_starbases),
        )
        .then_with(|| left.coords.cmp(&right.coords)),
        IntelOverlaySort::CurrentProduction => apply_sort_direction(
            app.intel_overlay.sort_direction,
            left.known_current_production
                .cmp(&right.known_current_production),
        )
        .then_with(|| left.coords.cmp(&right.coords)),
        IntelOverlaySort::Treasury => apply_sort_direction(
            app.intel_overlay.sort_direction,
            left.known_treasury.cmp(&right.known_treasury),
        )
        .then_with(|| left.coords.cmp(&right.coords)),
        IntelOverlaySort::ScoutYear => apply_sort_direction(
            app.intel_overlay.sort_direction,
            left.known_scout_year.cmp(&right.known_scout_year),
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

fn truncate(value: &str, width: usize) -> String {
    value.chars().take(width).collect()
}

fn format_intel_row(
    app: &DashApp,
    world: &nc_data::PlayerStarmapWorld,
    snapshot: Option<&PlanetIntelSnapshot>,
) -> IntelOverlayRow {
    let coords = world.coords;
    let owner_label = match world.known_owner_empire_id {
        Some(0) => String::from("Unowned"),
        Some(owner) => app
            .game_data
            .players
            .get(owner.saturating_sub(1) as usize)
            .map(|player| {
                if player.mode == nc_data::PlayerMode::CivilDisorder {
                    String::from("ICD")
                } else {
                    format!("#{owner}")
                }
            })
            .unwrap_or_else(|| format!("#{owner}")),
        None => String::from("?"),
    };
    IntelOverlayRow {
        planet_record_index_1_based: world.planet_record_index_1_based,
        coords,
        known_name: world.known_name.clone(),
        known_owner_empire_id: world.known_owner_empire_id,
        known_owner_name: world.known_owner_empire_name.clone(),
        known_max_production: world.known_potential_production,
        known_year_seen: snapshot.and_then(|row| row.last_intel_year),
        known_armies: world.known_armies,
        known_batteries: world.known_ground_batteries,
        known_starbases: world.known_starbase_count,
        known_current_production: world.known_current_production,
        known_treasury: world.known_stored_points,
        known_scout_year: snapshot.and_then(|row| row.scout_year),
        cells: vec![
            format_sector_coords_table(coords),
            truncate(world.known_name.as_deref().unwrap_or("?"), 11),
            owner_label,
            world
                .known_potential_production
                .map(|value| value.to_string())
                .unwrap_or_else(|| String::from("?")),
            snapshot
                .and_then(|row| row.last_intel_year)
                .map(|value| value.to_string())
                .unwrap_or_else(|| String::from("?")),
            world
                .known_armies
                .map(|value| value.to_string())
                .unwrap_or_else(|| String::from("?")),
            world
                .known_ground_batteries
                .map(|value| value.to_string())
                .unwrap_or_else(|| String::from("?")),
            world
                .known_starbase_count
                .map(|value| value.to_string())
                .unwrap_or_else(|| String::from("?")),
            world
                .known_current_production
                .map(|value| value.to_string())
                .unwrap_or_else(|| String::from("?")),
            world
                .known_stored_points
                .map(|value| value.to_string())
                .unwrap_or_else(|| String::from("?")),
            snapshot
                .and_then(|row| row.scout_year)
                .map(|value| value.to_string())
                .unwrap_or_else(|| String::from("?")),
        ],
    }
}

pub(crate) fn parse_coords_input(input: &str, default: [u8; 2]) -> Option<[u8; 2]> {
    if input.trim().is_empty() {
        return Some(default);
    }
    let digits = input
        .split(|ch: char| !ch.is_ascii_digit())
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    let [x, y] = digits.as_slice() else {
        return None;
    };
    Some([x.parse().ok()?, y.parse().ok()?])
}

pub(crate) fn sync_cursor_to_jump_input(app: &mut DashApp) -> bool {
    let rows = selection_rows(app);
    let Some(matched) = table_selection::find_typed_jump(&rows, 0, &app.intel_overlay.jump_input)
    else {
        return false;
    };
    app.intel_overlay.selected = matched.index;
    matched.is_terminal_exact_match
}

fn distance_sq(a: [u8; 2], b: [u8; 2]) -> u32 {
    let dx = i32::from(a[0]) - i32::from(b[0]);
    let dy = i32::from(a[1]) - i32::from(b[1]);
    (dx * dx + dy * dy) as u32
}

fn overlay_title(app: &DashApp) -> String {
    format!(
        "TOTAL PLANET DATABASE: {} {} {}",
        sort_label(app.intel_overlay.sort),
        app.intel_overlay.sort_direction.title_label(),
        app.intel_overlay
            .filter_clause
            .as_ref()
            .map(|clause| clause.summary.as_str())
            .unwrap_or(filter_label(app.intel_overlay.filter))
    )
}

const fn sort_label(sort: IntelOverlaySort) -> &'static str {
    match sort {
        IntelOverlaySort::Location => "COO",
        IntelOverlaySort::Range(_) => "RNG",
        IntelOverlaySort::PlanetName => "PLA",
        IntelOverlaySort::Owner => "OWN",
        IntelOverlaySort::MaxProduction => "MAX",
        IntelOverlaySort::YearSeen => "SEE",
        IntelOverlaySort::Armies => "ARS",
        IntelOverlaySort::Batteries => "GBS",
        IntelOverlaySort::Starbases => "SBS",
        IntelOverlaySort::CurrentProduction => "CUR",
        IntelOverlaySort::Treasury => "TRS",
        IntelOverlaySort::ScoutYear => "SCO",
    }
}

pub(crate) fn filter_columns() -> &'static [TableFilterColumn] {
    FILTER_COLUMNS
}

pub(crate) fn filter_default_value(app: &DashApp, column: TableFilterColumn) -> String {
    let row = table_rows(app).get(app.intel_overlay.selected).cloned();
    let Some(row) = row else {
        return String::new();
    };
    match column.code {
        "coo" => format!("{},{}", row.coords[0], row.coords[1]),
        "pla" => row.cells[1].clone(),
        "own" => {
            if row.known_owner_empire_id.is_some() {
                row.cells[2].clone()
            } else {
                "?".to_string()
            }
        }
        "max" => row.cells[3].clone(),
        "see" => row.cells[4].clone(),
        "ars" => row.cells[5].clone(),
        "gbs" => row.cells[6].clone(),
        "sbs" => row.cells[7].clone(),
        "cur" => row.cells[8].clone(),
        "trs" => row.cells[9].clone(),
        "sco" => row.cells[10].clone(),
        _ => String::new(),
    }
}

fn parse_unknown_i64(label: &str) -> Option<i64> {
    if label.trim() == "?" {
        None
    } else {
        label.trim().parse::<i64>().ok()
    }
}

pub(crate) fn intel_row_matches_clause(row: &IntelOverlayRow, clause: &TableFilterClause) -> bool {
    match clause.column.code {
        "coo" => clause.predicate.matches_coord(row.coords),
        "pla" => clause.predicate.matches_text(Some(&row.cells[1])),
        "own" => clause
            .predicate
            .matches_text(if row.known_owner_empire_id.is_some() {
                Some(&row.cells[2])
            } else {
                None
            }),
        "max" => clause
            .predicate
            .matches_number(row.known_max_production.map(i64::from)),
        "see" => clause
            .predicate
            .matches_number(parse_unknown_i64(&row.cells[4])),
        "ars" => clause
            .predicate
            .matches_number(parse_unknown_i64(&row.cells[5])),
        "gbs" => clause
            .predicate
            .matches_number(parse_unknown_i64(&row.cells[6])),
        "sbs" => clause
            .predicate
            .matches_number(parse_unknown_i64(&row.cells[7])),
        "cur" => clause
            .predicate
            .matches_number(parse_unknown_i64(&row.cells[8])),
        "trs" => clause
            .predicate
            .matches_number(parse_unknown_i64(&row.cells[9])),
        "sco" => clause
            .predicate
            .matches_number(parse_unknown_i64(&row.cells[10])),
        _ => true,
    }
}

#[cfg(test)]
fn sort_footer_label(app: &DashApp) -> String {
    format!("SORT {}", app.intel_overlay.sort_direction.label())
}

fn filter_label(filter: crate::dashboard::app::state::IntelOverlayFilter) -> &'static str {
    match filter {
        crate::dashboard::app::state::IntelOverlayFilter::All => "ALL",
        crate::dashboard::app::state::IntelOverlayFilter::Empire(_) => "EMP",
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
    use super::{HOTKEYS, overlay_title, parse_coords_input, sort_footer_label};
    use crate::dashboard::app::state::{DashApp, SortDirection};
    use crate::dashboard::geometry::ScreenGeometry;
    use nc_data::GameStateBuilder;
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::PathBuf;

    #[test]
    fn browse_hotkeys_match_supported_intel_commands() {
        assert_eq!(HOTKEYS, "? F S <ESC>");
    }

    #[test]
    fn parse_coords_input_accepts_rendered_coord_cell() {
        assert_eq!(parse_coords_input("(02,03)", [1, 1]), Some([2, 3]));
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
        app.intel_overlay.sort_direction = SortDirection::Desc;

        assert_eq!(
            overlay_title(&app),
            "TOTAL PLANET DATABASE: COO DESCENDING ALL"
        );
        assert_eq!(sort_footer_label(&app), "SORT DESC");
    }
}
