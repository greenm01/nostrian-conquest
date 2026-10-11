//! D overlay: centered diplomacy and leaderboard table.

use crate::dashboard::buffer::{CellStyle, PlayfieldBuffer};
use crate::dashboard::table::{
    TableColumn, TableFooter, TableWidthMode, centered_table_start_col, resolve_table_columns,
    table_column_start, table_render_width, with_command_line_toast,
    write_table_window_with_theme_at,
};

use crate::dashboard::app::state::{ActiveOverlay, DashApp};
use crate::dashboard::diplomacy_view::{
    display_name, empire_name_style, relation_label_and_style, state_label_and_style,
};
use crate::dashboard::layout::MapWidgetFrame;
use crate::dashboard::layout::dashboard;
use crate::dashboard::modal::Rect;
use crate::dashboard::overlays::frame::{
    OverlaySizePolicy, assert_overlay_body_write_fits, dashboard_overlay_parent_rect,
    draw_overlay_frame_for_body_in_parent_with_policy_and_origin, max_overlay_body_width,
    overlay_popup_rect_for_body_in_parent, standard_table_body_height, write_clipped,
};
use crate::dashboard::theme;
use nc_data::EmpireProductionRankingSort;

const HOTKEYS: &str = "? E N <ESC>";
const EMPTY_MESSAGE: &str = "No empires available.";
const EMPIRE_COLUMN: usize = 1;
const STATE_COLUMN: usize = 4;
const RELATION_COLUMN: usize = 5;
const COLUMNS: [TableColumn<'static>; 6] = [
    TableColumn::right("Rnk", 3),
    TableColumn::left_flex("Empire", 17, 1),
    TableColumn::right("Planets", 7),
    TableColumn::right("Prod", 4),
    TableColumn::left("State", 10),
    TableColumn::left("Relations", 9),
];

pub fn draw(buf: &mut PlayfieldBuffer, app: &DashApp, map_frame: MapWidgetFrame) {
    let rows = table_rows(app);
    let table_cells = rows.iter().map(|row| row.cells.clone()).collect::<Vec<_>>();
    let natural_visible_rows = rows.len().max(1);
    let footer = with_command_line_toast(
        TableFooter::CommandBar {
            hotkeys_markup: HOTKEYS,
            default: None,
            input: "",
        },
        app.active_command_line_toast(),
    );
    let columns = resolve_table_columns(
        &COLUMNS,
        &table_cells,
        max_overlay_body_width(map_frame),
        false,
        TableWidthMode::Compact,
    );
    let body_width = table_render_width(&columns).max(EMPTY_MESSAGE.chars().count() + 4);
    let frame = draw_overlay_frame_for_body_in_parent_with_policy_and_origin(
        buf,
        dashboard_overlay_parent_rect(dashboard::dashboard_layout(app).widgets),
        "DIPLOMACY",
        body_width,
        standard_table_body_height(natural_visible_rows),
        OverlaySizePolicy::default(),
        footer,
        app.overlay_position_for(ActiveOverlay::Diplomacy),
    );
    let visible_rows = frame.body_height.saturating_sub(4);
    assert_overlay_body_write_fits(
        frame,
        "DIPLOMACY",
        table_render_width(&columns),
        standard_table_body_height(visible_rows),
    );
    let selected = app
        .diplomacy_overlay
        .selected
        .min(rows.len().saturating_sub(1));
    let scroll = clamp_scroll(
        app.diplomacy_overlay.scroll,
        selected,
        visible_rows,
        rows.len(),
    );
    let table_col = frame.body_col + centered_table_start_col(frame.body_width, &columns);
    let metrics = write_table_window_with_theme_at(
        buf,
        frame.body_row,
        table_col,
        &columns,
        &table_cells,
        scroll,
        visible_rows,
        theme::table_theme(),
        table_cells.get(selected).map(|_| selected),
        0,
        None,
    );
    apply_diplomacy_cell_styles(
        buf,
        table_col,
        frame.body_row + 3,
        &columns,
        &rows,
        scroll,
        visible_rows,
        selected,
        app.player_record_index_1_based as u8,
    );

    if rows.is_empty() {
        write_clipped(
            buf,
            metrics.bottom_row.saturating_sub(1),
            frame.body_col,
            frame.body_width,
            EMPTY_MESSAGE,
            theme::dim_style(),
        );
    }
}

pub(crate) fn popup_rect(app: &DashApp, map_frame: MapWidgetFrame) -> Rect {
    let rows = table_rows(app);
    let table_cells = rows.iter().map(|row| row.cells.clone()).collect::<Vec<_>>();
    let columns = resolve_table_columns(
        &COLUMNS,
        &table_cells,
        max_overlay_body_width(map_frame),
        false,
        TableWidthMode::Compact,
    );
    let body_width = table_render_width(&columns).max(EMPTY_MESSAGE.chars().count() + 4);
    overlay_popup_rect_for_body_in_parent(
        dashboard_overlay_parent_rect(dashboard::dashboard_layout(app).widgets),
        "DIPLOMACY",
        body_width,
        standard_table_body_height(rows.len().max(1)),
        OverlaySizePolicy::default(),
        TableFooter::CommandBar {
            hotkeys_markup: HOTKEYS,
            default: None,
            input: "",
        },
        app.overlay_position_for(ActiveOverlay::Diplomacy),
    )
}

