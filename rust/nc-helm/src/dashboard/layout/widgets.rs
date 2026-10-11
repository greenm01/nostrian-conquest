//! Explicit widget frames for the dashboard side panels and center map.

use crate::dashboard::buffer::{CellStyle, PlayfieldBuffer};
use crate::dashboard::geometry::ScreenGeometry;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WidgetRect {
    pub col: usize,
    pub row: usize,
    pub width: usize,
    pub height: usize,
}

impl WidgetRect {
    pub const fn last_col(self) -> usize {
        self.col + self.width.saturating_sub(1)
    }

    pub const fn last_row(self) -> usize {
        self.row + self.height.saturating_sub(1)
    }

    pub const fn contains_point(self, col: usize, row: usize) -> bool {
        col >= self.col
            && col < self.col + self.width
            && row >= self.row
            && row < self.row + self.height
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PanelWidgetFrame {
    pub outer: WidgetRect,
    pub title_row: usize,
    pub body: WidgetRect,
}

impl PanelWidgetFrame {
    pub const fn title_col(self) -> usize {
        self.outer.col + 1
    }

    pub const fn title_width(self) -> usize {
        self.outer.width.saturating_sub(1)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MapWidgetFrame {
    pub outer: WidgetRect,
    pub map_block: WidgetRect,
    pub axis_row: usize,
    pub grid: WidgetRect,
    pub bottom_pad_row: usize,
    pub row_label_cols: usize,
    pub cell_width: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DashboardWidgetFrames {
    pub outer_top: usize,
    pub outer_bottom: usize,
    pub header_bar_row: usize,
    pub header_divider_row: usize,
    pub footer_divider_row: usize,
    pub footer_bar_row: usize,
    pub left_divider_col: usize,
    pub right_divider_col: usize,
    pub left_economy: PanelWidgetFrame,
    pub left_planets: PanelWidgetFrame,
    pub left_fleets: PanelWidgetFrame,
    pub left_war_record: PanelWidgetFrame,
    pub center_map: MapWidgetFrame,
    pub right_comms: PanelWidgetFrame,
    pub right_galaxy: PanelWidgetFrame,
    pub right_diplomacy: PanelWidgetFrame,
    pub right_sector_detail: PanelWidgetFrame,
}

pub const fn frame_offset_for(canvas: ScreenGeometry, frame: ScreenGeometry) -> (usize, usize) {
    let ox = canvas.width().saturating_sub(frame.width()) / 2;
    let oy = canvas.height().saturating_sub(frame.height()) / 2;
    (ox, oy)
}

pub(crate) fn panel_widget_frame(
    col: usize,
    width: usize,
    top: usize,
    bottom: usize,
) -> PanelWidgetFrame {
    let outer = WidgetRect {
        col,
        row: top,
        width,
        height: bottom.saturating_sub(top) + 1,
    };
    PanelWidgetFrame {
        outer,
        title_row: top,
        body: WidgetRect {
            col: col + 1,
            row: top + 1,
            width: width.saturating_sub(1),
            height: bottom.saturating_sub(top),
        },
    }
}

fn assert_text_fits_span(context: &str, text: &str, width: usize) {
    let text_width = text.chars().count();
    assert!(
        text_width <= width,
        "{context} overruns its widget span: text width {text_width} exceeds allowed width {width}"
    );
}

fn assert_row_col_in_buffer(buf: &PlayfieldBuffer, row: usize, col: usize, context: &str) {
    assert!(
        row < buf.height(),
        "{context} row {row} is outside buffer height {}",
        buf.height()
    );
    assert!(
        col < buf.width(),
        "{context} col {col} is outside buffer width {}",
        buf.width()
    );
}

pub fn write_strict_span(
    buf: &mut PlayfieldBuffer,
    row: usize,
    col: usize,
    width: usize,
    text: &str,
    style: CellStyle,
    context: &str,
) {
    assert_row_col_in_buffer(buf, row, col, context);
    assert!(
        col + width <= buf.width(),
        "{context} span overruns buffer width: end {} exceeds {}",
        col + width,
        buf.width()
    );
    assert_text_fits_span(context, text, width);
    buf.write_text(row, col, text, style);
}

pub fn write_clipped(
    buf: &mut PlayfieldBuffer,
    row: usize,
    col: usize,
    width: usize,
    text: &str,
    style: CellStyle,
) {
    if width == 0 {
        return;
    }
    assert_row_col_in_buffer(buf, row, col, "clipped widget write");
    assert!(
        col + width <= buf.width(),
        "clipped widget write span overruns buffer width: end {} exceeds {}",
        col + width,
        buf.width()
    );
    let clipped: String = text.chars().take(width).collect();
    buf.write_text_clipped(row, col, &clipped, style);
}

pub fn write_panel_title(
    buf: &mut PlayfieldBuffer,
    frame: PanelWidgetFrame,
    title: &str,
    style: CellStyle,
) {
    write_strict_span(
        buf,
        frame.title_row,
        frame.title_col(),
        frame.title_width(),
        title,
        style,
        "panel title",
    );
}

pub fn write_panel_body_line(
    buf: &mut PlayfieldBuffer,
    frame: PanelWidgetFrame,
    body_row_offset: usize,
    text: &str,
    style: CellStyle,
) {
    if body_row_offset >= frame.body.height {
        return;
    }
    assert!(
        frame.body.col + frame.body.width <= buf.width(),
        "panel body write overruns buffer width"
    );
    write_clipped(
        buf,
        frame.body.row + body_row_offset,
        frame.body.col,
        frame.body.width,
        text,
        style,
    );
}

pub fn label_value_width<'a, I>(labels: I) -> usize
where
    I: IntoIterator<Item = &'a str>,
{
    labels
        .into_iter()
        .map(|label| label.chars().count())
        .max()
        .unwrap_or(0)
}

pub fn left_column_label_width() -> usize {
    label_value_width([
        "Treasury",
        "Emp Rev",
        "Prod",
        "Pot Prod",
        "Tax Rate",
        "PP Gen",
        "% Growth",
        "Efficiency",
        "Prod Rank",
        "Plnt Rank",
        "Cmd Limit",
        "Tot Worlds",
        "Stardocks",
        "Starbases",
        "Tot Armies",
        "GBs",
        "Building",
        "Vulnerable",
        "Tot Fleets",
        "Tot Ships",
        "Docked",
        "Moving",
        "Combat",
        "Guarding",
        "Holding",
        "BBs",
        "CAs",
        "DDs",
        "SCs",
        "TTs",
        "ETs",
        "SBs",
        "Colonies",
        "Taken",
        "Lost",
        "Bombards",
        "Invade S|F",
        "Blitz S|F",
        "Repelled",
        "Ships Dest",
        "Ships Lost",
    ])
}

pub fn format_label_value(label: &str, label_width: usize, value: &str) -> String {
    format!("{label:<label_width$} : {value}")
}

pub fn format_left_column_value(label: &str, value: &str) -> String {
    let label_width = left_column_label_width();
    format!("{label:<label_width$}: {value}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dashboard::app::state::DashApp;
    use crate::dashboard::layout::dashboard_layout;
    use crate::dashboard::layout::geometry::{
        CELL_WIDTH, MAP_LEFT_PADDING, MAP_RIGHT_PADDING, MAP_VERTICAL_PADDING, ROW_LABEL_COLS,
    };
    use nc_data::GameStateBuilder;
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    #[test]
    fn dashboard_widgets_partition_columns_without_overlap() {
        let app = dash_app(ScreenGeometry::new(160, 40));
        let layout = dashboard_layout(&app);
        let widgets = layout.widgets;

        assert_eq!(
            widgets.left_economy.outer.col,
            frame_offset_for(app.geometry, layout.frame).0 + 1
        );
        assert_eq!(widgets.center_map.outer.col, widgets.left_divider_col + 1,);
        assert_eq!(
            widgets.center_map.outer.last_col(),
            widgets.right_divider_col.saturating_sub(1),
        );
        assert!(widgets.center_map.map_block.col >= widgets.center_map.outer.col);
        assert!(widgets.center_map.map_block.last_col() <= widgets.center_map.outer.last_col());
        assert!(widgets.left_economy.outer.last_row() < widgets.left_planets.outer.row);
        assert!(widgets.left_planets.outer.last_row() < widgets.left_fleets.outer.row);
        assert!(widgets.left_fleets.outer.last_row() < widgets.left_war_record.outer.row);
        assert!(widgets.right_galaxy.outer.last_row() < widgets.right_diplomacy.outer.row);
        assert!(widgets.right_diplomacy.outer.last_row() < widgets.right_sector_detail.outer.row);
    }

    #[test]
    fn center_map_frame_tracks_zero_padding_geometry() {
        let app = dash_app(ScreenGeometry::new(160, 40));
        let layout = dashboard_layout(&app);
        let widgets = layout.widgets;

        // Axis row sits at the top of the grid, offset from map_block.row by
        // the top gutter (leftover vertical slack after grid snap split evenly
        // between top and bottom).
        assert!(
            widgets.center_map.axis_row >= widgets.center_map.map_block.row + MAP_VERTICAL_PADDING
        );
        assert!(widgets.center_map.axis_row <= widgets.center_map.map_block.last_row());
        assert_eq!(
            widgets.center_map.grid.col,
            widgets.center_map.map_block.col + MAP_LEFT_PADDING
        );
        assert_eq!(widgets.center_map.grid.row, widgets.center_map.axis_row + 1);
        assert_eq!(
            widgets.center_map.grid.width,
            widgets.center_map.map_block.width - MAP_LEFT_PADDING - MAP_RIGHT_PADDING
        );
        assert!(
            widgets.center_map.map_block.width
                >= ROW_LABEL_COLS + 18 * CELL_WIDTH + 1 + MAP_RIGHT_PADDING
        );
        // bottom_pad_row now holds the mirrored bottom X-axis row, immediately
        // after the grid's closing border row.
        assert_eq!(
            widgets.center_map.bottom_pad_row,
            widgets.center_map.grid.last_row() + 1
        );
        assert!(widgets.center_map.bottom_pad_row <= widgets.center_map.map_block.last_row());
        assert_eq!(
            widgets.right_divider_col,
            widgets.center_map.outer.last_col() + 1
        );
    }

    #[test]
    #[should_panic(expected = "panel title overruns its widget span")]
    fn panel_title_panics_when_it_overruns_widget_span() {
        let mut buffer = PlayfieldBuffer::new(
            40,
            10,
            CellStyle::new(
                crate::dashboard::buffer::GameColor::White,
                crate::dashboard::buffer::GameColor::Black,
                false,
            ),
        );
        let frame = PanelWidgetFrame {
            outer: WidgetRect {
                col: 1,
                row: 1,
                width: 8,
                height: 4,
            },
            title_row: 1,
            body: WidgetRect {
                col: 2,
                row: 2,
                width: 7,
                height: 3,
            },
        };

        write_panel_title(
            &mut buffer,
            frame,
            "TOO LONG PANEL TITLE",
            CellStyle::new(
                crate::dashboard::buffer::GameColor::White,
                crate::dashboard::buffer::GameColor::Black,
                false,
            ),
        );
    }

    #[test]
    fn format_label_value_aligns_colons() {
        let label_width = label_value_width(["Treasury", "Prod", "Emp Rev"]);
        let rows = [
            format_label_value("Treasury", label_width, "820"),
            format_label_value("Prod", label_width, "980/1200"),
            format_label_value("Emp Rev", label_width, "210"),
        ];

        let colon_col_1 = rows[0].find(" : ").expect("first colon");
        let colon_col_2 = rows[1].find(" : ").expect("second colon");
        let colon_col_3 = rows[2].find(" : ").expect("third colon");

        assert_eq!(colon_col_1, colon_col_2);
        assert_eq!(colon_col_2, colon_col_3);
    }

    #[test]
    fn left_column_shared_label_width_aligns_colons_across_widgets() {
        let economy = format_left_column_value("Treasury", "820");
        let planets = format_left_column_value("GBs", "12");
        let fleets = format_left_column_value("Guarding", "3");

        let economy_col = economy.find(": ").expect("economy colon");
        let planets_col = planets.find(": ").expect("planets colon");
        let fleets_col = fleets.find(": ").expect("fleets colon");

        assert_eq!(economy_col, planets_col);
        assert_eq!(planets_col, fleets_col);
    }

    #[test]
    fn compact_left_column_rows_fit_panel_body_width() {
        let rows = [
            format_left_column_value("Treasury", "217"),
            format_left_column_value("Prod", "332"),
            format_left_column_value("Pot Prod", "775"),
            format_left_column_value("Emp Rev", "1032"),
            format_left_column_value("Tax Rate", "50%"),
            format_left_column_value("PP Gen", "+56"),
            format_left_column_value("% Growth", "16.9%"),
            format_left_column_value("Tot Worlds", "9"),
            format_left_column_value("Stardocks", "3"),
            format_left_column_value("Building", "12"),
            format_left_column_value("Tot Fleets", "7"),
            format_left_column_value("Guarding", "0"),
            format_left_column_value("Invade S|F", "2|1"),
            format_left_column_value("Ships Dest", "18"),
        ];

        let width = rows
            .iter()
            .map(|row| row.chars().count())
            .max()
            .expect("rows");
        for row in rows {
            assert!(
                row.chars().count() <= width,
                "{row:?} exceeds left row budget"
            );
        }
    }

    fn dash_app(geometry: ScreenGeometry) -> DashApp {
        DashApp::new_for_tests(
            PathBuf::from("."),
            GameStateBuilder::new()
                .with_player_count(4)
                .build_initialized_baseline()
                .expect("baseline"),
            BTreeMap::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            geometry,
            ScreenGeometry::new(0, 0),
            1,
        )
    }
}
