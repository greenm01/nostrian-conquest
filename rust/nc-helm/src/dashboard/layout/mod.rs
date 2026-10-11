//! Three-column frame, border drawing, resize handling.

pub mod dashboard;
pub mod geometry;
pub mod widgets;

use crate::dashboard::buffer::PlayfieldBuffer;
use crate::dashboard::geometry::ScreenGeometry;
use crate::dashboard::prompt;

use crate::dashboard::app::state::DashApp;
use crate::dashboard::theme;
pub use dashboard::{
    DashboardLayout, dashboard_fits_canvas, dashboard_layout, layout_canvas_requirement,
    preferred_dashboard_frame, required_dashboard_frame,
};
pub use widgets::{
    DashboardWidgetFrames, MapWidgetFrame, PanelWidgetFrame, format_label_value,
    format_left_column_value, frame_offset_for, label_value_width, write_clipped,
    write_panel_body_line, write_panel_title, write_strict_span,
};

/// Create a new PlayfieldBuffer at the full canvas size, filled with theme bg.
#[allow(dead_code)]
pub fn new_dashboard_buffer(geometry: ScreenGeometry) -> PlayfieldBuffer {
    PlayfieldBuffer::new(geometry.width(), geometry.height(), theme::body_style())
}

/// Draw the complete outer border + column dividers + header/footer dividers.
pub fn draw_frame(
    buf: &mut PlayfieldBuffer,
    frame: ScreenGeometry,
    widgets: &DashboardWidgetFrames,
) {
    let (ox, _) = frame_offset_for(ScreenGeometry::new(buf.width(), buf.height()), frame);
    let fw = frame.width();
    let bs = theme::border_style();

    let left_div = widgets.left_divider_col;
    let right_div = widgets.right_divider_col;
    let top = widgets.outer_top;
    let bottom = widgets.outer_bottom;
    let header_div = widgets.header_divider_row;
    let footer_div = widgets.footer_divider_row;
    let fh = bottom.saturating_sub(top).saturating_add(1);

    crate::chrome_box::draw_box_outline(
        buf.width(),
        buf.height(),
        ox,
        top,
        fw,
        fh,
        bs,
        |row, col, ch, style| buf.set_cell(row, col, ch, style),
    );

    // Header divider.
    for c in (ox + 1)..(ox + fw.saturating_sub(1)) {
        buf.set_cell(header_div, c, '─', bs);
    }
    buf.set_cell(header_div, ox, '├', bs);
    buf.set_cell(header_div, ox + fw.saturating_sub(1), '┤', bs);

    // Footer divider.
    for c in (ox + 1)..(ox + fw.saturating_sub(1)) {
        buf.set_cell(footer_div, c, '─', bs);
    }
    buf.set_cell(footer_div, ox, '├', bs);
    buf.set_cell(footer_div, ox + fw.saturating_sub(1), '┤', bs);

    // Column dividers (between header and footer dividers).
    for r in (header_div + 1)..footer_div {
        buf.set_cell(r, left_div, '│', bs);
        buf.set_cell(r, right_div, '│', bs);
    }
    // Tee joints.
    buf.set_cell(header_div, left_div, '┬', bs);
    buf.set_cell(header_div, right_div, '┬', bs);
    buf.set_cell(footer_div, left_div, '┴', bs);
    buf.set_cell(footer_div, right_div, '┴', bs);

    draw_panel_separator(
        buf,
        ox,
        left_div,
        widgets.left_planets.outer.row.saturating_sub(1),
        '├',
        '┤',
        bs,
    );
    draw_panel_separator(
        buf,
        ox,
        left_div,
        widgets.left_fleets.outer.row.saturating_sub(1),
        '├',
        '┤',
        bs,
    );
    draw_panel_separator(
        buf,
        ox,
        left_div,
        widgets.left_war_record.outer.row.saturating_sub(1),
        '├',
        '┤',
        bs,
    );
    draw_panel_separator(
        buf,
        right_div,
        ox + fw.saturating_sub(1),
        widgets.right_galaxy.outer.row.saturating_sub(1),
        '├',
        '┤',
        bs,
    );
    draw_panel_separator(
        buf,
        right_div,
        ox + fw.saturating_sub(1),
        widgets.right_diplomacy.outer.row.saturating_sub(1),
        '├',
        '┤',
        bs,
    );
    draw_panel_separator(
        buf,
        right_div,
        ox + fw.saturating_sub(1),
        widgets.right_sector_detail.outer.row.saturating_sub(1),
        '├',
        '┤',
        bs,
    );
}