#[cfg(test)]
mod tests {
    use super::HOTKEYS;

    #[test]
    fn browse_hotkeys_match_supported_diplomacy_commands() {
        assert_eq!(HOTKEYS, "? E N <ESC>");
    }
}

#[derive(Debug, Clone)]
pub(crate) struct DiplomacyRow {
    pub(crate) slot: u8,
    pub(crate) name: String,
    state: String,
    state_style: crate::dashboard::buffer::CellStyle,
    relation: String,
    relation_style: crate::dashboard::buffer::CellStyle,
    pub(crate) cells: Vec<String>,
}

pub(crate) fn table_rows(app: &DashApp) -> Vec<DiplomacyRow> {
    let player_idx = app.player_record_index_1_based.saturating_sub(1);
    let viewer_slot = app.player_record_index_1_based as u8;
    let viewer = app.game_data.players.get(player_idx);
    app.game_data
        .empire_production_ranking_rows(EmpireProductionRankingSort::Production)
        .into_iter()
        .enumerate()
        .filter_map(|(rank_idx, ranking)| {
            let slot = ranking.empire_id;
            let player = app.game_data.players.get(slot.saturating_sub(1) as usize)?;
            let name = display_name(player, slot);
            let (state, state_style) = state_label_and_style(
                &app.game_data,
                player,
                &app.player_activity_states,
                &app.player_lifecycle_states,
                viewer_slot,
                slot,
            );
            let (relation, relation_style) = relation_label_and_style(viewer, viewer_slot, slot);
            let state = state.to_string();
            let relation = relation.to_string();
            Some(DiplomacyRow {
                slot,
                name: name.clone(),
                state: state.clone(),
                state_style,
                relation: relation.clone(),
                relation_style,
                cells: vec![
                    (rank_idx + 1).to_string(),
                    name,
                    ranking.planets_owned.to_string(),
                    ranking.current_production.to_string(),
                    state,
                    relation,
                ],
            })
        })
        .collect()
}

pub(crate) fn selected_empire_slot(app: &DashApp) -> Option<u8> {
    let rows = table_rows(app);
    rows.get(
        app.diplomacy_overlay
            .selected
            .min(rows.len().saturating_sub(1)),
    )
    .map(|row| row.slot)
}

fn apply_diplomacy_cell_styles(
    buf: &mut PlayfieldBuffer,
    table_col: usize,
    first_body_row: usize,
    columns: &[TableColumn<'_>],
    rows: &[DiplomacyRow],
    scroll: usize,
    visible_rows: usize,
    selected: usize,
    viewer_slot: u8,
) {
    let table_theme = theme::table_theme();
    for (visible_idx, row_data) in rows.iter().skip(scroll).take(visible_rows).enumerate() {
        let absolute_idx = scroll + visible_idx;
        let row = first_body_row + visible_idx;
        let base_style = if absolute_idx == selected {
            table_theme.selected_style
        } else if row_data.slot == viewer_slot {
            theme::friendly_style()
        } else {
            table_theme.body_style
        };
        write_styled_table_cell(
            buf,
            row,
            table_col,
            columns,
            EMPIRE_COLUMN,
            &row_data.name,
            empire_name_style(row_data.slot, base_style.bg, base_style.bold),
        );
        write_styled_table_cell(
            buf,
            row,
            table_col,
            columns,
            STATE_COLUMN,
            &row_data.state,
            CellStyle::new(row_data.state_style.fg, base_style.bg, base_style.bold),
        );
        write_styled_table_cell(
            buf,
            row,
            table_col,
            columns,
            RELATION_COLUMN,
            &row_data.relation,
            CellStyle::new(row_data.relation_style.fg, base_style.bg, base_style.bold),
        );
    }
}

fn write_styled_table_cell(
    buf: &mut PlayfieldBuffer,
    row: usize,
    table_col: usize,
    columns: &[TableColumn<'_>],
    column_index: usize,
    text: &str,
    style: CellStyle,
) {
    let Some(column) = columns.get(column_index).copied() else {
        return;
    };
    let Some(start) = table_column_start(columns, column_index) else {
        return;
    };
    write_clipped(
        buf,
        row,
        table_col + start,
        column.width,
        &format_cell(text, column),
        style,
    );
}

fn format_cell(text: &str, column: TableColumn<'_>) -> String {
    let text = text.chars().take(column.width).collect::<String>();
    match column.align {
        crate::dashboard::table::TableAlign::Left => {
            format!("{text:<width$}", width = column.width)
        }
        crate::dashboard::table::TableAlign::Center => {
            let pad = column.width.saturating_sub(text.chars().count());
            let left = pad / 2;
            let right = pad.saturating_sub(left);
            format!("{}{}{}", " ".repeat(left), text, " ".repeat(right))
        }
        crate::dashboard::table::TableAlign::Right => {
            format!("{text:>width$}", width = column.width)
        }
    }
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