/// Draw header text on the interior header bar row.
pub fn draw_header(buf: &mut PlayfieldBuffer, app: &DashApp, layout: &DashboardLayout) {
    let (ox, _) = frame_offset_for(app.geometry, layout.frame);
    let fw = layout.frame.width();
    let row = layout.widgets.header_bar_row;
    let inner_right = ox + fw.saturating_sub(2);

    // Branding (after left corner).
    write_strict_span(
        buf,
        row,
        ox + 2,
        inner_right.saturating_sub(ox + 2) + 1,
        " NOSTRIAN CONQUEST ",
        theme::title_style(),
        "dashboard header branding",
    );

    // Empire name centered.
    let empire_name = app
        .game_data
        .players
        .get(app.player_record_index_1_based.saturating_sub(1))
        .map(|p| {
            String::from_utf8_lossy(p.empire_name.as_bytes())
                .trim_end_matches('\0')
                .to_string()
        })
        .unwrap_or_default();
    if !empire_name.is_empty() {
        let center = ox + fw / 2;
        let name_start = center.saturating_sub(empire_name.len() / 2);
        write_strict_span(
            buf,
            row,
            name_start,
            inner_right.saturating_sub(name_start) + 1,
            &empire_name,
            theme::title_style(),
            "dashboard header empire name",
        );
    }

    // Right-justified stats.
    let player = app
        .game_data
        .players
        .get(app.player_record_index_1_based.saturating_sub(1));
    let year = app.game_data.conquest.game_year;
    let tax = player.map(|p| p.tax_rate).unwrap_or(0);
    let right_str = format!("Y{year}                  Tax:{tax}% ");
    let right_col = (ox + fw).saturating_sub(right_str.len() + 1);
    write_strict_span(
        buf,
        row,
        right_col,
        inner_right.saturating_sub(right_col) + 1,
        &right_str,
        theme::header_style(),
        "dashboard header stats",
    );
}

/// Draw footer text on the interior footer bar row.
pub fn draw_footer(buf: &mut PlayfieldBuffer, app: &DashApp, layout: &DashboardLayout) {
    let (ox, _) = frame_offset_for(app.geometry, layout.frame);
    let row = layout.widgets.footer_bar_row;
    prompt::draw_table_command_bar_in_span_with_default_style(
        buf,
        row,
        ox + 1,
        layout.frame.width().saturating_sub(2),
        "? P F T I D X <ESC>",
        Some(&current_coord_default(app)),
        theme::map_selection_style(),
        &app.map_coord_input,
    );
}

fn current_coord_default(app: &DashApp) -> String {
    format!("{:02},{:02}", app.crosshair_x, app.crosshair_y)
}

fn draw_panel_separator(
    buf: &mut PlayfieldBuffer,
    left: usize,
    right: usize,
    row: usize,
    left_connector: char,
    right_connector: char,
    style: crate::dashboard::buffer::CellStyle,
) {
    if left > right {
        return;
    }
    for col in left..=right {
        let glyph = if col == left {
            left_connector
        } else if col == right {
            right_connector
        } else {
            '─'
        };
        buf.set_cell(row, col, glyph, style);
    }
}

#[cfg(test)]
fn line_char(buffer: &PlayfieldBuffer, row: usize, col: usize) -> Option<char> {
    buffer.row(row).get(col).map(|cell| cell.ch)
}

#[cfg(test)]
fn plain_separator(
    buf: &mut PlayfieldBuffer,
    left: usize,
    right: usize,
    row: usize,
    style: crate::dashboard::buffer::CellStyle,
) {
    for col in left..=right {
        buf.set_cell(row, col, '─', style);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dashboard::app::state::DashApp;
    use nc_data::GameStateBuilder;
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::PathBuf;

    #[test]
    fn write_clipped_respects_panel_width() {
        let mut buffer = PlayfieldBuffer::new(32, 4, theme::body_style());
        write_clipped(
            &mut buffer,
            1,
            2,
            8,
            "this text is wider than eight",
            theme::value_style(),
        );
        assert_eq!(buffer.plain_line(1), "  this tex");
    }

    #[test]
    fn panel_separator_stays_within_bounds() {
        let mut buffer = PlayfieldBuffer::new(20, 6, theme::body_style());
        plain_separator(&mut buffer, 2, 10, 3, theme::border_style());
        assert_eq!(buffer.plain_line(3), "  ─────────");
    }

    #[test]
    fn dashboard_frame_separators_follow_widget_boundaries() {
        let app = dash_app();
        let layout = dashboard_layout(&app);
        let frame = layout.frame;
        let widgets = layout.widgets;
        let mut buffer = PlayfieldBuffer::new(
            app.geometry.width(),
            app.geometry.height(),
            theme::body_style(),
        );

        draw_frame(&mut buffer, frame, &widgets);

        let left_sep_line: String = buffer
            .row(widgets.left_planets.outer.row.saturating_sub(1))
            .iter()
            .map(|cell| cell.ch)
            .collect();
        let right_sep_line: String = buffer
            .row(widgets.right_diplomacy.outer.row.saturating_sub(1))
            .iter()
            .map(|cell| cell.ch)
            .collect();

        assert_eq!(
            left_sep_line.chars().nth(widgets.left_economy.outer.col),
            Some('─')
        );
        assert_eq!(
            right_sep_line.chars().nth(widgets.right_galaxy.outer.col),
            Some('─')
        );
        assert_eq!(
            line_char(
                &buffer,
                widgets.left_planets.outer.row.saturating_sub(1),
                widgets.outer_top.saturating_sub(widgets.outer_top)
                    + frame_offset_for(app.geometry, frame).0,
            ),
            Some('├')
        );
    }

    #[test]
    fn dashboard_header_and_footer_rows_are_inside_the_outer_border() {
        let layout = dashboard_layout(&dash_app());
        let widgets = layout.widgets;
        assert_eq!(widgets.header_bar_row, widgets.outer_top + 1);
        assert_eq!(widgets.header_divider_row, widgets.outer_top + 2);
        assert_eq!(widgets.footer_bar_row, widgets.outer_bottom - 1);
        assert_eq!(widgets.footer_divider_row, widgets.outer_bottom - 2);
    }

    #[test]
    fn dashboard_frame_uses_shared_square_corners() {
        let app = dash_app();
        let layout = dashboard_layout(&app);
        let frame = layout.frame;
        let widgets = layout.widgets;
        let mut buffer = PlayfieldBuffer::new(
            app.geometry.width(),
            app.geometry.height(),
            theme::body_style(),
        );

        draw_frame(&mut buffer, frame, &widgets);

        let left = frame_offset_for(app.geometry, frame).0;
        let right = left + frame.width().saturating_sub(1);
        assert_eq!(line_char(&buffer, widgets.outer_top, left), Some('┌'));
        assert_eq!(line_char(&buffer, widgets.outer_top, right), Some('┐'));
        assert_eq!(line_char(&buffer, widgets.outer_bottom, left), Some('└'));
        assert_eq!(line_char(&buffer, widgets.outer_bottom, right), Some('┘'));
    }

    #[test]
    fn side_panel_separators_draw_t_connectors_into_shell_and_dividers() {
        let app = dash_app();
        let layout = dashboard_layout(&app);
        let frame = layout.frame;
        let widgets = layout.widgets;
        let mut buffer = PlayfieldBuffer::new(
            app.geometry.width(),
            app.geometry.height(),
            theme::body_style(),
        );

        draw_frame(&mut buffer, frame, &widgets);

        let left_row = widgets.left_planets.outer.row.saturating_sub(1);
        let right_row = widgets.right_diplomacy.outer.row.saturating_sub(1);
        let left_border_col = frame_offset_for(app.geometry, frame).0;
        let right_border_col = left_border_col + frame.width().saturating_sub(1);

        assert_eq!(line_char(&buffer, left_row, left_border_col), Some('├'));
        assert_eq!(
            line_char(&buffer, left_row, widgets.left_divider_col),
            Some('┤')
        );
        assert_eq!(
            line_char(&buffer, right_row, widgets.right_divider_col),
            Some('├')
        );
        assert_eq!(line_char(&buffer, right_row, right_border_col), Some('┤'));
    }

    #[test]
    fn dashboard_footer_uses_command_line_with_crosshair_default() {
        let mut app = dash_app();
        app.crosshair_x = 2;
        app.crosshair_y = 3;
        let mut buffer = PlayfieldBuffer::new(
            app.geometry.width(),
            app.geometry.height(),
            theme::body_style(),
        );
        let layout = dashboard_layout(&app);

        draw_footer(&mut buffer, &app, &layout);

        let widgets = layout.widgets;
        let line = buffer.plain_line(widgets.footer_bar_row);
        assert!(line.contains("COMMAND <- ? P F T I D X <ESC> [02,03] ->"));
        assert!(!line.contains("P:Planets"));

        let coord_col = line.find("[02,03]").expect("coord default col");
        let row = buffer.row(widgets.footer_bar_row);
        assert_eq!(
            row[coord_col].style,
            theme::classic::prompt_square_delimiter_style()
        );
        assert_eq!(row[coord_col + 1].style, theme::map_selection_style());
        assert_eq!(row[coord_col + 5].style, theme::map_selection_style());
        assert_eq!(
            row[coord_col + 6].style,
            theme::classic::prompt_square_delimiter_style()
        );
    }

    fn dash_app() -> DashApp {
        DashApp::new_for_tests(
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
            ScreenGeometry::new(160, 40),
            ScreenGeometry::new(0, 0),
            1,
        )
    }
}
